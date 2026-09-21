use crate::Manager;
use anyhow::{ensure, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Client {
    client_id: String,
    confirm: bool,
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
    fn github_account_settings_require_confirmation_and_return_no_tokens() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let client_id = "TestClient123456";
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
