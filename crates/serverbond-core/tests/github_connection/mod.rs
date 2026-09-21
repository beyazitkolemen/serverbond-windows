use super::*;
use std::{collections::VecDeque, sync::Arc, thread};
use tiny_http::{Header, Response, Server, StatusCode};

struct Reply {
    status: u16,
    body: Value,
    headers: Vec<(&'static str, String)>,
    delay: Duration,
}
fn reply(body: Value) -> Reply {
    Reply {
        status: 200,
        body,
        headers: Vec::new(),
        delay: Duration::ZERO,
    }
}
struct RequestLog {
    url: String,
    body: Value,
    authorization: Option<String>,
}
struct Mock {
    server: Arc<Server>,
    requests: Arc<Mutex<Vec<RequestLog>>>,
    thread: Option<thread::JoinHandle<()>>,
    url: String,
}
impl Mock {
    fn new(replies: Vec<Reply>) -> Self {
        let server = Arc::new(Server::http("127.0.0.1:0").unwrap());
        let url = format!("http://{}", server.server_addr().to_ip().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let accept = server.clone();
        let log = requests.clone();
        let handle = thread::spawn(move || {
            let mut replies: VecDeque<_> = replies.into();
            for mut request in accept.incoming_requests() {
                let mut bytes = Vec::new();
                request.as_reader().read_to_end(&mut bytes).unwrap();
                log.lock().unwrap().push(RequestLog {
                    url: request.url().into(),
                    body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
                    authorization: request
                        .headers()
                        .iter()
                        .find(|h| h.field.equiv("Authorization"))
                        .map(|h| h.value.as_str().into()),
                });
                let response = replies.pop_front().unwrap_or_else(|| Reply {
                    status: 500,
                    ..reply(json!({}))
                });
                thread::sleep(response.delay);
                let mut result = Response::from_string(response.body.to_string())
                    .with_status_code(StatusCode(response.status));
                for (key, value) in response.headers {
                    result.add_header(Header::from_bytes(key, value).unwrap());
                }
                let _ = request.respond(result);
            }
        });
        Self {
            server,
            requests,
            thread: Some(handle),
            url,
        }
    }
}
impl Drop for Mock {
    fn drop(&mut self) {
        self.server.unblock();
        self.thread.take().unwrap().join().unwrap();
    }
}
fn manager(mock: &Mock) -> (tempfile::TempDir, Manager) {
    let home = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(home.path().to_path_buf()).unwrap();
    manager.github_runtime.http = GithubHttp {
        api: mock.url.clone(),
        oauth: mock.url.clone(),
    };
    manager
        .save_github_client_id("testclient1234567890")
        .unwrap();
    (home, manager)
}
fn device() -> Value {
    json!({"device_code":"private-device-code","user_code":"ABCD-EFGH","verification_uri":VERIFY_URL,"interval":5,"expires_in":900})
}
fn allow_poll(manager: &Manager) {
    manager
        .github_runtime
        .pending
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .next_poll = 0;
}

#[test]
fn git_https_auth_uses_basic_tokens_only_for_the_github_host() {
    for (url, expected) in [
        ("https://github.com/acme/app", true),
        ("https://gitlab.com/acme/app", false),
        ("https://github.com.attacker.invalid/acme/app", false),
    ] {
        let mut cmd = crate::process::command("git");
        cmd.args(["config", "--get-urlmatch", "http.extraHeader", url]);
        cmd.env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        );
        apply_github_git_auth(&mut cmd, Some("test-oauth-token"));
        let output = cmd
            .output()
            .expect("git is required for Git integration tests");
        if expected {
            assert!(output.status.success());
            let header = String::from_utf8(output.stdout).unwrap();
            let encoded = header.trim().strip_prefix("AUTHORIZATION: basic ").unwrap();
            assert_eq!(
                base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .unwrap(),
                b"x-access-token:test-oauth-token"
            );
        } else {
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
        }
    }
}
#[cfg(windows)]
fn token_response() -> Value {
    json!({"access_token":format!("gho_{}","a".repeat(36)),"token_type":"bearer","scope":"repo"})
}

#[test]
fn device_flow_throttles_slowdown_and_keeps_secrets_in_the_backend() {
    let mock = Mock::new(vec![
        reply(device()),
        reply(json!({"error":"authorization_pending"})),
        reply(json!({"error":"slow_down","interval":12})),
        reply(json!({"error":"access_denied"})),
    ]);
    let (_home, manager) = manager(&mock);
    let flow = manager.github_auth_start().unwrap();
    let public = serde_json::to_string(&flow).unwrap();
    assert!(!public.contains("private-device-code"));
    assert_eq!(flow.interval, 5);
    assert_eq!(
        manager.github_auth_poll(&flow.flow_id).unwrap().status,
        "pending"
    );
    assert_eq!(mock.requests.lock().unwrap().len(), 1);
    allow_poll(&manager);
    assert_eq!(
        manager.github_auth_poll(&flow.flow_id).unwrap().retry_after,
        5
    );
    allow_poll(&manager);
    assert_eq!(
        manager.github_auth_poll(&flow.flow_id).unwrap().retry_after,
        12
    );
    let wait = manager.github_auth_poll(&flow.flow_id).unwrap().retry_after;
    assert!((1..=12).contains(&wait));
    allow_poll(&manager);
    assert_eq!(
        manager.github_auth_poll(&flow.flow_id).unwrap().status,
        "denied"
    );
    assert!(!manager.github_has_token());
    let requests = mock.requests.lock().unwrap();
    assert_eq!(requests[0].url, "/login/device/code");
    assert_eq!(requests[0].body["scope"], "repo");
    assert_eq!(
        requests[1].body["grant_type"],
        "urn:ietf:params:oauth:grant-type:device_code"
    );
    assert!(requests
        .iter()
        .all(|r| r.body.get("client_secret").is_none()));
}

#[test]
fn cancellation_expiry_and_replaced_flows_cannot_complete_old_logins() {
    let mock = Mock::new(vec![reply(device()), reply(device()), reply(device())]);
    let (_home, manager) = manager(&mock);
    let old = manager.github_auth_start().unwrap();
    let current = manager.github_auth_start().unwrap();
    manager.github_auth_cancel(&old.flow_id).unwrap();
    assert_eq!(
        manager.github_auth_poll(&old.flow_id).unwrap().status,
        "cancelled"
    );
    assert_eq!(
        manager.github_auth_poll(&current.flow_id).unwrap().status,
        "pending"
    );
    manager.github_auth_cancel(&current.flow_id).unwrap();
    assert_eq!(
        manager.github_auth_poll(&current.flow_id).unwrap().status,
        "cancelled"
    );
    let expired = manager.github_auth_start().unwrap();
    manager
        .github_runtime
        .pending
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .public
        .expires_at = now() - 1;
    assert_eq!(
        manager.github_auth_poll(&expired.flow_id).unwrap().status,
        "expired"
    );
    assert_eq!(mock.requests.lock().unwrap().len(), 3);
}

#[test]
fn oauth_configuration_errors_and_verification_redirects_are_rejected() {
    let mut malicious = device();
    malicious["verification_uri"] = json!("https://attacker.invalid");
    let mock = Mock::new(vec![
        reply(json!({"error":"device_flow_disabled"})),
        reply(malicious),
    ]);
    let (_home, manager) = manager(&mock);
    assert!(manager.save_github_client_id("not a valid id").is_err());
    assert!(manager
        .github_auth_start()
        .err()
        .unwrap()
        .to_string()
        .contains("Device Flow"));
    assert!(manager
        .github_auth_start()
        .err()
        .unwrap()
        .to_string()
        .contains("adres"));
    manager.save_github_client_id("").unwrap();
    assert!(manager.github_auth_start().is_err());
    assert_eq!(mock.requests.lock().unwrap().len(), 2);
}

#[test]
fn public_branches_preserve_slashes_protection_and_pagination_without_auth() {
    let mock = Mock::new(vec![
        reply(json!({"default_branch":"develop"})),
        Reply {
            headers: vec![("Link", "<https://api.github.com/next>; rel=\"next\"".into())],
            ..reply(
                json!([{"name":"release/v2","protected":true},{"name":"develop","protected":false}]),
            )
        },
    ]);
    let (_home, manager) = manager(&mock);
    let page = manager
        .github_branches("https://github.com/acme/app.git", 1)
        .unwrap();
    assert_eq!(page.default_branch, "develop");
    assert_eq!(page.branches[0].name, "release/v2");
    assert!(page.branches[0].protected);
    assert_eq!(page.next_page, Some(2));
    let requests = mock.requests.lock().unwrap();
    assert_eq!(requests[0].url, "/repos/acme/app");
    assert!(requests[1].url.contains("page=1"));
    assert!(requests.iter().all(|r| r.authorization.is_none()));
    assert!(manager.github_branches("acme/app?token=bad", 1).is_err());
    assert!(manager.github_branches("acme/app", 0).is_err());
}

#[test]
fn github_http_reports_rate_limits_sso_and_private_repository_errors_without_echoing_body() {
    let mock = Mock::new(vec![
        Reply {
            status: 403,
            headers: vec![("X-RateLimit-Remaining", "0".into())],
            ..reply(json!({"message":"secret-must-not-leak"}))
        },
        Reply {
            status: 403,
            ..reply(json!({}))
        },
        Reply {
            status: 404,
            ..reply(json!({}))
        },
    ]);
    let (_home, manager) = manager(&mock);
    let error = manager
        .github_branches("acme/app", 1)
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("sınır"));
    assert!(!error.contains("secret"));
    assert!(manager
        .github_branches("acme/app", 1)
        .err()
        .unwrap()
        .to_string()
        .contains("SSO"));
    assert!(manager
        .github_branches("acme/app", 1)
        .err()
        .unwrap()
        .to_string()
        .contains("bulunamadı"));
}

