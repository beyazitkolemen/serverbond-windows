//! Outbound-only Cloud connector. Credentials never enter snapshots or logs.

use crate::{secrets, storage, Manager};
use anyhow::{bail, Context, Result};
use reqwest::{blocking::Client, redirect::Policy, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
mod database;
mod desktop;
mod environment;
mod github;
mod jobs;
mod mysql;
mod operations;
mod postgres;
mod settings;
mod tunnel;
pub const DEFAULT_CLOUD_URL: &str = "https://serverbond.on-forge.com";
const SERVICES: &[&str] = &[
    "all",
    "php",
    "mysql",
    "caddy",
    "mail",
    "postgres",
    "redis",
    "tunnel",
    "composer",
    "phpmyadmin",
    "node",
];
const SERVICE_ACTIONS: &[&str] = &["install", "repair", "start", "stop", "restart"];

// Project paths, credentials and raw snapshot fields never enter the heartbeat.
fn service_report(inventory: Vec<Value>) -> Vec<Value> {
    inventory
        .into_iter()
        .filter_map(|item| {
            let id = item["id"].as_str()?;
            if !SERVICES.contains(&id) {
                return None;
            }
            let actions: Vec<&str> = item["actions"]
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .filter(|action| SERVICE_ACTIONS.contains(action))
                .collect();
            let state = &item["state"];
            let mut report = json!({
                "id": id,
                "running": state["running"].as_bool().unwrap_or(false),
                "actions": actions,
            });
            if let Some(installed) = state["installed"].as_bool() {
                report["installed"] = installed.into();
            }
            if let Some(version) = state["version"].as_str() {
                report["version"] = version.into();
            }
            Some(report)
        })
        .collect()
}
#[derive(Default)]
pub(crate) struct CloudState {
    started: AtomicBool,
    connector: Mutex<Option<std::thread::Thread>>,
    inner: Mutex<Runtime>,
}
impl CloudState {
    fn wake(&self) {
        if let Some(connector) = self
            .connector
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            connector.unpark();
        }
    }
}

