use super::*;
use serde_json::json;
use std::collections::HashSet;

fn enabled() -> Api {
    let api = start();
    let mut settings = api.manager.snapshot().unwrap().settings.api;
    settings.mcp_enabled = true;
    api.manager.save_api_settings(settings).unwrap();
    api
}

fn request(api: &Api) -> reqwest::blocking::RequestBuilder {
    api.client
        .post(format!("http://127.0.0.1:{}/mcp", api.port))
        .bearer_auth(&api.token)
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2025-11-25")
}

fn with_header(api: &Api, name: &str, value: &str) -> reqwest::blocking::RequestBuilder {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
        value.parse().unwrap(),
    );
    request(api).headers(headers)
}

fn rpc(api: &Api, method: &str, params: Value) -> Value {
    let response = request(api)
        .json(&json!({"jsonrpc":"2.0","id":"test","method":method,"params":params}))
        .send()
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let message: Value = response.json().unwrap();
    assert_eq!(message["id"], "test");
    message
}

fn call(api: &Api, name: &str, arguments: Value) -> Value {
    rpc(
        api,
        "tools/call",
        json!({"name":name,"arguments":arguments}),
    )
}

#[test]
fn mcp_is_opt_in_and_requires_a_valid_token_on_every_request() {
    let mut api = start();
    assert!(!api.manager.api_status().mcp_enabled);
    let body = json!({"jsonrpc":"2.0","id":1,"method":"ping"});
    assert_eq!(request(&api).json(&body).send().unwrap().status(), 404);
    let mut settings = api.manager.snapshot().unwrap().settings.api;
    settings.mcp_enabled = true;
    api.manager.save_api_settings(settings.clone()).unwrap();
    assert_eq!(rpc(&api, "ping", json!({}))["result"], json!({}));
    assert_eq!(
        with_header(&api, "Authorization", "Bearer wrong")
            .json(&body)
            .send()
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        api.client
            .post(format!("http://127.0.0.1:{}/mcp", api.port))
            .json(&body)
            .send()
            .unwrap()
            .status(),
        401
    );
    let token = api.manager.create_api_token().unwrap();
    assert_eq!(request(&api).json(&body).send().unwrap().status(), 401);
    api.token = token;
    assert_eq!(rpc(&api, "ping", json!({}))["result"], json!({}));
    settings.mcp_enabled = false;
    api.manager.save_api_settings(settings.clone()).unwrap();
    assert_eq!(request(&api).json(&body).send().unwrap().status(), 404);
    settings.mcp_enabled = true;
    api.manager.save_api_settings(settings).unwrap();
    api.manager.clear_api_token().unwrap();
    assert_eq!(request(&api).json(&body).send().unwrap().status(), 401);
}

#[test]
fn mcp_initializes_negotiates_versions_and_acknowledges_notifications() {
    let api = enabled();
    let without_version = || {
        api.client
            .post(format!("http://127.0.0.1:{}/mcp", api.port))
            .bearer_auth(&api.token)
            .header("Accept", "application/json, text/event-stream")
    };
    let response = without_version().json(&json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})).send().unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        without_version()
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
            .send()
            .unwrap()
            .status(),
        400
    );
    for version in ["2025-03-26", "2025-06-18", "2025-11-25", "2099-01-01"] {
        let response = rpc(
            &api,
            "initialize",
            json!({"protocolVersion":version,"capabilities":{},"clientInfo":{"name":"test","version":"1"}}),
        );
        assert_eq!(
            response["result"]["protocolVersion"],
            if version == "2099-01-01" || version == "2025-03-26" {
                "2025-11-25"
            } else {
                version
            }
        );
        assert_eq!(response["result"]["serverInfo"]["name"], "serverbond");
        assert_eq!(response["result"]["capabilities"]["tools"], json!({}));
    }
    assert_eq!(rpc(&api, "initialize", json!({}))["error"]["code"], -32602);
    assert_eq!(rpc(&api, "unknown", json!({}))["error"]["code"], -32601);
    let response = request(&api)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .unwrap();
    assert_eq!(response.status(), 202);
    assert_eq!(response.text().unwrap(), "");
    let response = request(&api).json(&json!({"jsonrpc":"2.0","method":"tools/call","params":{"name":"serverbond_delete_api_token"}})).send().unwrap();
    assert_eq!(response.status(), 400);
    assert!(api.manager.api_status().token_saved);
}

