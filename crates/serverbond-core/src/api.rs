//! Local management API.
//!
//! Everything the desktop interface and the CLI can do is reachable over HTTP
//! on `127.0.0.1:<port>` with a bearer token, so scripts, CI runners and
//! remote-control tools can drive the environment without the window. The
//! listener never binds a public address; only the SHA-256 of the token is
//! stored, and the token itself is shown once when it is created.
//!
//! Except for the raw OpenAPI document, JSON responses are `{ "ok": true, "data": … }` or
//! `{ "ok": false, "error": "…" }`. Manager errors are reported as `400`
//! with the same Turkish message the interface would show.

use crate::{
    model::{ProjectRelease, ProjectSchedule, QueueWorker, Settings},
    storage, Manager, ToolAction,
};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    net::{Ipv4Addr, SocketAddr, TcpListener},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, Weak,
    },
    thread::JoinHandle,
    time::Duration,
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

mod openapi;

/// Implemented by the desktop host. The core never depends on Tauri.
pub trait DesktopApi: Send + Sync {
    fn call(&self, operation: &str, body: Value) -> Result<DesktopReply>;
}

pub struct DesktopReply {
    pub data: Value,
    /// Exit and installer launch must happen only after the HTTP response.
    pub after_response: Option<Box<dyn FnOnce() -> Result<()> + Send>>,
}

impl DesktopReply {
    pub fn immediate(data: Value) -> Self {
        Self {
            data,
            after_response: None,
        }
    }
}

pub const VERSION: &str = "v1";
const MAX_BODY: usize = 1024 * 1024;
const MAX_IN_FLIGHT: usize = 8;
const TOKEN_FILE: &str = "config/api-token.sha256";

/// A running listener. Dropping it unblocks the accept loop and joins the
/// thread, so toggling the setting off really closes the port.
pub struct ApiServer {
    server: Arc<Server>,
    port: u16,
    thread: Option<JoinHandle<()>>,
}

impl ApiServer {
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for ApiServer {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Default)]
pub(crate) struct ApiState {
    server: Mutex<Option<ApiServer>>,
    desktop: Mutex<Option<Arc<dyn DesktopApi>>>,
}

/// Status the interface shows in Sol menü → API.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStatus {
    pub enabled: bool,
    pub port: u16,
    pub listening: bool,
    pub token_saved: bool,
    pub base_url: String,
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

impl Manager {
    pub fn attach_desktop_api(&self, desktop: Arc<dyn DesktopApi>) {
        *self.api.desktop.lock().unwrap_or_else(|e| e.into_inner()) = Some(desktop);
    }

    fn desktop_api(&self) -> Option<Arc<dyn DesktopApi>> {
        self.api
            .desktop
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn api_token_path(&self) -> PathBuf {
        self.home.join(TOKEN_FILE)
    }

    pub fn api_status(&self) -> ApiStatus {
        let settings = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .api
            .clone();
        let listening = self
            .api
            .server
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|server| server.port() == settings.port);
        ApiStatus {
            enabled: settings.enabled,
            port: settings.port,
            listening,
            token_saved: self.api_token_path().is_file(),
            base_url: format!("http://127.0.0.1:{}/api/{VERSION}", settings.port),
        }
    }

