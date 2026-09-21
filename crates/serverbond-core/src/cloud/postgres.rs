use super::mysql::{Confirm, Password};
use crate::Manager;
use anyhow::{ensure, Result};
use serde_json::{json, Value};

pub(super) fn connection(manager: &Manager) -> Result<Value> {
    let port = manager.snapshot()?.settings.postgres.port;
    Ok(json!({"host":"127.0.0.1","port":port,"username":"postgres"}))
}

pub(super) fn credentials(manager: &Manager, input: Confirm) -> Result<Value> {
    ensure!(input.confirm, "Parola görüntüleme onayı gerekli.");
    Ok(json!({"password":manager.postgres_credentials()?}))
}

pub(super) fn password(manager: &Manager, input: Password) -> Result<Value> {
    ensure!(
        input.confirm,
        "Cihaz PostgreSQL parola değişikliği onayı gerekli."
    );
    manager.change_postgres_password(&input.password)?;
    Ok(json!({"changed":true}))
}
