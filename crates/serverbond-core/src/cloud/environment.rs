use crate::{
    envfile::{env_revision, ProjectEnv, ENV_LIMIT},
    Manager,
};
use anyhow::{ensure, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Write {
    pub id: String,
    pub content_base64: String,
    pub expected_revision: String,
    pub confirm: bool,
}

fn result(id: &str, env: ProjectEnv) -> Value {
    json!({"projectId":id,"exists":env.exists,"revision":env_revision(&env),"contentBase64":STANDARD.encode(env.content.as_bytes()),"exampleBase64":env.example.as_ref().map(|text| STANDARD.encode(text.as_bytes()))})
}
pub(super) fn read(manager: &Manager, id: &str) -> Result<Value> {
    uuid::Uuid::parse_str(id)?;
    Ok(result(id, manager.read_project_env(id)?))
}
pub(super) fn write(manager: &Manager, input: Write) -> Result<Value> {
    uuid::Uuid::parse_str(&input.id)?;
    ensure!(input.confirm, "Ortam dosyası kayıt onayı gerekli.");
    let bytes = STANDARD.decode(input.content_base64)?;
    ensure!(bytes.len() as u64 <= ENV_LIMIT, ".env dosyası çok büyük.");
    let content = String::from_utf8(bytes)?;
    let env =
        manager.save_project_env_checked(&input.id, content, Some(&input.expected_revision))?;
    Ok(result(&input.id, env))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn writes_require_confirmation_and_current_revision_and_preserve_bytes() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let path = home.path().join("www/env-check");
        std::fs::create_dir_all(path.join("public")).unwrap();
        std::fs::write(path.join("public/index.php"), "<?php").unwrap();
        let project = manager
            .add_project("env-check".into(), path.clone())
            .unwrap();
        let missing = read(&manager, &project.id).unwrap();
        assert_eq!(missing["exists"], false);
        let input = |revision: &Value, text: &str, confirm| Write {
            id: project.id.clone(),
            content_base64: STANDARD.encode(text),
            expected_revision: revision.as_str().unwrap().into(),
            confirm,
        };
        assert!(write(&manager, input(&missing["revision"], "", false)).is_err());
        assert!(!path.join(".env").exists());
        let empty = write(&manager, input(&missing["revision"], "", true)).unwrap();
        assert_ne!(empty["revision"], missing["revision"]);
        let content = "\n APP_NAME=\"Türkçe\"\r\nTOKEN=local-test-only  \n";
        let saved = write(&manager, input(&empty["revision"], content, true)).unwrap();
        assert_eq!(
            std::fs::read(path.join(".env")).unwrap(),
            content.as_bytes()
        );
        assert_eq!(
            STANDARD
                .decode(saved["contentBase64"].as_str().unwrap())
                .unwrap(),
            content.as_bytes()
        );
        std::fs::write(path.join(".env"), "EXTERNAL=changed").unwrap();
        assert!(
            write(&manager, input(&saved["revision"], "OVERWRITE=no", true))
                .unwrap_err()
                .is::<crate::envfile::EnvConflict>()
        );
        assert_eq!(
            std::fs::read_to_string(path.join(".env")).unwrap(),
            "EXTERNAL=changed"
        );
        let current = read(&manager, &project.id).unwrap();
        assert!(write(&manager, input(&current["revision"], "BAD=\0", true)).is_err());
        let full = "ş".repeat(ENV_LIMIT as usize / 2);
        std::fs::write(path.join(".env.example"), &full).unwrap();
        let large = write(&manager, input(&current["revision"], &full, true)).unwrap();
        assert!(
            serde_json::to_vec(&large).unwrap().len()
                < super::super::operations::output_limit(Some("env.write"))
        );
        assert_eq!(
            std::fs::metadata(path.join(".env")).unwrap().len(),
            ENV_LIMIT
        );
        assert!(write(&manager, input(&large["revision"], &(full + "x"), true)).is_err());
    }
}