#[cfg(windows)]
#[test]
fn oauth_success_encrypts_credentials_and_lists_private_organization_repositories() {
    let repo = json!({"full_name":"acme/private-app","owner":{"login":"acme"},"private":true,"archived":false,"fork":true,"description":"app","default_branch":"develop","language":"PHP","updated_at":"2026-09-21T10:00:00Z","size":123});
    let mock = Mock::new(vec![
        reply(device()),
        reply(token_response()),
        reply(json!({"login":"octocat"})),
        Reply {
            headers: vec![("Link", "<https://api.github.com/next>; rel=\"next\"".into())],
            ..reply(json!([repo]))
        },
    ]);
    let (home, manager) = manager(&mock);
    let flow = manager.github_auth_start().unwrap();
    allow_poll(&manager);
    let status = manager.github_auth_poll(&flow.flow_id).unwrap();
    assert_eq!(status.status, "connected");
    assert_eq!(status.login.as_deref(), Some("octocat"));
    assert_eq!(manager.github_state().auth_method.as_deref(), Some("oauth"));
    assert!(!serde_json::to_string(&manager.github_state())
        .unwrap()
        .contains("gho_"));
    let disk = std::fs::read(home.path().join("config/github-connection.dpapi")).unwrap();
    assert!(!String::from_utf8_lossy(&disk).contains("gho_"));
    let repos = manager.github_repositories(1).unwrap();
    assert_eq!(repos.repositories[0].full_name, "acme/private-app");
    assert!(repos.repositories[0].private);
    assert_eq!(repos.next_page, Some(2));
    let request = mock.requests.lock().unwrap();
    assert!(request[3].url.starts_with("/user/repos?"));
    assert!(request[3].url.contains("organization_member"));
    assert!(request[3]
        .authorization
        .as_deref()
        .unwrap()
        .starts_with("Bearer gho_"));
    drop(request);
    manager.clear_github_token().unwrap();
    assert!(!manager.github_has_token());
}