    /// Create a fresh token, store only its hash and return the token once.
    pub fn create_api_token(&self) -> Result<String> {
        let token = format!(
            "sb_{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let path = self.api_token_path();
        std::fs::create_dir_all(path.parent().context("Yapılandırma klasörü yok.")?)?;
        storage::atomic_write(&path, hash_token(&token)).context("API jetonu kaydedilemedi.")?;
        self.log("Yeni API jetonu oluşturuldu. Önceki jeton geçersiz.");
        Ok(token)
    }

    pub fn clear_api_token(&self) -> Result<()> {
        let path = self.api_token_path();
        if path.exists() {
            std::fs::remove_file(&path).context("API jetonu silinemedi.")?;
        }
        self.log("API jetonu silindi. API istekleri artık kabul edilmez.");
        Ok(())
    }

    fn api_token_matches(&self, presented: &str) -> bool {
        let Ok(stored) = storage::read_limited(&self.api_token_path(), 4096) else {
            return false;
        };
        let stored = String::from_utf8_lossy(&stored).trim().to_string();
        constant_time_eq(stored.as_bytes(), hash_token(presented).as_bytes())
    }

    /// Bring the listener in line with the saved settings: start it when
    /// enabled, move it when the port changed, close it when disabled.
    pub fn ensure_api(self: &Arc<Self>) -> Result<()> {
        let settings = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .api
            .clone();
        let mut slot = self.api.server.lock().unwrap_or_else(|e| e.into_inner());
        match (&*slot, settings.enabled) {
            (Some(server), true) if server.port() == settings.port => Ok(()),
            (_, false) => {
                if slot.take().is_some() {
                    self.log("Yönetim API'si kapatıldı.");
                }
                Ok(())
            }
            (_, true) => {
                slot.take();
                let server = start_server(Arc::downgrade(self), settings.port)?;
                self.log(format!(
                    "Yönetim API'si dinliyor: http://127.0.0.1:{}/api/{VERSION}",
                    settings.port
                ));
                *slot = Some(server);
                Ok(())
            }
        }
    }

    pub fn stop_api(&self) {
        self.api
            .server
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
    }

    /// Resolve a project by id or by its name so scripts can use either.
    fn api_project_id(&self, key: &str) -> Result<String> {
        let config = self.config.lock().unwrap_or_else(|e| e.into_inner());
        config
            .projects
            .iter()
            .find(|p| p.id == key || p.name == key)
            .map(|p| p.id.clone())
            .context("Proje bulunamadı.")
    }
}

fn start_server(manager: Weak<Manager>, port: u16) -> Result<ApiServer> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = TcpListener::bind(address)
        .with_context(|| format!("API portu {port} bağlanamadı; başka bir uygulama kullanıyor."))?;
    let server = Arc::new(
        Server::from_listener(listener, None)
            .map_err(|error| anyhow::anyhow!("API sunucusu başlatılamadı: {error}"))?,
    );
    let accept = server.clone();
    let in_flight = Arc::new(AtomicUsize::new(0));
    let thread = std::thread::Builder::new()
        .name("serverbond-api".into())
        .spawn(move || {
            for request in accept.incoming_requests() {
                let Some(manager) = manager.upgrade() else {
                    break;
                };
                if in_flight.load(Ordering::Acquire) >= MAX_IN_FLIGHT {
                    respond(
                        request,
                        503,
                        json!({ "ok": false, "error": "Çok fazla eş zamanlı istek." }),
                    );
                    continue;
                }
                in_flight.fetch_add(1, Ordering::AcqRel);
                let counter = in_flight.clone();
                // One thread per request: a long installation must not block
                // status polls from other clients.
                let spawned = std::thread::Builder::new()
                    .name("serverbond-api-request".into())
                    .spawn(move || {
                        handle(&manager, request);
                        counter.fetch_sub(1, Ordering::AcqRel);
                    });
                if spawned.is_err() {
                    in_flight.fetch_sub(1, Ordering::AcqRel);
                }
            }
        })
        .context("API iş parçacığı başlatılamadı.")?;
    Ok(ApiServer {
        server,
        port,
        thread: Some(thread),
    })
}

fn respond(request: Request, status: u16, body: Value) {
    let bytes = serde_json::to_vec(&body).unwrap_or_else(|_| b"{\"ok\":false}".to_vec());
    let mut response = Response::from_data(bytes).with_status_code(StatusCode(status));
    for (name, value) in [
        ("Content-Type", "application/json; charset=utf-8"),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
    ] {
        if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response = response.with_header(header);
        }
    }
    let _ = request.respond(response);
}