#[test]
fn mcp_rejects_invalid_http_origins_hosts_versions_and_rpc_messages() {
    let api = enabled();
    let body = json!({"jsonrpc":"2.0","id":1,"method":"ping"});
    for (name, value, expected) in [
        ("Origin", "https://attacker.invalid", 403),
        ("Origin", "null", 403),
        ("Origin", "http://localhost:9999", 403),
        ("Host", "attacker.invalid", 403),
        ("MCP-Protocol-Version", "invalid", 400),
        ("MCP-Protocol-Version", "2025-03-26", 400),
        ("Accept", "text/html", 406),
        ("Content-Type", "text/plain", 415),
    ] {
        assert_eq!(
            with_header(&api, name, value)
                .json(&body)
                .send()
                .unwrap()
                .status()
                .as_u16(),
            expected,
            "{name}"
        );
    }
    assert_eq!(
        request(&api)
            .header("Origin", format!("http://127.0.0.1:{}", api.port))
            .json(&body)
            .send()
            .unwrap()
            .status(),
        200
    );
    for method in [reqwest::Method::GET, reqwest::Method::DELETE] {
        let response = api
            .client
            .request(method, format!("http://127.0.0.1:{}/mcp", api.port))
            .bearer_auth(&api.token)
            .send()
            .unwrap();
        assert_eq!(response.status(), 405);
        assert_eq!(response.headers()["allow"], "POST");
    }
    for body in [
        json!([]),
        json!({}),
        json!({"jsonrpc":"1.0","id":1,"method":"ping"}),
        json!({"jsonrpc":"2.0","id":null,"method":"ping"}),
        json!({"jsonrpc":"2.0","id":true,"method":"ping"}),
    ] {
        let response = request(&api).json(&body).send().unwrap();
        assert_eq!(response.status(), 400);
        assert_eq!(response.json::<Value>().unwrap()["error"]["code"], -32600);
    }
    let response = request(&api)
        .header("Content-Type", "application/json")
        .body("{")
        .send()
        .unwrap();
    assert_eq!(response.status(), 400);
    assert_eq!(response.json::<Value>().unwrap()["error"]["code"], -32700);
    assert_eq!(
        request(&api)
            .json(&json!({"padding":"x".repeat(1024*1024)}))
            .send()
            .unwrap()
            .status(),
        413
    );
}

#[test]
fn mcp_tools_cover_every_openapi_operation_with_valid_unique_schemas() {
    let api = enabled();
    let document = api::documentation()["document"].clone();
    let operations: usize = document["paths"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| v.as_object().unwrap().len())
        .sum();
    let response = rpc(&api, "tools/list", json!({}));
    let tools = response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), operations);
    let mut names = HashSet::new();
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        assert!(names.insert(name));
        assert!(name.len() <= 128 && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'));
        jsonschema::validator_for(&tool["inputSchema"]).unwrap();
    }
    assert_eq!(
        rpc(&api, "tools/list", json!({"cursor":"bogus"}))["error"]["code"],
        -32602
    );
    for name in [
        "serverbond_get_health",
        "serverbond_get_services",
        "serverbond_get_projects",
        "serverbond_get_capabilities",
        "serverbond_get_openapi_json",
    ] {
        let response = call(&api, name, json!({}));
        assert_eq!(response["result"]["isError"], false, "{name}: {response}");
        let structured: Value =
            serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(structured, response["result"]["structuredContent"]);
    }
}

