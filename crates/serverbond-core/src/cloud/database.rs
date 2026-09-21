use crate::Manager;
use anyhow::{ensure, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Restore {
    pub id: String,
    pub path: PathBuf,
    pub confirm: bool,
}

pub(super) fn execute(manager: &Manager, id: &str, action: &str) -> Result<Value> {
    uuid::Uuid::parse_str(id)?;
    let project = manager.project(id)?;
    let path = match action {
        "show" => None,
        "create" => {
            manager.create_database(&project.name)?;
            None
        }
        "backup" => Some(manager.backup_database(&project.name)?),
        _ => anyhow::bail!("Bilinmeyen veritabanı işlemi."),
    };
    Ok(
        json!({"projectId":id,"name":project.name,"database":crate::model::database_name(&project.name),"action":action,"path":path}),
    )
}

pub(super) fn restore(manager: &Manager, input: Restore) -> Result<Value> {
    ensure!(input.confirm, "Geri yükleme onayı gerekli.");
    uuid::Uuid::parse_str(&input.id)?;
    ensure!(
        input.path.is_absolute(),
        "SQL dosyasının tam Windows yolu gerekli."
    );
    let project = manager.project(&input.id)?;
    manager.restore_database(&project.name, input.path)?;
    Ok(
        json!({"projectId":input.id,"name":project.name,"database":crate::model::database_name(&project.name),"action":"restore","path":null}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn database_target_is_derived_from_registered_project_and_restore_needs_confirmation() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let path = home.path().join("www/db-check");
        std::fs::create_dir_all(path.join("public")).unwrap();
        std::fs::write(path.join("public/index.php"), "<?php").unwrap();
        std::fs::write(path.join(".env"), "DB_DATABASE=external_database").unwrap();
        let project = manager
            .add_project("db-check".into(), path.clone())
            .unwrap();
        let info = execute(&manager, &project.id, "show").unwrap();
        assert_eq!(info["database"], "db_check");
        assert!(execute(&manager, &uuid::Uuid::new_v4().to_string(), "show").is_err());
        assert!(execute(&manager, &project.id, "create").is_err());
        assert!(execute(&manager, &project.id, "backup").is_err());
        assert!(restore(
            &manager,
            Restore {
                id: project.id.clone(),
                path: home.path().join("test.sql"),
                confirm: false
            }
        )
        .unwrap_err()
        .to_string()
        .contains("onayı"));
        assert!(restore(
            &manager,
            Restore {
                id: project.id,
                path: "test.sql".into(),
                confirm: true
            }
        )
        .unwrap_err()
        .to_string()
        .contains("tam Windows yolu"));
        assert_eq!(
            std::fs::read_to_string(path.join(".env")).unwrap(),
            "DB_DATABASE=external_database"
        );
    }
}