/// HTTP outcome of one call: status and JSON payload.
struct Reply {
    status: u16,
    body: Value,
    after_response: Option<Box<dyn FnOnce() -> Result<()> + Send>>,
}

fn ok(data: impl serde::Serialize) -> Result<Reply> {
    Ok(Reply {
        status: 200,
        body: json!({ "ok": true, "data": data }),
        after_response: None,
    })
}

fn fail(status: u16, message: impl Into<String>) -> Reply {
    Reply {
        status,
        body: json!({ "ok": false, "error": message.into() }),
        after_response: None,
    }
}

fn handle(manager: &Arc<Manager>, mut request: Request) {
    let method = request.method().clone();
    let url = request.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
    let path = path.trim_end_matches('/').to_string();
    let query = query.to_string();

    let presented = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Authorization"))
        .map(|h| h.value.as_str().to_string())
        .and_then(|value| {
            value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("bearer "))
                .map(str::trim)
                .map(str::to_string)
        });

    if path == format!("/api/{VERSION}/health") && method == Method::Get {
        let reply = health(manager);
        return respond(request, reply.status, reply.body);
    }
    let authorised = presented.is_some_and(|token| manager.api_token_matches(&token));
    if !authorised {
        return respond(
            request,
            401,
            json!({
                "ok": false,
                "error": "Geçerli bir API jetonu gerekli (Authorization: Bearer …). Sol menü → API bölümünden oluşturun."
            }),
        );
    }

    let mut body = Vec::new();
    if request
        .body_length()
        .is_some_and(|length| length > MAX_BODY)
    {
        return respond(request, 413, fail(413, "İstek gövdesi 1 MB'ı aşıyor.").body);
    }
    if let Err(error) = request
        .as_reader()
        .take(MAX_BODY as u64 + 1)
        .read_to_end(&mut body)
    {
        return respond(
            request,
            400,
            fail(400, format!("Gövde okunamadı: {error}")).body,
        );
    }
    if body.len() > MAX_BODY {
        return respond(request, 413, fail(413, "İstek gövdesi 1 MB'ı aşıyor.").body);
    }

    let manager = manager.clone();
    let reply = manager.contain(|| Ok(route(&manager, &method, &path, &query, &body)));
    let reply = match reply {
        Ok(Ok(reply)) => reply,
        Ok(Err(error)) => fail(400, format!("{error:#}")),
        Err(error) => fail(500, format!("{error:#}")),
    };
    respond(request, reply.status, reply.body);
    if let Some(work) = reply.after_response {
        if let Err(error) = manager.contain(work) {
            manager.log(format!("API ertelenmiş işlemi tamamlanamadı: {error:#}"));
        }
    }
}

fn health(manager: &Manager) -> Reply {
    let snapshot = manager.snapshot();
    let (busy, running) = snapshot
        .as_ref()
        .map(|s| (s.busy, s.any_running))
        .unwrap_or((false, false));
    Reply {
        status: 200,
        after_response: None,
        body: json!({
            "ok": true,
            "data": {
                "name": crate::product::NAME,
                "version": env!("CARGO_PKG_VERSION"),
                "api": VERSION,
                "busy": busy,
                "anyRunning": running,
                "recoveryIssue": manager.recovery_issue(),
            }
        }),
    }
}

fn parse<T: for<'de> Deserialize<'de>>(body: &[u8]) -> Result<T> {
    if body.is_empty() {
        bail!("JSON gövde gerekli.");
    }
    serde_json::from_slice(body).context("JSON gövde okunamadı.")
}

