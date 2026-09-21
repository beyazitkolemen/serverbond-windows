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
