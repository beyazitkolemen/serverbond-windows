//! Typed Cloud operations call the same Manager methods as the local API.
use crate::Manager;
use anyhow::{ensure, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

pub(super) const NAMES: &[&str] = &[
    "projects.list",
    "projects.add",
    "projects.remove",
    "projects.create",
    "projects.import",
    "projects.github",
    "projects.discover",
    "projects.import-folders",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    #[serde(default)]
    offset: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Add {
    name: String,
    path: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Create {
    name: String,
    parent: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Remove {
    id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Import {
    url: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    branch: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Github {
    repository: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    branch: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Paths {
    paths: Vec<PathBuf>,
}

#[derive(Deserialize)]
#[serde(tag = "operation", content = "parameters", deny_unknown_fields)]
pub(super) enum Operation {
    #[serde(rename = "projects.list")]
    List(Page),
    #[serde(rename = "projects.add")]
    Add(Add),
    #[serde(rename = "projects.remove")]
    Remove(Remove),
    #[serde(rename = "projects.create")]
    Create(Create),
    #[serde(rename = "projects.import")]
    Import(Import),
    #[serde(rename = "projects.github")]
    Github(Github),
    #[serde(rename = "projects.discover")]
    Discover(Page),
    #[serde(rename = "projects.import-folders")]
    ImportFolders(Paths),
}

impl Operation {
    pub(super) fn parse(name: &str, parameters: &Value) -> Result<Self> {
        ensure!(
            serde_json::to_vec(parameters)?.len() <= 32768,
            "İşlem parametreleri çok büyük."
        );
        Ok(serde_json::from_value(
            json!({"operation":name,"parameters":parameters}),
        )?)
    }
    pub(super) fn execute(self, manager: &Manager) -> Result<Value> {
        match self {
            Self::List(page) => return project_page(manager, page.offset),
            Self::Discover(page) => {
                ensure!(page.offset <= 1000, "Sayfa aralığı geçersiz.");
                let folders = manager.discover_projects()?;
                return Ok(
                    json!({"folders": folders.iter().skip(page.offset).take(50).collect::<Vec<_>>(), "offset":page.offset,"total":folders.len()}),
                );
            }
            Self::Add(input) => {
                manager.add_project(input.name, input.path)?;
            }
            Self::Remove(input) => {
                uuid::Uuid::parse_str(&input.id)?;
                manager.remove_project(&input.id)?;
            }
            Self::Create(input) => {
                manager.create_project(input.name, input.parent)?;
            }
            Self::Import(input) => {
                manager.import_git_project(&input.url, input.name, input.branch)?;
            }
            Self::Github(input) => {
                manager.import_github_project(&input.repository, input.name, input.branch)?;
            }
            Self::ImportFolders(input) => {
                ensure!(
                    !input.paths.is_empty() && input.paths.len() <= 50,
                    "1–50 klasör seçin."
                );
                manager.import_projects(input.paths)?;
            }
        }
        project_page(manager, 0)
    }
}

fn project_page(manager: &Manager, offset: usize) -> Result<Value> {
    ensure!(offset <= 1000, "Sayfa aralığı geçersiz.");
    let snapshot = manager.snapshot()?;
    let total = snapshot.projects.len();
    let projects: Vec<Value> = snapshot
        .projects
        .into_iter()
        .skip(offset)
        .take(50)
        .map(|state| {
            let project = state.project;
            json!({"id":project.id,"name":project.name,"path":project.path,"host":project.host,
            "phpVersion":project.php_version,"running":state.running})
        })
        .collect();
    Ok(json!({"projects":projects,"total":total,"offset":offset}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operations_reject_unknown_fields_and_invalid_targets() {
        assert!(Operation::parse("shell", &json!({})).is_err());
        assert!(Operation::parse("projects.list", &json!({"command":"bad"})).is_err());
        assert!(Operation::parse("projects.remove", &json!({})).is_err());
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        assert!(Operation::parse("projects.list", &json!({"offset":1001}))
            .unwrap()
            .execute(&manager)
            .is_err());
        assert!(
            Operation::parse("projects.remove", &json!({"id":"../config"}))
                .unwrap()
                .execute(&manager)
                .is_err()
        );
        assert!(
            Operation::parse("projects.import-folders", &json!({"paths":[]}))
                .unwrap()
                .execute(&manager)
                .is_err()
        );
    }
    #[test]
    fn projects_are_added_listed_and_removed_without_deleting_files() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let path = home.path().join("www/cloud-project");
        std::fs::create_dir_all(path.join("public")).unwrap();
        std::fs::write(path.join("public/index.php"), "<?php echo 'ok';").unwrap();
        std::fs::write(path.join(".env"), "SENTINEL=private").unwrap();
        let result = Operation::parse("projects.add", &json!({"name":"cloud-project","path":path}))
            .unwrap()
            .execute(&manager)
            .unwrap();
        assert_eq!(result["projects"][0]["name"], "cloud-project");
        assert!(!result.to_string().contains("SENTINEL"));
        let id = result["projects"][0]["id"].as_str().unwrap();
        let result = Operation::parse("projects.remove", &json!({"id":id}))
            .unwrap()
            .execute(&manager)
            .unwrap();
        assert_eq!(result["total"], 0);
        assert_eq!(
            std::fs::read_to_string(path.join(".env")).unwrap(),
            "SENTINEL=private"
        );
    }
}