fn query_value(query: &str, key: &str) -> Option<String> {
    reqwest::Url::parse(&format!("http://localhost/?{query}"))
        .ok()?
        .query_pairs()
        .find_map(|(k, v)| (k == key).then(|| v.into_owned()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AddProject {
    name: String,
    path: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateProject {
    name: String,
    parent: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportGit {
    url: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    branch: String,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
#[derive(Default)]
struct ImportGithub {
    repository: String,
    name: String,
    branch: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Paths {
    paths: Vec<PathBuf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Jobs {
    workers: Vec<QueueWorker>,
    schedule: ProjectSchedule,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Content {
    content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Version {
    version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Password {
    password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Token {
    token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AutoStart {
    auto_start: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Job {
    #[serde(default)]
    job: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FilePath {
    path: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PermissionInput {
    defender: bool,
}

fn desktop_call(manager: &Manager, operation: &str, body: &[u8]) -> Result<Reply> {
    let Some(host) = manager.desktop_api() else {
        return Ok(fail(
            501,
            "Bu işlem masaüstü uygulamasının API'sinde kullanılabilir.",
        ));
    };
    let input = if body.is_empty() {
        json!({})
    } else {
        parse(body)?
    };
    let reply = host.call(operation, input)?;
    Ok(Reply {
        status: if reply.after_response.is_some() {
            202
        } else {
            200
        },
        body: json!({ "ok": true, "data": reply.data }),
        after_response: reply.after_response,
    })
}

/// Human-readable route index served at `/api/v1`.
pub fn routes() -> Vec<(&'static str, &'static str, &'static str)> {
    #[derive(Deserialize)]
    struct Route {
        method: String,
        path: String,
        description: String,
    }
    static ROUTES: std::sync::LazyLock<Vec<Route>> = std::sync::LazyLock::new(|| {
        serde_json::from_str(include_str!("../api-routes.json")).expect("embedded API routes")
    });
    ROUTES
        .iter()
        .map(|route| {
            (
                route.method.as_str(),
                route.path.as_str(),
                route.description.as_str(),
            )
        })
        .collect()
}

fn route(
    manager: &Arc<Manager>,
    method: &Method,
    path: &str,
    query: &str,
    body: &[u8],
) -> Result<Reply> {
    let prefix = format!("/api/{VERSION}");
    let Some(rest) = path
        .strip_prefix(prefix.as_str())
        .filter(|rest| rest.is_empty() || rest.starts_with('/'))
    else {
        return Ok(fail(404, "Bilinmeyen yol. /api/v1 altında istek yapın."));
    };
    let segments: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
    let get = *method == Method::Get;
    let post = *method == Method::Post;
    let put = *method == Method::Put;
    let delete = *method == Method::Delete;

    match segments.as_slice() {
        [] if get => ok(routes()
            .into_iter()
            .map(|(m, p, d)| json!({ "method": m, "path": p, "description": d }))
            .collect::<Vec<_>>()),
        ["openapi.json"] if get => Ok(Reply {
            status: 200,
            body: openapi::document(),
            after_response: None,
        }),
        ["capabilities"] if get => ok(
            json!({ "apiVersion": VERSION, "desktop": manager.desktop_api().is_some(), "authentication": "bearer", "maxBodyBytes": MAX_BODY, "maxConcurrentRequests": MAX_IN_FLIGHT }),
        ),
        ["api"] if get => ok(manager.api_status()),
        ["api"] if put => {
            manager.save_api_settings(parse(body)?)?;
            manager.ensure_api()?;
            ok(manager.api_status())
        }
        ["api", "token"] if post => ok(json!({ "token": manager.create_api_token()? })),
        ["api", "token"] if delete => {
            manager.clear_api_token()?;
            ok(json!({ "revoked": true }))
        }
        ["settings", "validate"] if post => {
            if body.len() > 256 * 1024 {
                bail!("Ayar dosyası 256 KB sınırını aşıyor.");
            }
            let settings: Settings = parse(body)?;
            settings.validate()?;
            ok(settings)
        }
        ["permissions"] if get => ok(manager.permission_state()),
        ["permissions", "ensure"] if post => {
            manager.ensure_permissions()?;
            ok(manager.permission_state())
        }
        ["permissions", "grant"] if post => {
            let input: PermissionInput = parse(body)?;
            ok(manager.grant_permissions(input.defender)?)
        }
        ["recovery"] if post => {
            manager.recover_configuration()?;
            manager.ensure_api()?;
            ok(json!({ "recovered": true }))
        }
        ["mysql", "credentials"] if get => ok(json!({ "text": manager.credentials()? })),
        ["postgres", "credentials"] if get => {
            ok(json!({ "text": manager.postgres_credentials()? }))
        }
        ["system", "open-home"] if post => {
            manager.open_home()?;
            ok(json!({ "opened": true }))
        }
        ["system", "runtime-download"] if post => {
            manager.open_runtime_download()?;
            ok(json!({ "opened": true }))
        }
        ["tunnel", "apply"] if post => {
            let input: Token = parse(body)?;
            manager.apply_tunnel(&input.token)?;
            ok(json!({ "started": true }))
        }
        ["desktop"] if get => desktop_call(manager, "status", body),
        ["desktop", "appearance"] if get => desktop_call(manager, "appearance-get", body),
        ["desktop", "appearance"] if put => desktop_call(manager, "appearance-save", body),
        ["desktop"] if put => desktop_call(manager, "save", body),
        ["desktop", action @ ("show" | "hide" | "menu" | "exit" | "restart" | "navigate")]
            if post =>
        {
            desktop_call(manager, action, body)
        }
        ["updates"] if get => desktop_call(manager, "update-check", body),
        ["updates", "status"] if get => desktop_call(manager, "update-status", body),
        ["updates", "install"] if post => desktop_call(manager, "update-install", body),
        ["status"] if get => ok(manager.snapshot()?),
        ["requirements"] if get => ok(manager.requirements()),
        ["logs", id] if get => ok(json!({ "text": manager.read_log(id)? })),
        ["settings"] if get => ok(manager.snapshot()?.settings),
        ["settings"] if put => {
            let settings: Settings = parse(body)?;
            manager.save_settings(settings)?;
            manager.ensure_api()?;
            ok(manager.snapshot()?.settings)
        }
        ["settings", "defaults"] if get => ok(manager.default_settings()),
        ["settings", "previous"] if get => ok(manager.previous_settings()?),
        ["services"] if get => ok(service_inventory(manager)?),
        ["services", id] if get => {
            let id = canonical_service_id(id);
            match service_inventory(manager)?
                .into_iter()
                .find(|item| item["id"] == id)
            {
                Some(item) => ok(item),
                None => Ok(fail(404, "Hizmet bulunamadı.")),
            }
        }
        ["php"] if get => ok(manager.snapshot()?.php_versions),
        ["github"] if get => ok(manager.snapshot()?.github),
        ["github", "import"] if post => {
            let input: ImportGithub = parse(body)?;
            ok(manager.import_github_project(&input.repository, input.name, input.branch)?)
        }
        ["services", id, action] if post => {
            service_action(manager, id, action)?;
            ok(json!({ "id": id, "action": action }))
        }
        ["php", version, "select"] if post => {
            manager.select_php(version)?;
            ok(json!({ "version": version }))
        }
        ["php", version, "repair"] if post => {
            manager.repair_php(version)?;
            ok(json!({ "version": version }))
        }
        ["projects"] if get => ok(manager.snapshot()?.projects),
        ["projects"] if post => {
            let input: AddProject = parse(body)?;
            ok(manager.add_project(input.name, input.path)?)
        }
        ["projects", "create"] if post => {
            let input: CreateProject = parse(body)?;
            ok(manager.create_project(input.name, input.parent)?)
        }
        ["projects", "import"] if post => {
            let input: ImportGit = parse(body)?;
            ok(manager.import_git_project(&input.url, input.name, input.branch)?)
        }
        ["projects", "discover"] if get => ok(manager.discover_projects()?),
        ["projects", "import-folders"] if post => {
            let input: Paths = parse(body)?;
            ok(manager.import_projects(input.paths)?)
        }
        ["projects", key, tail @ ..] => {
            let id = manager.api_project_id(key)?;
            project_route(manager, &id, method, tail, query, body)
        }
        ["mysql", "password"] if post => {
            let input: Password = parse(body)?;
            manager.change_mysql_password(&input.password)?;
            ok(json!({ "changed": true }))
        }
        ["postgres", "password"] if post => {
            let input: Password = parse(body)?;
            manager.change_postgres_password(&input.password)?;
            ok(json!({ "changed": true }))
        }
        ["tunnel", "token"] if post => {
            let input: Token = parse(body)?;
            manager.save_tunnel_token(&input.token)?;
            ok(json!({ "saved": true }))
        }
        ["tunnel", "token"] if delete => {
            manager.clear_tunnel_token()?;
            ok(json!({ "saved": false }))
        }
        ["tunnel", "auto-start"] if post => {
            let input: AutoStart = parse(body)?;
            manager.save_tunnel_auto_start(input.auto_start)?;
            ok(json!({ "autoStart": input.auto_start }))
        }
        ["github", "token"] if post => {
            let input: Token = parse(body)?;
            manager.save_github_token(&input.token)?;
            ok(json!({ "saved": true }))
        }
        ["github", "token"] if delete => {
            manager.clear_github_token()?;
            ok(json!({ "saved": false }))
        }
        ["https", "trust"] if post => {
            manager.trust_https()?;
            ok(json!({ "trusted": true }))
        }
        ["https", "untrust"] if post => {
            manager.untrust_https()?;
            ok(json!({ "trusted": false }))
        }
        _ => Ok(fail(
            404,
            format!("Yol bulunamadı: {} {}", method.as_str(), path),
        )),
    }
}

fn service_action(manager: &Manager, id: &str, action: &str) -> Result<()> {
    let id = canonical_service_id(id);
    if !service_actions(id).contains(&action) {
        bail!("Bu hizmet için işlem desteklenmiyor: {id}/{action}");
    }
    if action == "open" {
        return match id {
            "phpmyadmin" => manager.open_phpmyadmin(),
            "mail" => manager.open_mail(),
            _ => bail!("Bu hizmet için açılacak arayüz yok."),
        };
    }
    let tool = |install: fn(&Manager) -> Result<()>,
                repair: fn(&Manager) -> Result<()>,
                start: fn(&Manager) -> Result<()>,
                stop: fn(&Manager) -> Result<()>|
     -> Result<()> {
        match action.parse::<ToolAction>() {
            Ok(ToolAction::Install) => install(manager),
            Ok(ToolAction::Repair) => repair(manager),
            Ok(ToolAction::Start) => start(manager),
            Ok(ToolAction::Stop) => stop(manager),
            Ok(ToolAction::Open) => bail!("API üzerinden pencere açılamaz."),
            Err(_) if action == "restart" => {
                stop(manager)?;
                start(manager)
            }
            Err(error) => Err(error),
        }
    };
    match id {
        "mail" => tool(
            Manager::install_mail,
            Manager::repair_mail,
            Manager::start_mail,
            Manager::stop_mail,
        ),
        "postgres" => tool(
            Manager::install_postgres,
            Manager::repair_postgres,
            Manager::start_postgres,
            Manager::stop_postgres,
        ),
        "redis" => tool(
            Manager::install_redis,
            Manager::repair_redis,
            Manager::start_redis,
            Manager::stop_redis,
        ),
        "tunnel" => tool(
            Manager::install_tunnel,
            Manager::repair_tunnel,
            Manager::start_tunnel,
            Manager::stop_tunnel,
        ),
        "node" => match action {
            "install" => manager.install_node(),
            "repair" => manager.repair_node(),
            _ => bail!("Node.js yalnızca install ve repair destekler."),
        },
        _ => match action {
            "install" => manager.install(id),
            "repair" => manager.repair(id),
            "start" => manager.start(id),
            "stop" => manager.stop(id),
            "restart" if id == "all" => manager.restart(),
            "restart" => {
                manager.stop(id)?;
                manager.start(id)
            }
            _ => bail!("Bilinmeyen hizmet işlemi: {action}"),
        },
    }
}

fn canonical_service_id(id: &str) -> &str {
    match id {
        "cloudflared" => "tunnel",
        "mailpit" => "mail",
        _ => id,
    }
}

fn service_actions(id: &str) -> &'static [&'static str] {
    match id {
        "all" => &["install", "start", "stop", "restart"],
        "php" | "mysql" | "caddy" | "postgres" | "redis" | "tunnel" => {
            &["install", "repair", "start", "stop", "restart"]
        }
        "mail" => &["install", "repair", "start", "stop", "restart", "open"],
        "composer" | "node" => &["install", "repair"],
        "phpmyadmin" => &["install", "repair", "open"],
        _ => &[],
    }
}

fn service_inventory(manager: &Manager) -> Result<Vec<Value>> {
    let snapshot = manager.snapshot()?;
    let mut items = vec![
        json!({"id":"all","name":"Sunucu","actions":service_actions("all"),"state":{"running":snapshot.any_running,"busy":snapshot.busy}}),
    ];
    for package in snapshot.packages {
        let id = package.package.id.clone();
        items.push(json!({"id":id,"name":package.package.name,"actions":service_actions(&id),"state":package}));
    }
    for (id, name, state) in [
        (
            "tunnel",
            "Cloudflared",
            serde_json::to_value(snapshot.tunnel)?,
        ),
        ("mail", "Mailpit", serde_json::to_value(snapshot.mail)?),
        (
            "postgres",
            "PostgreSQL",
            serde_json::to_value(snapshot.postgres)?,
        ),
        ("redis", "Redis", serde_json::to_value(snapshot.redis)?),
        ("node", "Node.js", serde_json::to_value(snapshot.node)?),
    ] {
        items.push(json!({"id":id,"name":name,"actions":service_actions(id),"state":state}));
    }
    Ok(items)
}

/// Available through IPC even when the HTTP listener is disabled.
pub fn documentation() -> Value {
    json!({"routes": routes().into_iter().map(|(method, path, description)| json!({"method":method,"path":path,"description":description})).collect::<Vec<_>>(), "document":openapi::document()})
}

fn project_route(
    manager: &Manager,
    id: &str,
    method: &Method,
    tail: &[&str],
    query: &str,
    body: &[u8],
) -> Result<Reply> {
    let get = *method == Method::Get;
    let post = *method == Method::Post;
    let put = *method == Method::Put;
    let delete = *method == Method::Delete;
    match tail {
        [] if get => ok(manager
            .snapshot()?
            .projects
            .into_iter()
            .find(|p| p.project.id == id)
            .context("Proje bulunamadı.")?),
        [] if delete => {
            manager.remove_project(id)?;
            ok(json!({ "removed": true }))
        }
        ["php"] if post => {
            let input: Version = parse(body)?;
            manager.select_project_php(id, &input.version)?;
            ok(json!({ "version": input.version }))
        }
        ["php", "repair"] if post => {
            let input: Version = parse(body)?;
            manager.repair_project_php(id, &input.version)?;
            ok(json!({ "version": input.version }))
        }
        ["open"] if post => {
            manager.open_project(id)?;
            ok(json!({ "opened": true }))
        }
        ["terminal"] if post => {
            manager.open_project_terminal(id)?;
            ok(json!({ "opened": true }))
        }
        ["deploy"] if post => ok(manager.deploy_project(id)?),
        ["releases"] if get => ok(manager.list_project_releases(id)?),
        ["release"] if get => ok(manager.project(id)?.release),
        ["release"] if put => {
            let release: ProjectRelease = parse(body)?;
            manager.save_project_release(id, release)?;
            ok(manager.project(id)?.release)
        }
        ["git"] if get => ok(manager.project_git_status(id)?),
        ["env"] if get => ok(manager.read_project_env(id)?),
        ["env"] if put => {
            let input: Content = parse(body)?;
            manager.save_project_env(id, input.content)?;
            ok(json!({ "saved": true }))
        }
        ["jobs"] if get => {
            let project = manager.project(id)?;
            ok(json!({ "workers": project.workers, "schedule": project.schedule }))
        }
        ["jobs"] if put => {
            let input: Jobs = parse(body)?;
            manager.save_project_jobs(id, input.workers, input.schedule)?;
            let project = manager.project(id)?;
            ok(json!({ "workers": project.workers, "schedule": project.schedule }))
        }
        ["workers", worker, action] if post => {
            match *action {
                "start" => manager.start_project_worker(id, worker)?,
                "stop" => manager.stop_project_worker(id, worker)?,
                "restart" => manager.restart_project_worker(id, worker)?,
                other => bail!("Bilinmeyen işçi işlemi: {other}"),
            }
            ok(json!({ "worker": worker, "action": action }))
        }
        ["schedule"] if get => ok(json!({ "text": manager.list_project_schedule(id)? })),
        ["schedule", action] if post => {
            match *action {
                "start" => manager.start_project_schedule(id)?,
                "stop" => manager.stop_project_schedule(id)?,
                "restart" => manager.restart_project_schedule(id)?,
                other => bail!("Bilinmeyen zamanlayıcı işlemi: {other}"),
            }
            ok(json!({ "action": action }))
        }
        ["failed-jobs"] if get => ok(json!({ "text": manager.list_failed_jobs(id)? })),
        ["failed-jobs", "retry"] if post => {
            let input: Job = if body.is_empty() {
                Job { job: None }
            } else {
                parse(body)?
            };
            ok(json!({ "text": manager.retry_failed_jobs(id, input.job.as_deref())? }))
        }
        ["failed-jobs", "flush"] if post => ok(json!({ "text": manager.flush_failed_jobs(id)? })),
        ["logs"] if get => {
            let source = query_value(query, "source");
            ok(json!({ "text": manager.read_project_log(id, source.as_deref().unwrap_or("php"))? }))
        }
        ["database", "create"] if post => {
            let name = manager.project(id)?.name;
            manager.create_database(&name)?;
            ok(json!({ "database": crate::model::database_name(&name) }))
        }
        ["database", "backup"] if post => {
            let name = manager.project(id)?.name;
            ok(json!({ "path": manager.backup_database(&name)? }))
        }
        ["database", "restore"] if post => {
            let input: FilePath = parse(body)?;
            let name = manager.project(id)?.name;
            manager.restore_database(&name, input.path)?;
            ok(json!({ "restored": true }))
        }
        _ => Ok(fail(
            404,
            format!(
                "Proje yolu bulunamadı: {} /{}",
                method.as_str(),
                tail.join("/")
            ),
        )),
    }
}

/// Wait until the listener answers `/health`; used by the CLI and tests.
pub fn wait_ready(port: u16, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if std::net::TcpStream::connect_timeout(
            &SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
            Duration::from_millis(200),
        )
        .is_ok()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_hashes_compare_in_constant_time_by_length_and_content() {
        let a = hash_token("sb_one");
        assert_eq!(a, hash_token("sb_one"));
        assert_ne!(a, hash_token("sb_two"));
        assert!(constant_time_eq(a.as_bytes(), a.as_bytes()));
        assert!(!constant_time_eq(a.as_bytes(), b"short"));
    }

    #[test]
    fn query_values_are_looked_up_by_key() {
        assert_eq!(
            query_value("source=worker:abc&x=1", "source"),
            Some("worker:abc".into())
        );
        assert_eq!(
            query_value("source=worker%3Aabc&x=1", "source"),
            Some("worker:abc".into())
        );
        assert_eq!(query_value("x=1", "source"), None);
        assert_eq!(query_value("", "source"), None);
    }

    #[test]
    fn route_index_covers_every_area() {
        let index = routes();
        for needle in ["/status", "/projects", "/services/", "/settings", "/deploy"] {
            assert!(
                index.iter().any(|(_, path, _)| path.contains(needle)),
                "{needle}"
            );
        }
    }
}