#[test]
fn github_connection_settings_and_discovery_contract_work_through_http_and_mcp() {
    let api = enabled();
    let saved = call(
        &api,
        "serverbond_put_github_auth_settings",
        json!({"body":{"clientId":"testclient1234567890"}}),
    );
    assert_eq!(saved["result"]["isError"], false);
    let state = api.get("/github").1["data"].clone();
    assert_eq!(state["oauthClientId"], "testclient1234567890");
    assert_eq!(state["tokenSaved"], false);
    assert!(state.get("accessToken").is_none());
    assert!(state.get("refreshToken").is_none());
    let pending = api.send(
        reqwest::Method::POST,
        "/github/auth/poll",
        json!({"flowId":"unknown-flow"}),
    );
    assert_eq!(pending.0, 200);
    assert_eq!(pending.1["data"]["status"], "cancelled");
    assert_eq!(
        call(
            &api,
            "serverbond_post_github_auth_cancel",
            json!({"body":{"flowId":"unknown-flow"}})
        )["result"]["isError"],
        false
    );
    assert_eq!(
        api.send(
            reqwest::Method::PUT,
            "/github/auth/settings",
            json!({"clientId":"","clientSecret":"NEVER_ECHO"})
        )
        .0,
        400
    );
    assert_eq!(
        api.send(
            reqwest::Method::PUT,
            "/github/auth/settings",
            json!({"clientId":""})
        )
        .0,
        200
    );
    // Blank Client ID must fail locally; this test never contacts GitHub.
    assert_eq!(
        api.send(reqwest::Method::POST, "/github/auth/start", json!({}))
            .0,
        400
    );
    for path in [
        "/github/repositories?page=no",
        "/github/repositories?page=0",
        "/github/repositories?page=10001",
        "/github/branches?page=1",
        "/github/branches?repository=acme%2Fapp%3Fbad&page=1",
    ] {
        assert_eq!(api.get(path).0, 400, "{path}");
    }
    let tools = rpc(&api, "tools/list", json!({}));
    let branches = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "serverbond_get_github_branches")
        .unwrap();
    assert_eq!(
        branches["inputSchema"]["properties"]["page"]["type"],
        "integer"
    );
    assert_eq!(branches["inputSchema"]["properties"]["page"]["minimum"], 1);
    for page in [json!(0), json!(10001), json!("1")] {
        assert_eq!(
            call(
                &api,
                "serverbond_get_github_repositories",
                json!({"page":page})
            )["error"]["code"],
            -32602
        );
    }
    let repos = call(
        &api,
        "serverbond_get_github_repositories",
        json!({"page":2}),
    );
    assert_eq!(repos["result"]["isError"], true);
    assert!(repos.to_string().contains("bağlayın"));
}

#[test]
fn mcp_validates_arguments_and_distinguishes_execution_errors() {
    let api = enabled();
    for (name, args) in [
        ("missing", json!({})),
        ("serverbond_get_services_by_id", json!({})),
        ("serverbond_get_services_by_id", json!({"id":22})),
        ("serverbond_get_services_by_id", json!({"id":"php/stop"})),
        ("serverbond_get_services_by_id", json!({"id":".."})),
        ("serverbond_get_services_by_id", json!({"id":"php%2Fstop"})),
        (
            "serverbond_get_services_by_id",
            json!({"id":"php","extra":true}),
        ),
        (
            "serverbond_put_api",
            json!({"body":{"enabled":true,"port":0}}),
        ),
        (
            "serverbond_post_updates_install",
            json!({"body":{"version":"1.2.0","confirm":false}}),
        ),
        (
            "serverbond_post_mysql_password",
            json!({"body":{"password":false,"secret":"NEVER_ECHO"}}),
        ),
    ] {
        let response = call(&api, name, args);
        assert_eq!(response["error"]["code"], -32602, "{response}");
        assert!(!response.to_string().contains("NEVER_ECHO"));
    }
    let response = call(
        &api,
        "serverbond_get_services_by_id",
        json!({"id":"missing"}),
    );
    assert_eq!(response["result"]["isError"], true);
    assert_eq!(
        response["result"]["structuredContent"]["data"]["status"],
        404
    );
    assert_eq!(
        call(&api, "serverbond_get_desktop", json!({}))["result"]["structuredContent"]["data"]
            ["status"],
        501
    );
}

