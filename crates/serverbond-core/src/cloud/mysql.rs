use crate::Manager;
use anyhow::{ensure, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Confirm {
    pub confirm: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Password {
    pub password: String,
    pub confirm: bool,
}

pub(super) fn connection(manager: &Manager) -> Result<Value> {
    let port = manager.snapshot()?.settings.mysql_port;
    Ok(json!({"host":"127.0.0.1","port":port,"username":"root"}))
}

pub(super) fn credentials(manager: &Manager, input: Confirm) -> Result<Value> {
    ensure!(input.confirm, "Parola görüntüleme onayı gerekli.");
    Ok(json!({"password":manager.credentials()?}))
}

pub(super) fn password(manager: &Manager, input: Password) -> Result<Value> {
    ensure!(
        input.confirm,
        "Cihaz MySQL root parola değişikliği onayı gerekli."
    );
    manager.change_mysql_password(&input.password)?;
    Ok(json!({"changed":true}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_does_not_reveal_secrets_and_mutations_require_confirmation() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let value = connection(&manager).unwrap();
        assert_eq!(value["host"], "127.0.0.1");
        assert_eq!(value.as_object().unwrap().len(), 3);
        assert!(credentials(&manager, Confirm { confirm: false }).is_err());
        assert!(credentials(&manager, Confirm { confirm: true }).is_err());
        assert!(password(
            &manager,
            Password {
                password: "test-only-password".into(),
                confirm: false
            }
        )
        .unwrap_err()
        .to_string()
        .contains("onayı"));
        assert!(!home.path().join("config/mysql-password.dpapi").exists());
    }
}