type AfterAck = (String, Box<dyn FnOnce() -> Result<()> + Send>);
#[derive(Default)]
struct Runtime {
    completed: bool,
    active: Option<String>,
    progress: Vec<String>,
    last_socket_message: Option<String>,
    error: Option<String>,
    last_contact: Option<String>,
    disabled: bool,
    after_ack: Option<AfterAck>,
    connection: CloudConnection,
    socket_endpoint: Option<String>,
}
#[derive(Clone, Copy, Default, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum CloudConnection {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Retrying,
    Revoked,
}
#[derive(Serialize, Deserialize, Clone)]
struct Credentials {
    url: String,
    key: String,
    code_hash: String,
    device_id: Option<String>,
    name: Option<String>,
    account: Option<String>,
}
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudStatus {
    pub paired: bool,
    pub default_url: &'static str,
    pub connection: CloudConnection,
    pub socket_endpoint: Option<String>,
    pub url: Option<String>,
    pub name: Option<String>,
    pub account: Option<String>,
    pub last_contact: Option<String>,
    pub error: Option<String>,
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct Command {
    id: String,
    service: String,
    action: String,
    expires_at: String,
    #[serde(default)]
    operation: Option<String>,
    #[serde(default = "empty_parameters")]
    parameters: Value,
}
fn empty_parameters() -> Value {
    json!({})
}
#[derive(Deserialize)]
struct Poll {
    command: Option<Command>,
    #[serde(default)]
    commands_pending: bool,
}
#[derive(Deserialize)]
struct Pair {
    device_id: String,
    name: String,
    account: String,
}

fn connection_status(paired: bool, runtime: &Runtime) -> CloudConnection {
    if runtime.disabled {
        CloudConnection::Revoked
    } else if !paired {
        CloudConnection::Disconnected
    } else if runtime.connection == CloudConnection::Disconnected {
        CloudConnection::Connecting
    } else {
        runtime.connection
    }
}

fn pairing_code(input: &str) -> Result<String> {
    anyhow::ensure!(input.len() <= 256, "Bağlantı kodu çok uzun.");
    let code: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    anyhow::ensure!(
        code.len() == 32 && code.bytes().all(|b| b.is_ascii_hexdigit()),
        "Cloud’dan aldığınız 32 karakterli bağlantı kodunu yapıştırın."
    );
    Ok(code)
}

fn base_url(input: &str, allow_http: bool) -> Result<String> {
    let input = input.trim();
    let u = Url::parse(if input.is_empty() {
        DEFAULT_CLOUD_URL
    } else {
        input
    })
    .context("Geçerli bir Cloud adresi girin.")?;
    let loopback = matches!(u.host_str(), Some("127.0.0.1") | Some("[::1]"));
    if !(u.scheme() == "https" || (allow_http && u.scheme() == "http" && loopback))
        || !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
        || u.path() != "/"
    {
        bail!("HTTPS Cloud adresi gerekli. Yerel HTTP yalnızca geliştirme modunda loopback IP ile kullanılabilir.");
    }
    Ok(u.as_str().trim_end_matches('/').to_string())
}
fn allow_http() -> bool {
    cfg!(debug_assertions) && std::env::var("SERVERBOND_CLOUD_ALLOW_HTTP").as_deref() == Ok("1")
}
fn client() -> Result<Client> {
    Ok(Client::builder()
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()?)
}
fn post(client: &Client, c: &Credentials, path: &str, body: &Value) -> Result<(u16, Value)> {
    let response = client
        .post(format!("{}/api/agent/v1/{path}", c.url))
        .bearer_auth(&c.key)
        .json(body)
        .send()
        .map_err(|_| anyhow::anyhow!("Cloud bağlantısı kurulamadı."))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Ok((status, Value::Null));
    }
    let mut bytes = Vec::new();
    // Parameters are bounded separately; JSON escaping can expand Unicode paths.
    let limit = if path == "poll" {
        512 * 1024
    } else {
        256 * 1024
    };
    response.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= limit, "Cloud yanıtı çok büyük.");
    Ok((
        status,
        serde_json::from_slice(&bytes).context("Cloud yanıtı geçersiz.")?,
    ))
}
fn fingerprint(input: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(input.as_bytes()))
}
impl Manager {
    fn cloud_credentials(&self) -> Result<Option<Credentials>> {
        let path = self.home.join("config/cloud.dpapi");
        if !path.try_exists()? {
            return Ok(None);
        }
        Ok(Some(serde_json::from_str(&secrets::read(&path)?)?))
    }
    fn save_cloud_credentials(&self, c: &Credentials) -> Result<()> {
        secrets::save(
            &self.home.join("config/cloud.dpapi"),
            &serde_json::to_string(c)?,
        )
    }
    pub fn cloud_status(&self) -> Result<CloudStatus> {
        let runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
        let c = self.cloud_credentials()?;
        let paired = c.as_ref().is_some_and(|c| c.device_id.is_some());
        Ok(CloudStatus {
            paired: paired && !runtime.disabled,
            default_url: DEFAULT_CLOUD_URL,
            connection: connection_status(paired, &runtime),
            socket_endpoint: runtime.socket_endpoint.clone(),
            url: c.as_ref().map(|c| c.url.clone()),
            name: c.as_ref().and_then(|c| c.name.clone()),
            account: c.and_then(|c| c.account),
            last_contact: runtime.last_contact.clone(),
            error: runtime.error.clone(),
        })
    }
    pub fn cloud_pair(&self, url: &str, code: &str) -> Result<()> {
        let url = base_url(url, allow_http())?;
        let code = pairing_code(code)?;
        let mut runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
        anyhow::ensure!(runtime.active.is_none(), "Cloud işlemi devam ediyor.");
        let old = self.cloud_credentials()?;
        anyhow::ensure!(
            old.as_ref().is_none_or(|c| c.device_id.is_none()) || runtime.disabled,
            "Önce mevcut bağlantıyı kaldırın."
        );
        let mut c = match old {
            Some(c)
                if c.url == url && c.code_hash == fingerprint(&code) && c.device_id.is_none() =>
            {
                c
            }
            _ => Credentials {
                url,
                key: format!(
                    "{}{}",
                    uuid::Uuid::new_v4().simple(),
                    uuid::Uuid::new_v4().simple()
                ),
                code_hash: fingerprint(&code),
                device_id: None,
                name: None,
                account: None,
            },
        };
        // Persist first: the same key can complete pairing after a lost response.
        self.save_cloud_credentials(&c)?;
        let (status, reply) = post(&client()?, &c, "pair", &json!({"code":code,"key":c.key}))?;
        match status {
            200 => {},
            422 => bail!("Kod geçersiz veya başka bir cihazda kullanılmış. Cloud panelindeki cihazın bağlantı kodunu kontrol edin."),
            429 => bail!("Çok fazla bağlantı denemesi yapıldı. Bir dakika sonra yeniden deneyin."),
            _ => bail!("Cloud eşleştirmesi tamamlanamadı ({status}). Aynı kodla yeniden deneyebilirsiniz."),
        }
        let p: Pair = serde_json::from_value(reply)?;
        uuid::Uuid::parse_str(&p.device_id)?;
        c.device_id = Some(p.device_id);
        c.name = Some(p.name);
        c.account = Some(p.account);
        self.save_cloud_credentials(&c)?;
        runtime.disabled = false;
        runtime.error = None;
        runtime.last_contact = None;
        runtime.after_ack = None;
        runtime.connection = CloudConnection::Connecting;
        runtime.socket_endpoint = None;
        self.cloud.wake();
        Ok(())
    }
    pub fn cloud_disconnect(&self) -> Result<()> {
        let mut runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(c) = self.cloud_credentials()? {
            if c.device_id.is_some() && !runtime.disabled {
                let (status, _) = post(&client()?, &c, "revoke", &json!({}))?;
                anyhow::ensure!(
                    status == 200 || status == 401,
                    "Cloud iptal isteği başarısız ({status})."
                );
            }
            std::fs::remove_file(self.home.join("config/cloud.dpapi"))?;
        }
        runtime.disabled = false;
        runtime.error = None;
        runtime.last_contact = None;
        runtime.after_ack = None;
        runtime.connection = CloudConnection::Disconnected;
        runtime.socket_endpoint = None;
        self.cloud.wake();
        Ok(())
    }
    /// One connector per desktop Manager, stopped when the Manager is dropped.
    pub fn start_cloud(self: &Arc<Self>) {
        if self.cloud.started.swap(true, Ordering::AcqRel) {
            return;
        }
        let weak = Arc::downgrade(self);
        let connector = std::thread::spawn(move || {
            let mut delay = 5;
            loop {
                let Some(m) = weak.upgrade() else { break };
                if m.shutting_down.load(Ordering::Acquire) {
                    break;
                }
                match m.cloud_socket_session() {
                    Ok(()) => {
                        m.cloud
                            .inner
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .connection = CloudConnection::Disconnected;
                        delay = 5;
                    }
                    Err(_) => {
                        let mut runtime = m.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
                        runtime.connection = CloudConnection::Retrying;
                        if !runtime.disabled {
                            runtime.error = Some(
                                "Cloud bağlantısı veya yerel kayıt başarısız. Yeniden deneniyor."
                                    .into(),
                            );
                        }
                        drop(runtime);
                        // Keep command delivery and diagnostics available over HTTPS while Reverb recovers.
                        let _ = m.cloud_tick(false);
                        delay = (delay * 2).min(15);
                    }
                }
                drop(m);
                std::thread::park_timeout(Duration::from_secs(delay));
            }
        });
        *self
            .cloud
            .connector
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(connector.thread().clone());
    }
    fn cloud_journal_path(&self, c: &Credentials) -> Result<std::path::PathBuf> {
        let id = uuid::Uuid::parse_str(c.device_id.as_deref().context("Cihaz eşleştirilmedi.")?)?;
        Ok(self.home.join(format!("config/cloud-commands-{id}.json")))
    }
    fn journal(&self, c: &Credentials) -> Result<BTreeMap<String, String>> {
        let p = self.cloud_journal_path(c)?;
        if !p.try_exists()? {
            return Ok(BTreeMap::new());
        }
        Ok(serde_json::from_slice(&storage::read_limited(
            &p,
            4 * 1024 * 1024,
        )?)?)
    }
    fn save_journal(&self, c: &Credentials, journal: &BTreeMap<String, String>) -> Result<()> {
        storage::atomic_write(&self.cloud_journal_path(c)?, serde_json::to_vec(journal)?)
    }
    fn cloud_output_path(&self, c: &Credentials, id: &str) -> Result<std::path::PathBuf> {
        let device =
            uuid::Uuid::parse_str(c.device_id.as_deref().context("Cihaz eşleştirilmedi.")?)?;
        let command = uuid::Uuid::parse_str(id)?;
        Ok(self
            .home
            .join(format!("config/cloud-output-{device}-{command}.dpapi")))
    }
    pub(crate) fn cloud_stage(&self, stage: &str) {
        let mut runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
        if runtime.active.is_some()
            && runtime.progress.last().is_none_or(|last| last != stage)
            && runtime.progress.len() < 16
        {
            runtime.progress.push(stage.to_string());
        }
    }
    fn cloud_tick(self: &Arc<Self>, claim: bool) -> Result<()> {
        let c = {
            let runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
            if runtime.disabled {
                return Ok(());
            }
            let Some(c) = self.cloud_credentials()? else {
                return Ok(());
            };
            if c.device_id.is_none() {
                return Ok(());
            }
            base_url(&c.url, allow_http())?;
            c
        };
        let services = service_report(crate::api::service_inventory(self)?);
        let http = client()?;
        let pending = self
            .cloud
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .after_ack
            .as_ref()
            .map(|(id, _)| id.clone());
        if let Some(id) = pending {
            let output: Value =
                serde_json::from_str(&secrets::read(&self.cloud_output_path(&c, &id)?)?)?;
            let (status, _) = post(
                &http,
                &c,
                &format!("commands/{id}/result"),
                &json!({"status":"succeeded","output":output}),
            )?;
            if status == 401 {
                let mut runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
                runtime.disabled = true;
                runtime.after_ack = None;
            }
            anyhow::ensure!(
                status == 200,
                "Güncelleme hazırlığı Cloud tarafından onaylanmadı."
            );
            let mut runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
            if !runtime.disabled
                && self
                    .cloud_credentials()?
                    .as_ref()
                    .is_some_and(|current| current.key == c.key)
            {
                if let Some((_, work)) = runtime.after_ack.take() {
                    drop(runtime);
                    work()?;
                }
            }
            return Ok(());
        }
        let (progress, connection) = {
            let runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
            (
                runtime
                    .active
                    .as_ref()
                    .map(|id| json!({"commandId":id,"stages":runtime.progress})),
                json!({"state":runtime.connection,"lastSocketMessage":runtime.last_socket_message}),
            )
        };
        let (status, reply) = post(
            &http,
            &c,
            if claim { "poll" } else { "heartbeat" },
            &json!({"version":env!("CARGO_PKG_VERSION"),"services":services,"progress":progress,"connection":connection,"operations":operations::NAMES.iter().copied().filter(|name| !name.starts_with("desktop.") || self.desktop_api().is_some()).collect::<Vec<_>>()}),
        )?;
        let mut runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
        if self
            .cloud_credentials()?
            .as_ref()
            .is_none_or(|current| current.key != c.key)
        {
            return Ok(());
        }
        if status == 401 {
            runtime.disabled = true;
            runtime.error = Some("Cloud erişimi iptal edildi. Yeniden eşleştirin.".into());
            return Ok(());
        }
        anyhow::ensure!(status == 200, "Cloud isteği başarısız.");
        runtime.last_contact = Some(chrono::Utc::now().to_rfc3339());
        runtime.error = None;
        let p: Poll = serde_json::from_value(reply)?;
        if !claim && p.commands_pending {
            // Heartbeat is only a wake-up hint. Poll owns delivery and journal replay.
            drop(runtime);
            return self.cloud_tick(true);
        }
        let Some(command) = p.command else {
            return Ok(());
        };
        uuid::Uuid::parse_str(&command.id)?;
        let mut journal = self.journal(&c)?;
        if let Some(saved) = journal.get(&command.id).cloned() {
            if saved == "running" && runtime.active.as_deref() == Some(&command.id) {
                return Ok(());
            }
            let mut result = if saved == "running" {
                "uncertain"
            } else {
                &saved
            };
            let output_path = self.cloud_output_path(&c, &command.id)?;
            let mut output = None;
            if command.operation.is_some() && ["succeeded", "failed"].contains(&result) {
                let decoded = (|| -> Result<Option<Value>> {
                    if !output_path.try_exists()? {
                        return Ok(None);
                    }
                    let bytes = secrets::read(&output_path)?;
                    anyhow::ensure!(
                        bytes.len() <= operations::output_limit(command.operation.as_deref()),
                        "Cloud sonucu çok büyük."
                    );
                    Ok(Some(serde_json::from_str(&bytes)?))
                })();
                match decoded {
                    Ok(Some(value)) => output = Some(value),
                    Ok(None) if result == "failed" => {}
                    // Preserve execution history: lost/corrupt output is never a reason to re-run.
                    _ => result = "uncertain",
                }
            }
            journal.insert(command.id.clone(), result.into());
            self.save_journal(&c, &journal)?;
            drop(runtime);
            let mut body = json!({"status":result});
            if let Some(output) = output {
                body["output"] = output;
            }
            let (status, _) = post(&http, &c, &format!("commands/{}/result", command.id), &body)?;
            anyhow::ensure!(
                status == 200 || status == 401,
                "İşlem sonucu gönderilemedi."
            );
            if status == 200 {
                let mut runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
                if !runtime.disabled
                    && self
                        .cloud_credentials()?
                        .as_ref()
                        .is_some_and(|current| current.key == c.key)
                    && runtime
                        .after_ack
                        .as_ref()
                        .is_some_and(|(id, _)| id == &command.id)
                {
                    let (_, work) = runtime.after_ack.take().unwrap();
                    drop(runtime);
                    work()?;
                }
            }
            return Ok(());
        }
        if runtime.active.is_some() {
            return Ok(());
        }
        let expires = chrono::DateTime::parse_from_rfc3339(&command.expires_at)?;
        let parsed_operation = command
            .operation
            .as_ref()
            .map(|name| operations::Operation::parse(name, &command.parameters));
        let invalid = if let Some(operation) = &parsed_operation {
            operation.is_err()
        } else {
            !SERVICES.contains(&command.service.as_str())
                || !SERVICE_ACTIONS.contains(&command.action.as_str())
                || !services.iter().any(|service| {
                    service["id"] == command.service
                        && service["actions"].as_array().is_some_and(|actions| {
                            actions.iter().any(|action| action == &command.action)
                        })
                })
        };
        let state = if invalid {
            "failed"
        } else if expires <= chrono::Utc::now() {
            "expired"
        } else {
            "running"
        };
        anyhow::ensure!(
            journal.len() < 50000,
            "Cloud işlem günlüğü dolu; bağlantıyı yenileyin."
        );
        if invalid && command.operation.is_some() {
            secrets::save(&self.cloud_output_path(&c, &command.id)?, &json!({
                "error":"İşlem parametreleri bu Windows sürümüyle uyumlu değil. Windows uygulamasını güncelleyip formu yenileyin."
            }).to_string())?;
        }
        journal.insert(command.id.clone(), state.into());
        self.save_journal(&c, &journal)?;
        if state != "running" {
            runtime.completed = true;
            return Ok(());
        }
        runtime.active = Some(command.id.clone());
        runtime.progress = vec!["started".into()];
        let m = self.clone();
        std::thread::spawn(move || {
            let mut result = m.contain(|| match parsed_operation {
                Some(operation) => operation?.prepare(&m),
                None => crate::api::service_action(&m, &command.service, &command.action)
                    .map(|()| crate::api::DesktopReply::immediate(Value::Null)),
            });
            let mut runtime = m.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
            let saved = (|| -> Result<()> {
                if command.operation.is_some() {
                    let output = match &result {
                        Ok(value) => json!({"data":value.data}),
                        Err(error) if error.is::<crate::envfile::EnvConflict>() => {
                            json!({"error":crate::envfile::EnvConflict.to_string()})
                        }
                        Err(error) if error.is::<crate::preferences::SettingsConflict>() => {
                            json!({"error":crate::preferences::SettingsConflict.to_string()})
                        }
                        Err(error) if error.is::<crate::projects::ProjectError>() => {
                            json!({"error":error.downcast_ref::<crate::projects::ProjectError>().unwrap().to_string()})
                        }
                        // Raw errors can contain repository URLs or local secrets.
                        Err(_) => {
                            json!({"error":"İşlem tamamlanamadı. Girdileri, kurulu bileşenleri ve Windows uygulamasını kontrol edin."})
                        }
                    };
                    let encoded = serde_json::to_string(&output)?;
                    anyhow::ensure!(
                        encoded.len() <= operations::output_limit(command.operation.as_deref()),
                        "Cloud sonucu çok büyük."
                    );
                    secrets::save(&m.cloud_output_path(&c, &command.id)?, &encoded)?;
                }
                let mut journal = m.journal(&c)?;
                journal.insert(
                    command.id.clone(),
                    if result.is_ok() {
                        "succeeded"
                    } else {
                        "failed"
                    }
                    .into(),
                );
                m.save_journal(&c, &journal)
            })();
            if saved.is_err() {
                runtime.error = Some("İşlem sonucu kaydedilemedi; sonuç belirsiz.".into());
            }
            if saved.is_ok()
                && !runtime.disabled
                && m.cloud_credentials()
                    .ok()
                    .flatten()
                    .as_ref()
                    .is_some_and(|current| current.key == c.key)
            {
                if let Ok(reply) = &mut result {
                    runtime.after_ack = reply
                        .after_response
                        .take()
                        .map(|work| (command.id.clone(), work));
                }
            }
            runtime.active = None;
            runtime.completed = true;
        });
        Ok(())
    }
}

