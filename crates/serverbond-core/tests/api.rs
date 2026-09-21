//! The local management API end to end: token handling, authorisation, the
//! route table, project lookups by name, and closing the listener again.

use serde_json::Value;
use serverbond_core::{api, model::Settings, Manager};
use std::{net::TcpListener, sync::Arc, time::Duration};

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn settings_with_api(port: u16) -> Settings {
    let mut settings = Settings {
        web_port: free_port(),
        mysql_port: free_port(),
        php_port: free_port(),
        ..Default::default()
    };
    settings.web.https_port = free_port();
    settings.mail.smtp_port = free_port();
    settings.mail.web_port = free_port();
    settings.postgres.port = free_port();
    settings.redis.port = free_port();
    settings.api.enabled = true;
    settings.api.port = port;
    settings
}

struct Api {
    _home: tempfile::TempDir,
    manager: Arc<Manager>,
    port: u16,
    token: String,
    client: reqwest::blocking::Client,
}

fn start() -> Api {
    let home = tempfile::tempdir().unwrap();
    let manager = Arc::new(Manager::new(home.path().to_path_buf()).unwrap());
    let port = free_port();
    manager.save_settings(settings_with_api(port)).unwrap();
    manager.ensure_api().unwrap();
    assert!(api::wait_ready(port, Duration::from_secs(5)));
    let token = manager.create_api_token().unwrap();
    Api {
        _home: home,
        manager,
        port,
        token,
        client: reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap(),
    }
}

impl Api {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}/api/v1{path}", self.port)
    }

    fn get(&self, path: &str) -> (u16, Value) {
        let response = self
            .client
            .get(self.url(path))
            .bearer_auth(&self.token)
            .send()
            .unwrap();
        (response.status().as_u16(), response.json().unwrap())
    }

    fn send(&self, method: reqwest::Method, path: &str, body: Value) -> (u16, Value) {
        let response = self
            .client
            .request(method, self.url(path))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .unwrap();
        (response.status().as_u16(), response.json().unwrap())
    }
}

#[test]
fn health_is_public_but_everything_else_needs_the_token() {
    let api = start();
    let health: Value = api
        .client
        .get(api.url("/health"))
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(health["ok"], true);
    assert_eq!(health["data"]["name"], "ServerBond");
    assert_eq!(health["data"]["api"], "v1");

    let anonymous = api.client.get(api.url("/status")).send().unwrap();
    assert_eq!(anonymous.status().as_u16(), 401);
    let wrong = api
        .client
        .get(api.url("/status"))
        .bearer_auth("sb_wrong")
        .send()
        .unwrap();
    assert_eq!(wrong.status().as_u16(), 401);

    let (status, body) = api.get("/status");
    assert_eq!(status, 200);
    assert_eq!(body["ok"], true);
    assert_eq!(body["data"]["settings"]["api"]["enabled"], true);
    assert!(body["data"]["projects"].as_array().unwrap().is_empty());
    assert!(api.manager.api_status().listening);
    assert!(api.manager.api_status().token_saved);
}