#[test]
fn mcp_manages_projects_env_jobs_and_services_through_the_shared_api() {
    let api = enabled();
    let folder = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(folder.path().join("public")).unwrap();
    std::fs::write(folder.path().join("public/index.php"), "<?php\n").unwrap();
    std::fs::write(folder.path().join("artisan"), "<?php\n").unwrap();
    std::fs::write(folder.path().join(".env"), "APP_NAME=Original\n").unwrap();
    assert_eq!(
        call(
            &api,
            "serverbond_post_projects",
            json!({"body":{"name":"mcp-proje","path":folder.path()}})
        )["result"]["isError"],
        false
    );
    assert_eq!(api.get("/projects").1["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        call(
            &api,
            "serverbond_put_projects_by_id_env",
            json!({"id":"mcp-proje","body":{"content":"APP_NAME=MCP\n"}})
        )["result"]["isError"],
        false
    );
    assert_eq!(
        api.get("/projects/mcp-proje/env").1["data"]["content"],
        "APP_NAME=MCP\n"
    );
    let jobs = api.get("/projects/mcp-proje/jobs").1["data"].clone();
    assert_eq!(
        call(
            &api,
            "serverbond_put_projects_by_id_jobs",
            json!({"id":"mcp-proje","body":jobs})
        )["result"]["isError"],
        false
    );
    assert_eq!(
        call(
            &api,
            "serverbond_get_projects_by_id_logs",
            json!({"id":"mcp-proje","source":"php"})
        )["result"]["isError"],
        false
    );
    assert_eq!(
        call(
            &api,
            "serverbond_post_services_by_id_stop",
            json!({"id":"redis"})
        )["result"]["isError"],
        false
    );
    assert_eq!(
        call(
            &api,
            "serverbond_delete_projects_by_id",
            json!({"id":"mcp-proje"})
        )["result"]["isError"],
        false
    );
    assert!(api.get("/projects").1["data"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(folder.path().join(".env").exists());
}

#[test]
fn mcp_finishes_deferred_desktop_work_after_sending_the_rpc_response() {
    use std::sync::{mpsc, Mutex};
    struct Host {
        receive: Arc<Mutex<mpsc::Receiver<()>>>,
        done: mpsc::Sender<()>,
    }
    impl api::DesktopApi for Host {
        fn call(&self, operation: &str, _: Value) -> anyhow::Result<api::DesktopReply> {
            assert_eq!(operation, "exit");
            let receive = self.receive.clone();
            let done = self.done.clone();
            Ok(api::DesktopReply {
                data: json!({"accepted":true}),
                after_response: Some(Box::new(move || {
                    receive
                        .lock()
                        .unwrap()
                        .recv_timeout(Duration::from_secs(5))?;
                    done.send(())?;
                    Ok(())
                })),
            })
        }
    }
    let api = enabled();
    let (send, receive) = mpsc::channel();
    let (done, finished) = mpsc::channel();
    api.manager.attach_desktop_api(Arc::new(Host {
        receive: Arc::new(Mutex::new(receive)),
        done,
    }));
    assert_eq!(
        call(&api, "serverbond_post_desktop_exit", json!({}))["result"]["structuredContent"]
            ["data"]["accepted"],
        true
    );
    send.send(()).unwrap();
    finished.recv_timeout(Duration::from_secs(5)).unwrap();
}

#[test]
fn mcp_setting_is_backward_compatible_and_survives_reload() {
    let legacy: serverbond_core::preferences::ApiSettings =
        serde_json::from_value(json!({"enabled":true,"port":18800})).unwrap();
    assert!(!legacy.mcp_enabled);
    let api = enabled();
    let path = api._home.path().to_path_buf();
    api.manager.stop_api();
    drop(api.manager);
    let manager = Manager::new(path).unwrap();
    assert!(manager.api_status().mcp_enabled);
}