#[derive(Deserialize)]
struct SocketSettings {
    key: String,
    host: String,
    port: u16,
    scheme: String,
    channel: String,
}
impl Manager {
    fn cloud_socket_session(self: &Arc<Self>) -> Result<()> {
        use std::net::{TcpStream, ToSocketAddrs};
        use std::time::Instant;
        use tungstenite::{client::IntoClientRequest, stream::MaybeTlsStream, Message};
        let c = {
            let mut runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
            if runtime.disabled {
                return Ok(());
            }
            let Some(c) = self.cloud_credentials()? else {
                return Ok(());
            };
            if c.device_id.is_none() {
                return Ok(());
            }
            base_url(&c.url, allow_http())?;
            runtime.connection = CloudConnection::Connecting;
            runtime.socket_endpoint = None;
            c
        };
        let http = client()?;
        let (status, reply) = post(&http, &c, "socket", &json!({}))?;
        if status == 401 {
            let mut runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
            runtime.disabled = true;
            runtime.error = Some("Cloud erişimi iptal edildi. Yeniden eşleştirin.".into());
            return Ok(());
        }
        anyhow::ensure!(status == 200, "Reverb ayarları alınamadı.");
        let settings: SocketSettings = serde_json::from_value(reply)?;
        anyhow::ensure!(
            settings.channel == format!("private-devices.{}", c.device_id.as_deref().unwrap_or("")),
            "Geçersiz cihaz kanalı."
        );
        let endpoint = base_url(
            &format!("{}://{}:{}", settings.scheme, settings.host, settings.port),
            allow_http(),
        )?;
        let mut socket_url = Url::parse(&endpoint)?;
        socket_url
            .set_scheme(if settings.scheme == "https" {
                "wss"
            } else {
                "ws"
            })
            .map_err(|_| anyhow::anyhow!("Soket adresi geçersiz."))?;
        self.cloud
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .socket_endpoint = Some(socket_url.as_str().trim_end_matches('/').to_string());
        socket_url
            .path_segments_mut()
            .map_err(|_| anyhow::anyhow!("Soket adresi geçersiz."))?
            .clear()
            .extend(["app", &settings.key]);
        socket_url.set_query(Some("protocol=7&client=serverbond&version=1.0"));
        let mut request = socket_url.as_str().into_client_request()?;
        request.headers_mut().insert("Origin", c.url.parse()?);
        let addresses = (settings.host.as_str(), settings.port).to_socket_addrs()?;
        let stream = addresses
            .take(4)
            .find_map(|addr| TcpStream::connect_timeout(&addr, Duration::from_secs(5)).ok())
            .context("Reverb bağlantısı kurulamadı.")?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let mut config = tungstenite::protocol::WebSocketConfig::default();
        config.max_message_size = Some(65536);
        config.max_frame_size = Some(65536);
        let (mut socket, _) =
            tungstenite::client_tls_with_config(request, stream, Some(config), None)?;
        match socket.get_mut() {
            MaybeTlsStream::Plain(s) => s.set_read_timeout(Some(Duration::from_secs(1)))?,
            MaybeTlsStream::NativeTls(s) => {
                s.get_mut().set_read_timeout(Some(Duration::from_secs(1)))?
            }
            _ => bail!("Desteklenmeyen TLS bağlantısı."),
        }
        let mut subscribed = false;
        let started = Instant::now();
        let mut heartbeat = Instant::now();
        let mut last_message = Instant::now();
        let mut last_ping = Instant::now();
        loop {
            if self.shutting_down.load(Ordering::Acquire) || Arc::strong_count(self) == 1 {
                break;
            }
            let completed = {
                let mut runtime = self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
                if runtime.disabled
                    || self
                        .cloud_credentials()?
                        .as_ref()
                        .is_none_or(|current| current.key != c.key)
                {
                    break;
                }
                std::mem::take(&mut runtime.completed)
            };
            if subscribed && completed {
                // Completion is persisted first. Recover the same delivered command and send its result.
                self.cloud_tick(true)?;
            }
            if !subscribed && started.elapsed() > Duration::from_secs(20) {
                bail!("Reverb aboneliği zaman aşımına uğradı.");
            }
            if last_message.elapsed() > Duration::from_secs(45) {
                bail!("Reverb yanıt vermiyor.");
            }
            if subscribed && heartbeat.elapsed() >= Duration::from_secs(10) {
                self.cloud_tick(false)?;
                heartbeat = Instant::now();
            }
            if last_ping.elapsed() >= Duration::from_secs(15) {
                socket.send(Message::Text(
                    json!({"event":"pusher:ping","data":{}}).to_string().into(),
                ))?;
                last_ping = Instant::now();
            }
            let message = match socket.read() {
                Ok(m) => m,
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    continue
                }
                Err(e) => return Err(e.into()),
            };
            last_message = Instant::now();
            self.cloud
                .inner
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .last_socket_message = Some(chrono::Utc::now().to_rfc3339());
            match message {
                Message::Text(text) => {
                    let event: Value = serde_json::from_str(&text)?;
                    match event["event"].as_str().unwrap_or("") {
                        "pusher:connection_established" => {
                            let data: Value = match &event["data"] {
                                Value::String(s) => serde_json::from_str(s)?,
                                v => v.clone(),
                            };
                            let (status, auth) = post(
                                &http,
                                &c,
                                "authorize",
                                &json!({"socket_id":data["socket_id"],"channel_name":settings.channel}),
                            )?;
                            anyhow::ensure!(status == 200, "Reverb kanalı yetkilendirilemedi.");
                            socket.send(Message::Text(json!({"event":"pusher:subscribe","data":{"channel":settings.channel,"auth":auth["auth"]}}).to_string().into()))?;
                        }
                        "pusher_internal:subscription_succeeded"
                            if event["channel"] == settings.channel =>
                        {
                            subscribed = true;
                            self.cloud_tick(true)?;
                            let mut runtime =
                                self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
                            if !runtime.disabled
                                && self
                                    .cloud_credentials()?
                                    .as_ref()
                                    .is_some_and(|current| current.key == c.key)
                            {
                                runtime.connection = CloudConnection::Connected;
                            }
                        }
                        "command.ready" if subscribed && event["channel"] == settings.channel => {
                            self.cloud_tick(true)?
                        }
                        "connection.revoked" if event["channel"] == settings.channel => {
                            let mut runtime =
                                self.cloud.inner.lock().unwrap_or_else(|e| e.into_inner());
                            runtime.disabled = true;
                            runtime.error =
                                Some("Cloud erişimi iptal edildi. Yeniden eşleştirin.".into());
                            break;
                        }
                        "pusher:ping" => socket.send(Message::Text(
                            json!({"event":"pusher:pong","data":{}}).to_string().into(),
                        ))?,
                        "pusher:error" => bail!("Reverb protokol hatası."),
                        _ => {}
                    }
                }
                Message::Ping(_) => {
                    socket.flush()?;
                }
                Message::Close(_) => bail!("Reverb bağlantısı kapandı."),
                _ => {}
            }
        }
        let _ = socket.close(None);
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod windows_cloud_tests {
    use super::*;
    #[test]
    fn heartbeat_hint_polls_and_replays_saved_result_without_reexecution() {
        let home = tempfile::tempdir().unwrap();
        let m = Arc::new(Manager::new(home.path().into()).unwrap());
        let mut c = Credentials {
            url: String::new(),
            key: "b".repeat(64),
            code_hash: String::new(),
            device_id: Some(uuid::Uuid::new_v4().to_string()),
            name: None,
            account: None,
        };
        let id = uuid::Uuid::new_v4().to_string();
        let output = json!({"data":{"projects":[],"total":0,"offset":0}});
        secrets::save(&m.cloud_output_path(&c, &id).unwrap(), &output.to_string()).unwrap();
        m.save_journal(
            &c,
            &std::collections::BTreeMap::from([(id.clone(), "succeeded".into())]),
        )
        .unwrap();
        let requests = exchange_tick(&m, &mut c, vec![
            (200, json!({"command":null,"commands_pending":true})),
            (200, json!({"command":{"id":id,"service":"projects","action":"execute","operation":"projects.list","parameters":{},"expires_at":"2099-01-01T00:00:00Z"}})),
            (200, json!({"ok":true})),
        ], false).unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[2]["output"], output);
        assert_eq!(requests[2]["status"], "succeeded");
        assert!(m.cloud.inner.lock().unwrap().active.is_none());
        assert_eq!(
            exchange_tick(&m, &mut c, vec![(200, json!({"command":null}))], false)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn malformed_operation_persists_an_actionable_safe_error() {
        let home = tempfile::tempdir().unwrap();
        let m = Arc::new(Manager::new(home.path().into()).unwrap());
        let mut c = Credentials {
            url: String::new(),
            key: "b".repeat(64),
            code_hash: String::new(),
            device_id: Some(uuid::Uuid::new_v4().to_string()),
            name: None,
            account: None,
        };
        let id = uuid::Uuid::new_v4().to_string();
        let command = json!({"id":id,"service":"projects","action":"execute","operation":"projects.create","parameters":{"secret":"never-return-this"},"expires_at":"2099-01-01T00:00:00Z"});
        exchange(&m, &mut c, vec![(200, json!({"command":command}))]).unwrap();
        assert_eq!(m.journal(&c).unwrap()[&id], "failed");
        let requests = exchange(
            &m,
            &mut c,
            vec![(200, json!({"command":command})), (200, json!({"ok":true}))],
        )
        .unwrap();
        assert_eq!(requests[1]["status"], "failed");
        assert!(requests[1]["output"]["error"]
            .as_str()
            .unwrap()
            .contains("parametreleri"));
        assert!(!requests[1].to_string().contains("never-return-this"));
    }

    #[test]
    fn update_waits_for_ack_retries_failed_delivery_and_runs_only_once() {
        let home = tempfile::tempdir().unwrap();
        let m = Arc::new(Manager::new(home.path().into()).unwrap());
        let mut c = Credentials {
            url: String::new(),
            key: "b".repeat(64),
            code_hash: String::new(),
            device_id: Some(uuid::Uuid::new_v4().to_string()),
            name: None,
            account: None,
        };
        // Establish the fixture credentials and encrypted output before queuing installation.
        exchange(&m, &mut c, vec![(200, json!({"command":null}))]).unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let output = json!({"data":{"accepted":true,"version":"1.3.1","signatureVerified":true}});
        secrets::save(&m.cloud_output_path(&c, &id).unwrap(), &output.to_string()).unwrap();
        let installed = Arc::new(AtomicBool::new(false));
        let flag = installed.clone();
        m.cloud.inner.lock().unwrap().after_ack = Some((
            id.clone(),
            Box::new(move || {
                flag.store(true, Ordering::Release);
                Ok(())
            }),
        ));
        assert!(exchange(&m, &mut c, vec![(500, json!({}))]).is_err());
        assert!(!installed.load(Ordering::Acquire));
        let requests = exchange(&m, &mut c, vec![(200, json!({"ok":true}))]).unwrap();
        assert_eq!(requests[0]["output"], output);
        assert!(installed.load(Ordering::Acquire));
        assert!(m.cloud.inner.lock().unwrap().after_ack.is_none());
        let flag = installed.clone();
        installed.store(false, Ordering::Release);
        m.cloud.inner.lock().unwrap().after_ack = Some((
            id,
            Box::new(move || {
                flag.store(true, Ordering::Release);
                Ok(())
            }),
        ));
        assert!(exchange(&m, &mut c, vec![(401, json!({}))]).is_err());
        assert!(!installed.load(Ordering::Acquire));
        assert!(m.cloud.inner.lock().unwrap().after_ack.is_none());
    }
    fn exchange(
        m: &Arc<Manager>,
        c: &mut Credentials,
        replies: Vec<(u16, Value)>,
    ) -> Result<Vec<Value>> {
        exchange_tick(m, c, replies, true)
    }
    fn exchange_tick(
        m: &Arc<Manager>,
        c: &mut Credentials,
        replies: Vec<(u16, Value)>,
        claim: bool,
    ) -> Result<Vec<Value>> {
        std::env::set_var("SERVERBOND_CLOUD_ALLOW_HTTP", "1");
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        c.url = format!("http://{}", server.server_addr());
        m.save_cloud_credentials(c)?;
        let handler = std::thread::spawn(move || {
            let mut bodies = Vec::new();
            for (status, body) in replies {
                let mut request = server
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap()
                    .expect("missing request");
                let mut text = String::new();
                request.as_reader().read_to_string(&mut text).unwrap();
                bodies.push(serde_json::from_str(&text).unwrap());
                request
                    .respond(
                        tiny_http::Response::from_string(body.to_string()).with_status_code(status),
                    )
                    .unwrap();
            }
            bodies
        });
        let result = m.cloud_tick(claim);
        let bodies = handler.join().unwrap();
        result.map(|()| bodies)
    }
    #[test]
    fn persisted_results_are_replayed_and_interrupted_commands_are_not_reexecuted() {
        let home = tempfile::tempdir().unwrap();
        let m = Arc::new(Manager::new(home.path().to_path_buf()).unwrap());
        let mut c = Credentials {
            url: String::new(),
            key: "b".repeat(64),
            code_hash: String::new(),
            device_id: Some(uuid::Uuid::new_v4().to_string()),
            name: None,
            account: None,
        };
        let id = uuid::Uuid::new_v4().to_string();
        let command = json!({"command":{"id":id,"service":"redis","action":"stop","expires_at":(chrono::Utc::now()+chrono::Duration::minutes(1)).to_rfc3339()}});
        exchange(&m, &mut c, vec![(200, command.clone())]).unwrap();
        let start = std::time::Instant::now();
        while m.cloud.inner.lock().unwrap().active.is_some() {
            assert!(start.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(m.journal(&c).unwrap()[&id], "succeeded");
        let calls = exchange(
            &m,
            &mut c,
            vec![(200, command.clone()), (200, json!({"ok":true}))],
        )
        .unwrap();
        assert_eq!(calls[1]["status"], "succeeded");
        assert!(m.cloud.inner.lock().unwrap().active.is_none());
        let mut journal = m.journal(&c).unwrap();
        journal.insert(id.clone(), "running".into());
        m.save_journal(&c, &journal).unwrap();
        let calls = exchange(&m, &mut c, vec![(200, command), (200, json!({"ok":true}))]).unwrap();
        assert_eq!(calls[1]["status"], "uncertain");
        let invalid = uuid::Uuid::new_v4().to_string();
        exchange(&m,&mut c,vec![(200,json!({"command":{"id":invalid,"service":"shell","action":"start","expires_at":chrono::Utc::now().to_rfc3339()}}))]).unwrap();
        assert_eq!(m.journal(&c).unwrap()[&invalid], "failed");
        let expired = uuid::Uuid::new_v4().to_string();
        exchange(&m,&mut c,vec![(200,json!({"command":{"id":expired,"service":"redis","action":"stop","expires_at":(chrono::Utc::now()-chrono::Duration::seconds(1)).to_rfc3339()}}))]).unwrap();
        assert_eq!(m.journal(&c).unwrap()[&expired], "expired");
        let encrypted = std::fs::read(home.path().join("config/cloud.dpapi")).unwrap();
        assert!(!String::from_utf8_lossy(&encrypted).contains(&c.key));
        exchange(&m, &mut c, vec![(401, Value::Null)]).unwrap();
        assert!(!m.cloud_status().unwrap().paired);
    }
    #[test]
    fn project_results_survive_replay_and_are_encrypted_on_disk() {
        let home = tempfile::tempdir().unwrap();
        let m = Arc::new(Manager::new(home.path().into()).unwrap());
        let project = home.path().join("www/private-project");
        std::fs::create_dir_all(project.join("public")).unwrap();
        std::fs::write(project.join("public/index.php"), "<?php echo 'ok';").unwrap();
        let mut c = Credentials {
            url: String::new(),
            key: "b".repeat(64),
            code_hash: String::new(),
            device_id: Some(uuid::Uuid::new_v4().to_string()),
            name: None,
            account: None,
        };
        let id = uuid::Uuid::new_v4().to_string();
        let command = json!({"command":{"id":id,"service":"projects","action":"execute","operation":"projects.add","parameters":{"name":"private-project","path":project},"expires_at":(chrono::Utc::now()+chrono::Duration::minutes(1)).to_rfc3339()}});
        exchange(&m, &mut c, vec![(200, command.clone())]).unwrap();
        let start = std::time::Instant::now();
        while m.cloud.inner.lock().unwrap().active.is_some() {
            assert!(start.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(m.journal(&c).unwrap()[&id], "succeeded");
        let bytes = std::fs::read(m.cloud_output_path(&c, &id).unwrap()).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("private-project"));
        let requests = exchange(
            &m,
            &mut c,
            vec![(200, command.clone()), (200, json!({"ok":true}))],
        )
        .unwrap();
        assert_eq!(requests[1]["status"], "succeeded");
        assert_eq!(
            requests[1]["output"]["data"]["projects"][0]["name"],
            "private-project"
        );
        assert_eq!(m.snapshot().unwrap().projects.len(), 1);
        std::fs::remove_file(m.cloud_output_path(&c, &id).unwrap()).unwrap();
        let requests = exchange(
            &m,
            &mut c,
            vec![(200, command.clone()), (200, json!({"ok":true}))],
        )
        .unwrap();
        assert_eq!(requests[1]["status"], "uncertain");
        assert!(requests[1].get("output").is_none());
        assert_eq!(m.snapshot().unwrap().projects.len(), 1);
        let mut journal = m.journal(&c).unwrap();
        journal.insert(id.clone(), "running".into());
        m.save_journal(&c, &journal).unwrap();
        let requests =
            exchange(&m, &mut c, vec![(200, command), (200, json!({"ok":true}))]).unwrap();
        assert_eq!(requests[1]["status"], "uncertain");
        assert!(requests[1].get("output").is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pairing_codes_accept_copied_formatting_without_accepting_invalid_codes() {
        let code = "0123456789ABCDEF0123456789ABCDEF";
        assert_eq!(
            pairing_code(" 01234567-89abcdef 01234567-89abcdef\r\n").unwrap(),
            code
        );
        assert!(pairing_code("0123456789ABCDEF0123456789ABCDEG").is_err());
        assert!(pairing_code("short").is_err());
        assert!(pairing_code(&"a".repeat(257)).is_err());
        assert!(pairing_code("0123456789ABCDEF0123456789ABCDEFextra").is_err());
    }
    #[test]
    fn cloud_default_uses_forge_without_replacing_custom_servers() {
        assert_eq!(base_url("", false).unwrap(), DEFAULT_CLOUD_URL);
        assert_eq!(base_url("  ", false).unwrap(), DEFAULT_CLOUD_URL);
        assert_eq!(
            base_url(DEFAULT_CLOUD_URL, false).unwrap(),
            DEFAULT_CLOUD_URL
        );
        assert_eq!(
            base_url(" https://custom.example/ ", false).unwrap(),
            "https://custom.example"
        );
    }
    #[test]
    fn cloud_connection_distinguishes_pairing_from_live_subscription() {
        let mut runtime = Runtime::default();
        assert_eq!(
            connection_status(false, &runtime),
            CloudConnection::Disconnected
        );
        assert_eq!(
            connection_status(true, &runtime),
            CloudConnection::Connecting
        );
        runtime.connection = CloudConnection::Connected;
        assert_eq!(
            connection_status(true, &runtime),
            CloudConnection::Connected
        );
        assert_eq!(
            connection_status(false, &runtime),
            CloudConnection::Disconnected
        );
        runtime.connection = CloudConnection::Retrying;
        assert_eq!(connection_status(true, &runtime), CloudConnection::Retrying);
        runtime.disabled = true;
        assert_eq!(connection_status(true, &runtime), CloudConnection::Revoked);
        assert_eq!(
            serde_json::to_value(CloudConnection::Retrying).unwrap(),
            "retrying"
        );
    }
    #[test]
    fn service_report_exposes_capabilities_without_snapshot_secrets() {
        let report = service_report(vec![
            json!({"id":"node","actions":["install","repair"],"state":{"installed":true,"version":"24.0.0","directory":"private-path","token":"secret"}}),
            json!({"id":"mail","actions":["install","repair","start","stop","restart","open"],"state":{"running":true,"installed":true,"smtpPassword":"secret"}}),
            json!({"id":"unknown","actions":["start"],"state":{}}),
        ]);
        assert_eq!(report.len(), 2);
        assert_eq!(
            report[0],
            json!({"id":"node","actions":["install","repair"],"running":false,"installed":true,"version":"24.0.0"})
        );
        assert_eq!(
            report[1]["actions"],
            json!(["install", "repair", "start", "stop", "restart"])
        );
        assert!(!serde_json::to_string(&report).unwrap().contains("secret"));
    }
    #[test]
    fn cloud_urls_require_https_and_never_allow_embedded_credentials() {
        assert_eq!(
            base_url("https://cloud.example/", false).unwrap(),
            "https://cloud.example"
        );
        for u in [
            "http://cloud.example",
            "https://user:pass@cloud.example",
            "https://cloud.example/a",
            "https://cloud.example/?token=x",
            "file:///tmp",
        ] {
            assert!(base_url(u, false).is_err(), "{u}");
        }
        assert!(base_url("http://127.0.0.1:18080", true).is_ok());
        assert!(base_url("http://localhost:18080", true).is_err());
        assert!(base_url("http://192.168.1.1", true).is_err());
        assert!(base_url("http://127.0.0.1", false).is_err());
    }
    #[test]
    fn cloud_transport_does_not_follow_redirects() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let task = std::thread::spawn(move || {
            let request = server.recv().unwrap();
            request
                .respond(tiny_http::Response::empty(302).with_header(
                    tiny_http::Header::from_bytes("Location", "http://127.0.0.1:1/leak").unwrap(),
                ))
                .unwrap();
        });
        let c = Credentials {
            url,
            key: "a".repeat(64),
            code_hash: String::new(),
            device_id: None,
            name: None,
            account: None,
        };
        assert_eq!(
            post(&client().unwrap(), &c, "poll", &json!({})).unwrap().0,
            302
        );
        task.join().unwrap();
    }
}
