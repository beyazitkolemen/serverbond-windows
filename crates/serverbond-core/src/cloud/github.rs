use crate::Manager;
use anyhow::{ensure, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    page: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Branches {
    repository: String,
    page: u32,
}

pub(super) fn repositories(manager: &Manager, input: Page) -> Result<Value> {
    let page = serde_json::to_value(manager.github_repositories(input.page)?)?;
    let repositories: Vec<Value> = page["repositories"].as_array().unwrap().iter().map(|repo| {
        json!({"fullName":repo["fullName"],"private":repo["private"],"archived":repo["archived"],"empty":repo["empty"],"defaultBranch":repo["defaultBranch"]})
    }).collect();
    Ok(json!({"page":input.page,"nextPage":page["nextPage"],"repositories":repositories}))
}

pub(super) fn branches(manager: &Manager, input: Branches) -> Result<Value> {
    let page = serde_json::to_value(manager.github_branches(&input.repository, input.page)?)?;
    Ok(
        json!({"repository":input.repository,"page":input.page,"nextPage":page["nextPage"],"defaultBranch":page["defaultBranch"],"branches":page["branches"]}),
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Client {
    client_id: String,
    confirm: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Flow {
    flow_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Cancel {
    flow_id: String,
    confirm: bool,
}

pub(super) fn start(manager: &Manager, input: super::mysql::Confirm) -> Result<Value> {
    ensure!(
        input.confirm,
        "GitHub girişini başlatmak için açık onay gerekli."
    );
    Ok(serde_json::to_value(manager.github_auth_start()?)?)
}

pub(super) fn poll(manager: &Manager, input: Flow) -> Result<Value> {
    uuid::Uuid::parse_str(&input.flow_id)?;
    let state = manager.github_auth_poll(&input.flow_id)?;
    Ok(
        json!({"flowId":input.flow_id,"status":state.status,"retryAfter":state.retry_after,"login":state.login}),
    )
}

pub(super) fn cancel(manager: &Manager, input: Cancel) -> Result<Value> {
    ensure!(
        input.confirm,
        "GitHub girişini iptal etmek için açık onay gerekli."
    );
    uuid::Uuid::parse_str(&input.flow_id)?;
    manager.github_auth_cancel(&input.flow_id)?;
    Ok(json!({"flowId":input.flow_id,"status":"cancelled","retryAfter":0,"login":null}))
}

pub(super) fn show(manager: &Manager) -> Value {
    let state = manager.github_state();
    json!({"tokenSaved":state.token_saved,"login":state.login,"clientId":state.oauth_client_id,"authMethod":state.auth_method,"expiresAt":state.expires_at})
}

pub(super) fn client(manager: &Manager, input: Client) -> Result<Value> {
    ensure!(
        input.confirm,
        "GitHub istemci ayarı için açık onay gerekli."
    );
    manager.save_github_client_id(&input.client_id)?;
    Ok(show(manager))
}

pub(super) fn disconnect(manager: &Manager, input: super::mysql::Confirm) -> Result<Value> {
    ensure!(
        input.confirm,
        "GitHub bağlantısını kaldırmak için açık onay gerekli."
    );
    manager.clear_github_token()?;
    Ok(show(manager))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn github_catalog_rejects_invalid_pages_before_network_access() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        assert!(repositories(&manager, Page { page: 0 }).is_err());
        assert!(repositories(&manager, Page { page: 1 }).is_err());
        assert!(branches(
            &manager,
            Branches {
                repository: "owner/repo".into(),
                page: 10001
            }
        )
        .is_err());
        assert!(branches(
            &manager,
            Branches {
                repository: "https://invalid.test/repo".into(),
                page: 1
            }
        )
        .is_err());
    }
    #[test]
    fn github_account_settings_require_confirmation_and_return_no_tokens() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let client_id = "TestClient123456";
        assert!(start(&manager, super::super::mysql::Confirm { confirm: false }).is_err());
        let flow_id = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            poll(
                &manager,
                Flow {
                    flow_id: flow_id.clone()
                }
            )
            .unwrap()["status"],
            "cancelled"
        );
        assert!(cancel(
            &manager,
            Cancel {
                flow_id: flow_id.clone(),
                confirm: false
            }
        )
        .is_err());
        assert_eq!(
            cancel(
                &manager,
                Cancel {
                    flow_id,
                    confirm: true
                }
            )
            .unwrap()["status"],
            "cancelled"
        );
        assert!(client(
            &manager,
            Client {
                client_id: client_id.into(),
                confirm: false
            }
        )
        .is_err());
        assert!(client(
            &manager,
            Client {
                client_id: "invalid!".into(),
                confirm: true
            }
        )
        .is_err());
        let saved = client(
            &manager,
            Client {
                client_id: client_id.into(),
                confirm: true,
            },
        )
        .unwrap();
        assert_eq!(saved["clientId"], client_id);
        assert_eq!(saved["tokenSaved"], false);
        assert_eq!(saved.as_object().unwrap().len(), 5);
        assert!(disconnect(&manager, super::super::mysql::Confirm { confirm: false }).is_err());
        assert_eq!(
            disconnect(&manager, super::super::mysql::Confirm { confirm: true }).unwrap()
                ["tokenSaved"],
            false
        );
        assert_eq!(show(&manager)["clientId"], client_id);
        assert_eq!(
            client(
                &manager,
                Client {
                    client_id: String::new(),
                    confirm: true
                }
            )
            .unwrap()["clientId"],
            ""
        );
    }
}
