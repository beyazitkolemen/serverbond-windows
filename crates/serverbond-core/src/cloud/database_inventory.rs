//! Named MySQL databases and scoped users exposed to Cloud.
use crate::Manager;
use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Create {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct User {
    username: String,
    password: String,
    databases: Vec<String>,
    read_only: bool,
    confirm: bool,
}

pub(super) fn show(manager: &Manager) -> Result<Value> {
    manager.database_inventory()
}

pub(super) fn create(manager: &Manager, input: Create) -> Result<Value> {
    manager.create_named_database(&input.name)?;
    Ok(json!({"name":input.name,"created":true}))
}

pub(super) fn user(manager: &Manager, input: User) -> Result<Value> {
    manager.create_database_user(
        &input.username,
        &input.password,
        &input.databases,
        input.read_only,
        input.confirm,
    )?;
    Ok(
        json!({"username":input.username,"databases":input.databases,"readOnly":input.read_only,"created":true}),
    )
}

#[cfg(test)]
mod tests {
    use super::super::operations::Operation;
    use super::*;

    #[test]
    fn cloud_contract_requires_exact_named_database_and_user_parameters() {
        assert!(Operation::parse("databases.show", &json!({})).is_ok());
        assert!(Operation::parse("databases.create", &json!({"name":"shop_db"})).is_ok());
        assert!(
            Operation::parse("databases.create", &json!({"name":"shop_db","id":"other"})).is_err()
        );
        assert!(Operation::parse("databases.user", &json!({"username":"app_user","password":"Only-test-123!","databases":["shop_db"],"readOnly":true,"confirm":true})).is_ok());
        assert!(Operation::parse("databases.user", &json!({"username":"app_user","password":"Only-test-123!","databases":["shop_db"],"readOnly":true})).is_err());
    }
}
