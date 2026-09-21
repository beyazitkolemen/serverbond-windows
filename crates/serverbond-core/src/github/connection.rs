//! GitHub Device Flow and repository discovery. Credentials never cross IPC;
//! OAuth polling is bound to an opaque local flow id and throttled server-side.
use super::*;
use serde_json::{json, Value};
use std::{
    io::Read,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const VERIFY_URL: &str = "https://github.com/login/device";
const RESPONSE_LIMIT: u64 = 2 * 1024 * 1024;

pub(crate) struct GithubRuntime {
    pub(super) http: GithubHttp,
    pending: Mutex<Option<Pending>>,
    credentials: Mutex<()>,
}

impl Default for GithubRuntime {
    fn default() -> Self {
        Self {
            http: GithubHttp {
                api: "https://api.github.com".into(),
                oauth: "https://github.com".into(),
            },
            pending: Mutex::new(None),
            credentials: Mutex::new(()),
        }
    }
}

pub(super) struct GithubHttp {
    api: String,
    oauth: String,
}

impl GithubHttp {
    fn client(&self) -> Result<reqwest::blocking::Client> {
        reqwest::blocking::Client::builder()
            .user_agent(format!("ServerBond/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("GitHub istemcisi oluşturulamadı.")
    }

    fn read(response: reqwest::blocking::Response, oauth: bool) -> Result<(Value, bool)> {
        let status = response.status();
        let next = response
            .headers()
            .get("link")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.split(',').any(|part| part.contains("rel=\"next\"")));
        if status.as_u16() == 429
            || response
                .headers()
                .get("x-ratelimit-remaining")
                .is_some_and(|v| v == "0")
                && !status.is_success()
        {
            bail!("GitHub istek sınırına ulaşıldı. Bir süre sonra yeniden deneyin.");
        }
        if !status.is_success() && !(oauth && status.as_u16() == 400) {
            match status.as_u16() {
                401 => bail!("GitHub oturumu geçersiz veya süresi dolmuş. Yeniden bağlanın."),
                403 => bail!("GitHub erişimi reddedildi. Depo yetkilerini ve organizasyon SSO onayını kontrol edin."),
                404 => bail!("GitHub deposu bulunamadı veya bu hesaba erişim verilmedi."),
                _ => bail!("GitHub isteği tamamlanamadı (HTTP {}).", status.as_u16()),
            }
        }
        let mut bytes = Vec::new();
        response
            .take(RESPONSE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .context("GitHub yanıtı alınamadı.")?;
        if bytes.len() as u64 > RESPONSE_LIMIT {
            bail!("GitHub yanıtı boyut sınırını aşıyor.");
        }
        Ok((
            serde_json::from_slice(&bytes).context("GitHub yanıtı okunamadı.")?,
            next,
        ))
    }

    fn get(
        &self,
        path: &str,
        query: &[(&str, String)],
        token: Option<&str>,
    ) -> Result<(Value, bool)> {
        let mut request = self
            .client()?
            .get(format!("{}{path}", self.api))
            .query(query)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        Self::read(
            request.send().context("GitHub API'ye bağlanılamadı.")?,
            false,
        )
    }

    fn post(&self, path: &str, body: Value) -> Result<Value> {
        let response = self
            .client()?
            .post(format!("{}{path}", self.oauth))
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .context("GitHub yetkilendirme sunucusuna bağlanılamadı.")?;
        Ok(Self::read(response, true)?.0)
    }

    pub(super) fn login(&self, token: &str) -> Result<String> {
        let value = self.get("/user", &[], Some(token))?.0;
        let login = value["login"]
            .as_str()
            .filter(|v| super::valid_github_name(v))
            .context("GitHub hesap yanıtı geçersiz.")?;
        Ok(login.into())
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Connection {
    pub(super) login: String,
    pub(super) method: String,
    access_token: String,
    client_id: Option<String>,
    refresh_token: Option<String>,
    pub(super) expires_at: Option<u64>,
    refresh_expires_at: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubAuthFlow {
    pub flow_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_at: u64,
    pub interval: u64,
}

struct Pending {
    public: GithubAuthFlow,
    device_code: String,
    client_id: String,
    next_poll: u64,
    resolved: Option<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubAuthPoll {
    pub status: String,
    pub retry_after: u64,
    pub login: Option<String>,
}

fn poll_state(status: &str, retry_after: u64, login: Option<String>) -> GithubAuthPoll {
    GithubAuthPoll {
        status: status.into(),
        retry_after,
        login,
    }
}

fn oauth_error(value: &Value) -> Result<()> {
    if let Some(error) = value["error"].as_str() {
        bail!(
            "{}",
            match error {
                "device_flow_disabled" =>
                    "GitHub OAuth App ayarlarında Device Flow etkinleştirilmeli.",
                "incorrect_client_credentials" => "GitHub OAuth Client ID geçersiz.",
                "bad_refresh_token" => "GitHub oturumunun yenileme süresi doldu. Yeniden bağlanın.",
                "access_denied" => "GitHub erişim isteği reddedildi.",
                _ => "GitHub yetkilendirmesi tamamlanamadı. Yeniden bağlanın.",
            }
        );
    }
    Ok(())
}

fn connection_from_token(value: &Value, client_id: &str, login: String) -> Result<Connection> {
    oauth_error(value)?;
    if value["token_type"]
        .as_str()
        .is_none_or(|v| !v.eq_ignore_ascii_case("bearer"))
    {
        bail!("GitHub jeton türü desteklenmiyor.");
    }
    let access_token = validate_github_token(
        value["access_token"]
            .as_str()
            .context("GitHub jetonu alınamadı.")?,
    )?;
    let expiry = value["expires_in"]
        .as_u64()
        .map(|s| now().saturating_add(s));
    Ok(Connection {
        login,
        method: "oauth".into(),
        access_token,
        client_id: Some(client_id.into()),
        refresh_token: value["refresh_token"].as_str().map(str::to_owned),
        expires_at: expiry,
        refresh_expires_at: value["refresh_token_expires_in"]
            .as_u64()
            .map(|s| now().saturating_add(s)),
    })
}

impl Manager {
    fn github_connection_path(&self) -> PathBuf {
        self.home.join("config/github-connection.dpapi")
    }

    pub(super) fn github_has_token(&self) -> bool {
        self.github_connection_path().is_file() || self.github_token_path().is_file()
    }

    pub(super) fn github_connection(&self) -> Result<Option<Connection>> {
        if !self.github_connection_path().exists() {
            return Ok(None);
        }
        let encoded = secrets::read(&self.github_connection_path())?;
        serde_json::from_str(&encoded)
            .map(Some)
            .context("GitHub bağlantısı okunamadı. Yeniden bağlanın.")
    }

    fn github_write_connection(&self, connection: &Connection) -> Result<()> {
        secrets::save(
            &self.github_connection_path(),
            &serde_json::to_string(connection)?,
        )
    }

    pub(super) fn github_store_manual_token(&self, token: &str, login: &str) -> Result<()> {
        let _credentials = self
            .github_runtime
            .credentials
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.github_write_connection(&Connection {
            login: login.into(),
            method: "token".into(),
            access_token: token.into(),
            client_id: None,
            refresh_token: None,
            expires_at: None,
            refresh_expires_at: None,
        })?;
        self.github_runtime
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        Ok(())
    }

    pub(super) fn github_forget_connection(&self) -> Result<()> {
        self.github_runtime
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let _credentials = self
            .github_runtime
            .credentials
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if self.github_connection_path().exists() {
            std::fs::remove_file(self.github_connection_path())
                .context("GitHub bağlantısı silinemedi.")?;
        }
        Ok(())
    }

    pub(super) fn github_access_token(&self) -> Result<String> {
        let _credentials = self
            .github_runtime
            .credentials
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some(mut connection) = self.github_connection()? else {
            return secrets::read(&self.github_token_path());
        };
        if connection
            .expires_at
            .is_some_and(|t| t <= now().saturating_add(60))
        {
            if connection.refresh_expires_at.is_some_and(|t| t <= now()) {
                bail!("GitHub oturumunun süresi doldu. Yeniden bağlanın.");
            }
            let refresh = connection
                .refresh_token
                .as_deref()
                .context("GitHub oturumunun süresi doldu. Yeniden bağlanın.")?;
            let client_id = connection
                .client_id
                .as_deref()
                .context("GitHub Client ID bulunamadı. Yeniden bağlanın.")?;
            let value = self.github_runtime.http.post(
                "/login/oauth/access_token",
                json!({"client_id":client_id,"grant_type":"refresh_token","refresh_token":refresh}),
            )?;
            connection = connection_from_token(&value, client_id, connection.login.clone())?;
            self.github_write_connection(&connection)?;
        }
        Ok(connection.access_token)
    }

    pub(super) fn github_client_id(&self) -> Result<String> {
        let path = self.home.join("config/github-oauth.json");
        if path.exists() {
            let value: Value = serde_json::from_slice(&storage::read_limited(&path, 4096)?)?;
            return Ok(value["clientId"]
                .as_str()
                .context("GitHub OAuth ayarı okunamadı.")?
                .into());
        }
        Ok(option_env!("SERVERBOND_GITHUB_CLIENT_ID")
            .unwrap_or("")
            .into())
    }

    pub fn save_github_client_id(&self, client_id: &str) -> Result<()> {
        let _guard = self.gate()?;
        let client_id = client_id.trim();
        if !client_id.is_empty()
            && (!(10..=100).contains(&client_id.len())
                || !client_id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_'))
        {
            bail!("Geçerli bir GitHub OAuth Client ID girin; Client Secret gerekli değildir.");
        }
        storage::atomic_write(
            &self.home.join("config/github-oauth.json"),
            serde_json::to_vec(&json!({"clientId":client_id}))?,
        )?;
        self.github_runtime
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        Ok(())
    }

    pub fn github_auth_start(&self) -> Result<GithubAuthFlow> {
        let _guard = self.gate()?;
        let client_id = self.github_client_id()?;
        if client_id.is_empty() {
            bail!("Önce GitHub OAuth App Client ID ayarını kaydedin.");
        }
        let value = self.github_runtime.http.post(
            "/login/device/code",
            json!({"client_id":client_id,"scope":"repo"}),
        )?;
        oauth_error(&value)?;
        let device_code = value["device_code"]
            .as_str()
            .filter(|v| !v.is_empty() && v.len() <= 512)
            .context("GitHub cihaz kodu alınamadı.")?;
        let user_code = value["user_code"]
            .as_str()
            .filter(|v| {
                v.len() <= 32
                    && !v.is_empty()
                    && v.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            })
            .context("GitHub doğrulama kodu alınamadı.")?;
        if value["verification_uri"] != VERIFY_URL {
            bail!("GitHub doğrulama adresi geçersiz.");
        }
        let expires = value["expires_in"]
            .as_u64()
            .filter(|n| *n > 0 && *n <= 3600)
            .context("GitHub kod süresi geçersiz.")?;
        let interval = value["interval"].as_u64().unwrap_or(5).clamp(5, 300);
        let public = GithubAuthFlow {
            flow_id: uuid::Uuid::new_v4().to_string(),
            user_code: user_code.into(),
            verification_uri: VERIFY_URL.into(),
            expires_at: now() + expires,
            interval,
        };
        *self
            .github_runtime
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(Pending {
            public: public.clone(),
            device_code: device_code.into(),
            client_id,
            next_poll: now() + interval,
            resolved: None,
        });
        Ok(public)
    }

    pub fn github_auth_cancel(&self, flow_id: &str) -> Result<()> {
        let mut slot = self
            .github_runtime
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if slot.as_ref().is_some_and(|p| p.public.flow_id == flow_id) {
            slot.take();
        }
        Ok(())
    }

    pub fn github_auth_open(&self) -> Result<()> {
        #[cfg(windows)]
        {
            crate::process::command("rundll32.exe")
                .args(["url.dll,FileProtocolHandler", VERIFY_URL])
                .spawn()
                .context("GitHub doğrulama sayfası açılamadı.")?;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            bail!("GitHub doğrulama sayfasını tarayıcınızda açın: {VERIFY_URL}")
        }
    }

    pub fn github_auth_poll(&self, flow_id: &str) -> Result<GithubAuthPoll> {
        let _guard = self.gate()?;
        let (client_id, device_code, cached) = {
            let mut slot = self
                .github_runtime
                .pending
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let Some(pending) = slot.as_mut().filter(|p| p.public.flow_id == flow_id) else {
                return Ok(poll_state("cancelled", 0, None));
            };
            if now() >= pending.public.expires_at {
                slot.take();
                return Ok(poll_state("expired", 0, None));
            }
            if now() < pending.next_poll {
                return Ok(poll_state(
                    "pending",
                    pending.next_poll.saturating_sub(now()),
                    None,
                ));
            }
            pending.next_poll = now() + pending.public.interval;
            (
                pending.client_id.clone(),
                pending.device_code.clone(),
                pending.resolved.clone(),
            )
        };
        let value = match cached { Some(value) => value, None => self.github_runtime.http.post("/login/oauth/access_token",json!({"client_id":client_id,"device_code":device_code,"grant_type":"urn:ietf:params:oauth:grant-type:device_code"}))? };
        let mut slot = self
            .github_runtime
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some(pending) = slot.as_mut().filter(|p| p.public.flow_id == flow_id) else {
            return Ok(poll_state("cancelled", 0, None));
        };
        match value["error"].as_str() {
            Some("authorization_pending") => {
                pending.next_poll = now() + pending.public.interval;
                return Ok(poll_state("pending", pending.public.interval, None));
            }
            Some("slow_down") => {
                pending.public.interval = value["interval"]
                    .as_u64()
                    .unwrap_or(0)
                    .max(pending.public.interval.saturating_add(5))
                    .min(3600);
                pending.next_poll = now() + pending.public.interval;
                return Ok(poll_state("pending", pending.public.interval, None));
            }
            Some("expired_token" | "token_expired") => {
                slot.take();
                return Ok(poll_state("expired", 0, None));
            }
            Some("access_denied") => {
                slot.take();
                return Ok(poll_state("denied", 0, None));
            }
            Some(_) => {
                slot.take();
                oauth_error(&value)?;
                unreachable!();
            }
            None => {}
        }
        pending.resolved = Some(value.clone());
        // Keep the flow cancellable while identity verification is on the wire.
        drop(slot);
        let token = validate_github_token(
            value["access_token"]
                .as_str()
                .context("GitHub jetonu alınamadı.")?,
        )?;
        let login = self.github_runtime.http.login(&token)?;
        let connection = connection_from_token(&value, &client_id, login.clone())?;
        let mut slot = self
            .github_runtime
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if !slot.as_ref().is_some_and(|p| p.public.flow_id == flow_id) {
            return Ok(poll_state("cancelled", 0, None));
        }
        let _credentials = self
            .github_runtime
            .credentials
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.github_write_connection(&connection)?;
        slot.take();
        self.log(format!("GitHub ile giriş tamamlandı: {login}."));
        Ok(poll_state("connected", 0, Some(login)))
    }
}

#[cfg(test)]
#[path = "../../tests/github_connection/mod.rs"]
mod tests;

#[derive(Deserialize)]
struct RawOwner {
    login: String,
}
#[derive(Deserialize)]
struct RawRepo {
    full_name: String,
    owner: RawOwner,
    private: bool,
    archived: bool,
    fork: bool,
    description: Option<String>,
    default_branch: String,
    language: Option<String>,
    updated_at: String,
    size: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubRepo {
    full_name: String,
    owner: String,
    private: bool,
    archived: bool,
    fork: bool,
    description: Option<String>,
    default_branch: String,
    language: Option<String>,
    updated_at: String,
    empty: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubRepoPage {
    repositories: Vec<GithubRepo>,
    next_page: Option<u32>,
}
#[derive(Serialize, Deserialize)]
pub struct GithubBranch {
    name: String,
    protected: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubBranchPage {
    branches: Vec<GithubBranch>,
    default_branch: String,
    next_page: Option<u32>,
}

fn page_number(page: u32) -> Result<()> {
    if !(1..=10000).contains(&page) {
        bail!("Sayfa 1–10000 arasında olmalı.");
    }
    Ok(())
}

impl Manager {
    pub fn github_repositories(&self, page: u32) -> Result<GithubRepoPage> {
        page_number(page)?;
        if !self.github_has_token() {
            bail!("Depoları listelemek için GitHub hesabınızı bağlayın.");
        }
        let token = self.github_access_token()?;
        let (value, next) = self.github_runtime.http.get(
            "/user/repos",
            &[
                ("visibility", "all".into()),
                (
                    "affiliation",
                    "owner,collaborator,organization_member".into(),
                ),
                ("sort", "updated".into()),
                ("direction", "desc".into()),
                ("per_page", "50".into()),
                ("page", page.to_string()),
            ],
            Some(&token),
        )?;
        let raw: Vec<RawRepo> =
            serde_json::from_value(value).context("GitHub depo listesi okunamadı.")?;
        let repositories = raw
            .into_iter()
            .map(|r| GithubRepo {
                full_name: r.full_name,
                owner: r.owner.login,
                private: r.private,
                archived: r.archived,
                fork: r.fork,
                description: r.description,
                default_branch: r.default_branch,
                language: r.language,
                updated_at: r.updated_at,
                empty: r.size == 0,
            })
            .collect();
        Ok(GithubRepoPage {
            repositories,
            next_page: next.then_some(page + 1),
        })
    }

    pub fn github_branches(&self, repository: &str, page: u32) -> Result<GithubBranchPage> {
        page_number(page)?;
        let repo = parse_github_repository(repository)?;
        let token = if self.github_has_token() {
            Some(self.github_access_token()?)
        } else {
            None
        };
        let path = format!("/repos/{}/{}", repo.owner, repo.name);
        let details = self
            .github_runtime
            .http
            .get(&path, &[], token.as_deref())?
            .0;
        let default_branch = details["default_branch"]
            .as_str()
            .context("GitHub varsayılan dalı okunamadı.")?
            .to_string();
        let (value, next) = self.github_runtime.http.get(
            &format!("{path}/branches"),
            &[("per_page", "100".into()), ("page", page.to_string())],
            token.as_deref(),
        )?;
        let branches = serde_json::from_value(value).context("GitHub dal listesi okunamadı.")?;
        Ok(GithubBranchPage {
            branches,
            default_branch,
            next_page: next.then_some(page + 1),
        })
    }
}