#[test]
fn route_index_unknown_paths_and_bad_bodies_are_reported_as_json() {
    let api = start();
    let (status, index) = api.get("");
    assert_eq!(status, 200);
    let paths: Vec<&str> = index["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"/api/v1/status"));
    assert!(paths.contains(&"/api/v1/projects/{id}/deploy"));

    let (status, body) = api.get("/nothing-here");
    assert_eq!(status, 404);
    assert_eq!(body["ok"], false);

    let (status, body) = api.send(reqwest::Method::POST, "/projects", Value::Null);
    assert_eq!(status, 400);
    assert!(body["error"].as_str().unwrap().contains("JSON"));

    let (status, body) = api.send(
        reqwest::Method::POST,
        "/projects",
        serde_json::json!({ "name": "BAD NAME", "path": "/nowhere" }),
    );
    assert_eq!(status, 400);
    assert!(
        body["error"].as_str().unwrap().contains("Proje adı"),
        "{body}"
    );

    let (status, _) = api.send(reqwest::Method::POST, "/services/redis/open", Value::Null);
    assert_eq!(status, 400);
}

#[test]
fn projects_are_addressable_by_name_and_settings_can_close_the_listener() {
    let api = start();
    let folder = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(folder.path().join("public")).unwrap();
    std::fs::write(folder.path().join("public/index.php"), "<?php\n").unwrap();
    let (status, created) = api.send(
        reqwest::Method::POST,
        "/projects",
        serde_json::json!({ "name": "magaza", "path": folder.path() }),
    );
    assert_eq!(status, 200, "{created}");
    let id = created["data"]["id"].as_str().unwrap().to_string();

    let (status, by_name) = api.get("/projects/magaza");
    assert_eq!(status, 200);
    assert_eq!(by_name["data"]["id"], id);
    let (status, release) = api.get("/projects/magaza/release");
    assert_eq!(status, 200);
    assert_eq!(release["data"]["branch"], "");
    let (status, jobs) = api.get(&format!("/projects/{id}/jobs"));
    assert_eq!(status, 200);
    assert_eq!(jobs["data"]["schedule"]["enabled"], false);
    let (status, git) = api.get("/projects/magaza/git");
    assert_eq!(status, 200);
    assert_eq!(git["data"]["present"], false);

    let (status, _) = api.send(reqwest::Method::DELETE, "/projects/magaza", Value::Null);
    assert_eq!(status, 200);
    let (status, _) = api.get("/projects/magaza");
    assert_eq!(status, 400);

    // Disabling the API through the API itself answers, then closes the port.
    let (_, current) = api.get("/settings");
    let mut settings = current["data"].clone();
    settings["api"]["enabled"] = Value::Bool(false);
    let (status, _) = api.send(reqwest::Method::PUT, "/settings", settings);
    assert_eq!(status, 200);
    assert!(!api.manager.api_status().listening);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline
        && std::net::TcpStream::connect(("127.0.0.1", api.port)).is_ok()
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(std::net::TcpStream::connect(("127.0.0.1", api.port)).is_err());
}

#[test]
fn a_new_token_invalidates_the_old_one_and_forgetting_locks_the_api() {
    let api = start();
    let old = api.token.clone();
    let fresh = api.manager.create_api_token().unwrap();
    assert_ne!(old, fresh);
    let stale = api
        .client
        .get(api.url("/status"))
        .bearer_auth(&old)
        .send()
        .unwrap();
    assert_eq!(stale.status().as_u16(), 401);
    let good = api
        .client
        .get(api.url("/status"))
        .bearer_auth(&fresh)
        .send()
        .unwrap();
    assert_eq!(good.status().as_u16(), 200);
    api.manager.clear_api_token().unwrap();
    let locked = api
        .client
        .get(api.url("/status"))
        .bearer_auth(&fresh)
        .send()
        .unwrap();
    assert_eq!(locked.status().as_u16(), 401);
    assert!(!api.manager.api_status().token_saved);
}

#[test]
fn every_documented_operation_requires_authentication_except_health() {
    let api = start();
    let (status, spec) = api.get("/openapi.json");
    assert_eq!(status, 200);
    assert_eq!(spec["openapi"], "3.1.0");
    assert_eq!(spec["security"][0]["bearerAuth"], serde_json::json!([]));
    let mut operation_ids = std::collections::HashSet::new();
    for (path, methods) in spec["paths"].as_object().unwrap() {
        assert!(!path.contains('|'), "Unexpanded path: {path}");
        for (method, operation) in methods.as_object().unwrap() {
            assert!(operation_ids.insert(operation["operationId"].as_str().unwrap()));
            if path == "/health" {
                assert_eq!(operation["security"], serde_json::json!([]));
                continue;
            }
            assert!(operation.get("security").is_none());
            let response = api
                .client
                .request(
                    reqwest::Method::from_bytes(method.to_uppercase().as_bytes()).unwrap(),
                    api.url(path),
                )
                .send()
                .unwrap();
            assert_eq!(response.status().as_u16(), 401, "{method} {path}");
            assert_eq!(response.headers()["cache-control"], "no-store");
        }
    }
    for path in [
        "/desktop/appearance",
        "/updates/install",
        "/permissions/ensure",
        "/projects/{id}/php/repair",
        "/api/token",
    ] {
        assert!(spec["paths"].get(path).is_some(), "{path}");
    }
    assert_eq!(
        spec["paths"]["/updates/install"]["post"]["requestBody"]["content"]["application/json"]
            ["schema"]["properties"]["confirm"]["const"],
        true
    );
}

#[test]
fn settings_validation_has_no_side_effects_and_rejects_bad_input() {
    let api = start();
    let (_, original) = api.get("/settings");
    let mut proposed = original["data"].clone();
    proposed["api"]["enabled"] = Value::Bool(false);
    let (status, valid) = api.send(
        reqwest::Method::POST,
        "/settings/validate",
        proposed.clone(),
    );
    assert_eq!(status, 200, "{valid}");
    assert_eq!(valid["data"], proposed);
    assert_eq!(api.get("/settings").1, original);
    proposed["api"]["port"] = serde_json::json!(0);
    proposed["api"]["enabled"] = Value::Bool(true);
    assert_eq!(
        api.send(reqwest::Method::POST, "/settings/validate", proposed)
            .0,
        400
    );
    assert_eq!(api.get("/settings").1, original);
    let malformed = api
        .client
        .post(api.url("/settings/validate"))
        .bearer_auth(&api.token)
        .body("{")
        .send()
        .unwrap();
    assert_eq!(malformed.status().as_u16(), 400);
    let oversized = api
        .client
        .post(api.url("/settings/validate"))
        .bearer_auth(&api.token)
        .body(vec![b' '; 1024 * 1024 + 1])
        .send()
        .unwrap();
    assert_eq!(oversized.status().as_u16(), 413);
}

#[test]
fn token_can_be_rotated_and_revoked_over_http() {
    let mut api = start();
    let (status, rotated) = api.send(reqwest::Method::POST, "/api/token", Value::Null);
    assert_eq!(status, 200);
    let new_token = rotated["data"]["token"].as_str().unwrap().to_owned();
    assert_ne!(api.token, new_token);
    assert_eq!(api.get("/api").0, 401);
    api.token = new_token;
    assert_eq!(api.get("/api").1["data"]["tokenSaved"], true);
    assert_eq!(
        api.send(reqwest::Method::DELETE, "/api/token", Value::Null)
            .0,
        200
    );
    assert_eq!(api.get("/api").0, 401);
    assert!(!api.manager.api_status().token_saved);
}

#[test]
fn desktop_capabilities_distinguish_cli_and_wrong_version_paths() {
    let api = start();
    assert_eq!(api.get("/capabilities").1["data"]["desktop"], false);
    for path in [
        "/desktop",
        "/desktop/appearance",
        "/updates",
        "/updates/status",
    ] {
        assert_eq!(api.get(path).0, 501, "{path}");
    }
    for path in ["/desktop/show", "/desktop/exit", "/updates/install"] {
        assert_eq!(
            api.send(reqwest::Method::POST, path, Value::Null).0,
            501,
            "{path}"
        );
    }
    let response = api
        .client
        .get(format!("http://127.0.0.1:{}/api/v10/status", api.port))
        .bearer_auth(&api.token)
        .send()
        .unwrap();
    assert_eq!(response.status().as_u16(), 404);
}

struct EchoDesktop;
impl api::DesktopApi for EchoDesktop {
    fn call(&self, operation: &str, body: Value) -> anyhow::Result<api::DesktopReply> {
        Ok(api::DesktopReply::immediate(
            serde_json::json!({"operation":operation,"body":body}),
        ))
    }
}

#[test]
fn desktop_routes_forward_to_the_host_without_changing_payloads() {
    let api = start();
    api.manager.attach_desktop_api(Arc::new(EchoDesktop));
    assert_eq!(api.get("/capabilities").1["data"]["desktop"], true);
    for (path, operation) in [
        ("/desktop", "status"),
        ("/desktop/appearance", "appearance-get"),
        ("/updates", "update-check"),
        ("/updates/status", "update-status"),
    ] {
        assert_eq!(api.get(path).1["data"]["operation"], operation);
    }
    for (method, path, operation, body) in [
        (
            reqwest::Method::PUT,
            "/desktop",
            "save",
            serde_json::json!({"preferences":{"closeToTray":true,"startMinimized":false},"autostart":false}),
        ),
        (
            reqwest::Method::PUT,
            "/desktop/appearance",
            "appearance-save",
            serde_json::json!({"theme":"dark"}),
        ),
        (
            reqwest::Method::POST,
            "/desktop/navigate",
            "navigate",
            serde_json::json!({"page":"updates"}),
        ),
        (
            reqwest::Method::POST,
            "/updates/install",
            "update-install",
            serde_json::json!({"confirm":true,"version":"1.2.0"}),
        ),
    ] {
        let (status, result) = api.send(method, path, body.clone());
        assert_eq!(status, 200);
        assert_eq!(result["data"]["operation"], operation);
        assert_eq!(result["data"]["body"], body);
    }
    for action in ["show", "hide", "menu", "exit", "restart"] {
        let (_, result) = api.send(
            reqwest::Method::POST,
            &format!("/desktop/{action}"),
            Value::Null,
        );
        assert_eq!(result["data"]["operation"], action);
    }
}

#[test]
fn deferred_desktop_work_does_not_hold_the_http_response_open() {
    use std::sync::{mpsc, Mutex};
    struct DeferredHost {
        release: Arc<Mutex<mpsc::Receiver<()>>>,
        finished: mpsc::Sender<bool>,
    }
    impl api::DesktopApi for DeferredHost {
        fn call(&self, _: &str, _: Value) -> anyhow::Result<api::DesktopReply> {
            let release = self.release.clone();
            let finished = self.finished.clone();
            Ok(api::DesktopReply {
                data: serde_json::json!({"accepted":true}),
                after_response: Some(Box::new(move || {
                    let released = release
                        .lock()
                        .unwrap()
                        .recv_timeout(Duration::from_secs(10))
                        .is_ok();
                    finished.send(released).unwrap();
                    Ok(())
                })),
            })
        }
    }
    let api = start();
    let (release, receive) = mpsc::channel();
    let (finished, done) = mpsc::channel();
    api.manager.attach_desktop_api(Arc::new(DeferredHost {
        release: Arc::new(Mutex::new(receive)),
        finished,
    }));
    let (status, response) = api.send(reqwest::Method::POST, "/desktop/exit", Value::Null);
    assert_eq!(status, 202);
    assert_eq!(response["data"]["accepted"], true);
    release.send(()).unwrap();
    assert!(done.recv_timeout(Duration::from_secs(5)).unwrap());
}