#[cfg(windows)]
#[test]
fn refresh_rotates_tokens_without_a_client_secret_and_preserves_legacy_pat_support() {
    let mut refreshed = token_response();
    refreshed["refresh_token"] = json!("new-refresh");
    refreshed["expires_in"] = json!(28800);
    refreshed["refresh_token_expires_in"] = json!(100000);
    let mock = Mock::new(vec![reply(refreshed)]);
    let (_home, manager) = manager(&mock);
    secrets::save(
        &manager.github_token_path(),
        &format!("ghp_{}", "b".repeat(36)),
    )
    .unwrap();
    assert!(manager.github_access_token().unwrap().starts_with("ghp_"));
    manager
        .github_write_connection(&Connection {
            login: "octocat".into(),
            method: "oauth".into(),
            access_token: "old-access".into(),
            client_id: Some("testclient1234567890".into()),
            refresh_token: Some("old-refresh".into()),
            expires_at: Some(now() - 1),
            refresh_expires_at: Some(now() + 10000),
        })
        .unwrap();
    assert!(manager.github_access_token().unwrap().starts_with("gho_"));
    assert_eq!(
        manager
            .github_connection()
            .unwrap()
            .unwrap()
            .refresh_token
            .as_deref(),
        Some("new-refresh")
    );
    let requests = mock.requests.lock().unwrap();
    assert_eq!(requests[0].body["grant_type"], "refresh_token");
    assert_eq!(requests[0].body["refresh_token"], "old-refresh");
    assert!(requests[0].body.get("client_secret").is_none());
}

