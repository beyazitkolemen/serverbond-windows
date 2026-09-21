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
    "projects.show",
    "projects.release",
    "projects.deploy",
    "php.list",
    "php.select",
    "php.repair",
    "projects.php",
    "projects.php-repair",
    "jobs.show",
    "jobs.save",
    "jobs.worker",
    "jobs.schedule",
    "jobs.failed",
    "jobs.retry",
    "jobs.flush",
    "jobs.tasks",
    "projects.log",
    "env.read",
    "env.write",
];

pub(super) fn output_limit(name: Option<&str>) -> usize {
    if matches!(name, Some("env.read" | "env.write")) {
        768 * 1024
    } else {
        256 * 1024
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Empty {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Php {
    version: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProjectPhp {
    id: String,
    version: String,
}

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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Release {
    id: String,
    release: crate::model::ProjectRelease,
    expected_revision: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Deploy {
    id: String,
    expected_revision: String,
    confirm: bool,
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
    #[serde(rename = "env.read")]
    EnvRead(Remove),
    #[serde(rename = "env.write")]
    EnvWrite(super::environment::Write),
    #[serde(rename = "jobs.failed")]
    JobsFailed(Remove),
    #[serde(rename = "jobs.retry")]
    JobsRetry(super::jobs::Retry),
    #[serde(rename = "jobs.flush")]
    JobsFlush(super::jobs::Flush),
    #[serde(rename = "jobs.tasks")]
    JobsTasks(Remove),
    #[serde(rename = "projects.log")]
    ProjectLog(super::jobs::Log),
    #[serde(rename = "jobs.show")]
    JobsShow(Remove),
    #[serde(rename = "jobs.save")]
    JobsSave(super::jobs::Save),
    #[serde(rename = "jobs.worker")]
    JobsWorker(super::jobs::Control),
    #[serde(rename = "jobs.schedule")]
    JobsSchedule(super::jobs::Control),
    #[serde(rename = "php.list")]
    PhpList(Empty),
    #[serde(rename = "php.select")]
    PhpSelect(Php),
    #[serde(rename = "php.repair")]
    PhpRepair(Php),
    #[serde(rename = "projects.php")]
    ProjectPhp(ProjectPhp),
    #[serde(rename = "projects.php-repair")]
    ProjectPhpRepair(ProjectPhp),
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
    #[serde(rename = "projects.show")]
    Show(Remove),
    #[serde(rename = "projects.release")]
    Release(Release),
    #[serde(rename = "projects.deploy")]
    Deploy(Deploy),
}

impl Operation {
    pub(super) fn parse(name: &str, parameters: &Value) -> Result<Self> {
        ensure!(
            serde_json::to_vec(parameters)?.len()
                <= if name == "env.write" {
                    384 * 1024
                } else {
                    32768
                },
            "İşlem parametreleri çok büyük."
        );
        Ok(serde_json::from_value(
            json!({"operation":name,"parameters":parameters}),
        )?)
    }
    pub(super) fn execute(self, manager: &Manager) -> Result<Value> {
        match self {
            Self::EnvRead(input) => return super::environment::read(manager, &input.id),
            Self::EnvWrite(input) => return super::environment::write(manager, input),
            Self::JobsFailed(input) => {
                uuid::Uuid::parse_str(&input.id)?;
                return Ok(super::jobs::text_result(
                    &input.id,
                    "failed",
                    manager.list_failed_jobs(&input.id)?,
                    false,
                ));
            }
            Self::JobsRetry(input) => {
                uuid::Uuid::parse_str(&input.id)?;
                ensure!(input.confirm, "Yeniden deneme onayı gerekli.");
                return Ok(super::jobs::text_result(
                    &input.id,
                    "retry",
                    manager.retry_failed_jobs(&input.id, Some(&input.job))?,
                    false,
                ));
            }
            Self::JobsFlush(input) => {
                uuid::Uuid::parse_str(&input.id)?;
                ensure!(input.confirm, "Temizleme onayı gerekli.");
                return Ok(super::jobs::text_result(
                    &input.id,
                    "flush",
                    manager.flush_failed_jobs(&input.id)?,
                    false,
                ));
            }
            Self::JobsTasks(input) => {
                uuid::Uuid::parse_str(&input.id)?;
                return Ok(super::jobs::text_result(
                    &input.id,
                    "tasks",
                    manager.list_project_schedule(&input.id)?,
                    false,
                ));
            }
            Self::ProjectLog(input) => {
                uuid::Uuid::parse_str(&input.id)?;
                return Ok(super::jobs::text_result(
                    &input.id,
                    &input.source,
                    manager.read_project_log(&input.id, &input.source)?,
                    true,
                ));
            }
            Self::JobsShow(input) => return super::jobs::show(manager, &input.id),
            Self::JobsSave(input) => return super::jobs::save(manager, input),
            Self::JobsWorker(input) => return super::jobs::control(manager, input, true),
            Self::JobsSchedule(input) => return super::jobs::control(manager, input, false),
            Self::PhpList(_) => return php_inventory(manager),
            Self::PhpSelect(input) => {
                manager.select_php(&input.version)?;
                return php_inventory(manager);
            }
            Self::PhpRepair(input) => {
                manager.repair_php(&input.version)?;
                return php_inventory(manager);
            }
            Self::ProjectPhp(input) => {
                uuid::Uuid::parse_str(&input.id)?;
                manager.select_project_php(&input.id, &input.version)?;
                return project_details(manager, &input.id);
            }
            Self::ProjectPhpRepair(input) => {
                uuid::Uuid::parse_str(&input.id)?;
                manager.repair_project_php(&input.id, &input.version)?;
                return project_details(manager, &input.id);
            }
            Self::Show(input) => return project_details(manager, &input.id),
            Self::Release(input) => {
                uuid::Uuid::parse_str(&input.id)?;
                manager.save_project_release_checked(
                    &input.id,
                    input.release,
                    Some(&input.expected_revision),
                )?;
                return project_details(manager, &input.id);
            }
            Self::Deploy(input) => {
                uuid::Uuid::parse_str(&input.id)?;
                ensure!(input.confirm, "Dağıtım onayı gerekli.");
                let record =
                    manager.deploy_project_checked(&input.id, Some(&input.expected_revision))?;
                return Ok(json!({"projectId":input.id, "deployment":release_summary(record)}));
            }
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

fn release_summary(record: crate::release::ReleaseRecord) -> Value {
    let truncated = record.output.chars().count() > 8000;
    let output: String = record.output.chars().take(8000).collect();
    json!({"startedAt":record.started_at,"durationMs":record.duration_ms,"branch":record.branch,
        "sha":record.sha,"success":record.success,"output":output,"truncated":truncated})
}

fn project_details(manager: &Manager, id: &str) -> Result<Value> {
    uuid::Uuid::parse_str(id)?;
    let project = manager.project(id)?;
    let revision = crate::release::release_revision(&project.release)?;
    let git = manager.project_git_status(id)?;
    let releases: Vec<Value> = manager
        .list_project_releases(id)?
        .into_iter()
        .take(5)
        .map(release_summary)
        .collect();
    Ok(
        json!({"project":{"id":project.id,"name":project.name,"path":project.path,"host":project.host,"phpVersion":project.php_version},
        "git":git,"release":project.release,"revision":revision,"releases":releases,
        "phpVersions":crate::model::php_versions().into_iter().map(|p| p.version).collect::<Vec<_>>()}),
    )
}

fn php_inventory(manager: &Manager) -> Result<Value> {
    let snapshot = manager.snapshot()?;
    let selected = snapshot
        .packages
        .iter()
        .find(|p| p.package.id == "php")
        .map(|p| p.package.version.clone())
        .unwrap_or_default();
    let versions: Vec<Value> = snapshot.php_versions.into_iter().map(|p| json!({"version":p.package.version,"installed":p.installed,"running":p.running,"repairable":p.repairable})).collect();
    Ok(json!({"selected":selected,"anyRunning":snapshot.any_running,"versions":versions}))
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
    fn php_catalog_is_safe_and_unknown_versions_do_not_change_selection() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let before = php_inventory(&manager).unwrap();
        assert_eq!(
            before["versions"].as_array().unwrap().len(),
            crate::model::php_versions().len()
        );
        assert!(!before.to_string().contains("sha256"));
        assert!(!before.to_string().contains("settings"));
        assert!(Operation::parse("php.list", &json!({"shell":"bad"})).is_err());
        for name in ["php.select", "php.repair"] {
            assert!(Operation::parse(name, &json!({"version":"99.0.0"}))
                .unwrap()
                .execute(&manager)
                .is_err());
        }
        assert_eq!(before, php_inventory(&manager).unwrap());
    }
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

    #[test]
    fn stale_recipes_and_unconfirmed_deployments_cannot_mutate_projects() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let path = home.path().join("www/revision-test");
        std::fs::create_dir_all(path.join("public")).unwrap();
        std::fs::write(path.join("public/index.php"), "<?php").unwrap();
        let project = manager.add_project("revision-test".into(), path).unwrap();
        let details = project_details(&manager, &project.id).unwrap();
        let mut recipe = details["release"].clone();
        recipe["branch"] = json!("staging");
        let parameters =
            json!({"id":project.id,"release":recipe,"expectedRevision":details["revision"]});
        let saved = Operation::parse("projects.release", &parameters)
            .unwrap()
            .execute(&manager)
            .unwrap();
        assert_ne!(saved["revision"], details["revision"]);
        assert!(Operation::parse("projects.release", &parameters)
            .unwrap()
            .execute(&manager)
            .is_err());
        let stale = json!({"id":project.id,"expectedRevision":details["revision"],"confirm":true});
        let error = Operation::parse("projects.deploy", &stale)
            .unwrap()
            .execute(&manager)
            .unwrap_err();
        assert!(error.to_string().contains("tarifi değişti"));
        let unconfirmed =
            json!({"id":project.id,"expectedRevision":saved["revision"],"confirm":false});
        assert!(Operation::parse("projects.deploy", &unconfirmed)
            .unwrap()
            .execute(&manager)
            .is_err());
        assert!(manager
            .list_project_releases(&project.id)
            .unwrap()
            .is_empty());
        assert_eq!(
            manager.project(&project.id).unwrap().release.branch,
            "staging"
        );
    }
}