#[cfg(windows)]
#[test]
fn identity_retry_reuses_exchanged_token_and_preserves_existing_connection_until_success() {
    let mock = Mock::new(vec![
        reply(device()),
        reply(token_response()),
        Reply {
            status: 500,
            ..reply(json!({}))
        },
        reply(json!({"login":"new-user"})),
    ]);
    let (_home, manager) = manager(&mock);
    manager
        .github_store_manual_token(&format!("ghp_{}", "b".repeat(36)), "old-user")
        .unwrap();
    let flow = manager.github_auth_start().unwrap();
    allow_poll(&manager);
    assert!(manager.github_auth_poll(&flow.flow_id).is_err());
    assert_eq!(manager.github_state().login.as_deref(), Some("old-user"));
    allow_poll(&manager);
    assert_eq!(
        manager.github_auth_poll(&flow.flow_id).unwrap().status,
        "connected"
    );
    assert_eq!(manager.github_state().login.as_deref(), Some("new-user"));
    let requests = mock.requests.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|r| r.url == "/login/oauth/access_token")
            .count(),
        1
    );
    assert_eq!(requests.iter().filter(|r| r.url == "/user").count(), 2);
}

#[cfg(windows)]
#[test]
fn cancelling_during_token_exchange_never_saves_the_inflight_token() {
    let mock = Mock::new(vec![
        reply(device()),
        Reply {
            delay: Duration::from_millis(200),
            ..reply(token_response())
        },
    ]);
    let (_home, manager) = manager(&mock);
    let manager = Arc::new(manager);
    let flow = manager.github_auth_start().unwrap();
    allow_poll(&manager);
    let cloned = manager.clone();
    let id = flow.flow_id.clone();
    let handle = thread::spawn(move || cloned.github_auth_poll(&id).unwrap());
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while mock.requests.lock().unwrap().len() < 2 {
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    manager.github_auth_cancel(&flow.flow_id).unwrap();
    assert_eq!(handle.join().unwrap().status, "cancelled");
    assert!(!manager.github_has_token());
}
