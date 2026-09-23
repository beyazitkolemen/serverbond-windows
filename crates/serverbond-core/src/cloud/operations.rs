//! Typed Cloud operations call the same Manager methods as the local API.
use crate::Manager;
use anyhow::{ensure, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ServiceLog {
    service: String,
}

pub(super) const NAMES: &[&str] = &[
    "desktop.update-check",
    "desktop.update-install",
    "desktop.show",
    "desktop.save",
    "desktop.appearance",
    "system.diagnostics",
    "services.log",
    "github.auth-start",
    "github.repositories",
    "github.branches",
    "github.auth-poll",
    "github.auth-cancel",
    "github.show",
    "github.client",
    "github.disconnect",
    "tunnel.show",
    "tunnel.token",
    "tunnel.apply",
    "tunnel.clear",
    "tunnel.auto-start",
    "settings.show",
    "settings.save",
    "settings.validate",
    "settings.defaults",
    "settings.previous",
    "postgres.connection",
    "postgres.credentials",
    "postgres.password",
    "mysql.connection",
    "mysql.credentials",
    "mysql.password",
    "databases.show",
    "databases.create",
    "databases.user",
    "database.show",
    "database.create",
    "database.backup",
    "database.restore",
    "projects.preflight",
    "projects.setup-check",
    "projects.setup",
    "projects.health",
    "projects.create-template",
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
    #[serde(default)]
    parent: Option<PathBuf>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CreateTemplate {
    name: String,
    template: crate::project_workflow::ProjectTemplate,
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
    #[serde(default)]
    access_token: Option<String>,
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Github {
    repository: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    branch: String,
    #[serde(default)]
    access_token: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Paths {
    paths: Vec<PathBuf>,
}

#[derive(Deserialize)]
#[serde(tag = "operation", content = "parameters", deny_unknown_fields)]
pub(super) enum Operation {
    #[serde(rename = "projects.setup-check")]
    SetupCheck(crate::project_setup::SetupRequest),
    #[serde(rename = "projects.setup")]
    Setup(crate::project_setup::SetupRequest),
    #[serde(rename = "projects.preflight")]
    Preflight(Empty),
    #[serde(rename = "projects.health")]
    Health(Remove),
    #[serde(rename = "projects.create-template")]
    CreateTemplate(CreateTemplate),
    #[serde(rename = "desktop.update-check")]
    DesktopUpdateCheck(Empty),
    #[serde(rename = "desktop.update-install")]
    DesktopUpdateInstall(super::desktop::Install),
    #[serde(rename = "desktop.show")]
    DesktopShow(Empty),
    #[serde(rename = "desktop.save")]
    DesktopSave(super::desktop::Save),
    #[serde(rename = "desktop.appearance")]
    DesktopAppearance(super::desktop::Appearance),
    #[serde(rename = "system.diagnostics")]
    Diagnostics(Empty),
    #[serde(rename = "services.log")]
    ServiceLog(ServiceLog),
    #[serde(rename = "github.repositories")]
    GithubRepositories(super::github::Page),
    #[serde(rename = "github.branches")]
    GithubBranches(super::github::Branches),
    #[serde(rename = "github.auth-start")]
    GithubAuthStart(super::mysql::Confirm),
    #[serde(rename = "github.auth-poll")]
    GithubAuthPoll(super::github::Flow),
    #[serde(rename = "github.auth-cancel")]
    GithubAuthCancel(super::github::Cancel),
    #[serde(rename = "github.show")]
    GithubShow(Empty),
    #[serde(rename = "github.client")]
    GithubClient(super::github::Client),
    #[serde(rename = "github.disconnect")]
    GithubDisconnect(super::mysql::Confirm),
    #[serde(rename = "tunnel.show")]
    TunnelShow(Empty),
    #[serde(rename = "tunnel.token")]
    TunnelToken(super::tunnel::Token),
    #[serde(rename = "tunnel.apply")]
    TunnelApply(super::tunnel::Token),
    #[serde(rename = "tunnel.clear")]
    TunnelClear(super::mysql::Confirm),
    #[serde(rename = "tunnel.auto-start")]
    TunnelAutoStart(super::tunnel::AutoStart),
    #[serde(rename = "settings.show")]
    SettingsShow(Empty),
    #[serde(rename = "settings.save")]
    SettingsSave(Box<super::settings::Save>),
    #[serde(rename = "settings.validate")]
    SettingsValidate(Box<super::settings::Validate>),
    #[serde(rename = "settings.defaults")]
    SettingsDefaults(Empty),
    #[serde(rename = "settings.previous")]
    SettingsPrevious(Empty),
    #[serde(rename = "postgres.connection")]
    PostgresConnection(Empty),
    #[serde(rename = "postgres.credentials")]
    PostgresCredentials(super::mysql::Confirm),
    #[serde(rename = "postgres.password")]
    PostgresPassword(super::mysql::Password),
    #[serde(rename = "mysql.connection")]
    MysqlConnection(Empty),
    #[serde(rename = "mysql.credentials")]
    MysqlCredentials(super::mysql::Confirm),
    #[serde(rename = "mysql.password")]
    MysqlPassword(super::mysql::Password),
    #[serde(rename = "databases.show")]
    DatabasesShow(Empty),
    #[serde(rename = "databases.create")]
    DatabasesCreate(super::database_inventory::Create),
    #[serde(rename = "databases.user")]
    DatabasesUser(super::database_inventory::User),
    #[serde(rename = "database.show")]
    DatabaseShow(Remove),
    #[serde(rename = "database.create")]
    DatabaseCreate(Remove),
    #[serde(rename = "database.backup")]
    DatabaseBackup(Remove),
    #[serde(rename = "database.restore")]
    DatabaseRestore(super::database::Restore),
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
                <= match name {
                    "env.write" => 384 * 1024,
                    "settings.save" | "settings.validate" => 256 * 1024,
                    _ => 32768,
                },
            "İşlem parametreleri çok büyük."
        );
        Ok(serde_json::from_value(
            json!({"operation":name,"parameters":parameters}),
        )?)
    }
    pub(super) fn prepare(self, manager: &Manager) -> Result<crate::api::DesktopReply> {
        match self {
            Self::DesktopUpdateInstall(input) => super::desktop::install(manager, input),
            other => other
                .execute(manager)
                .map(crate::api::DesktopReply::immediate),
        }
    }
    pub(super) fn execute(self, manager: &Manager) -> Result<Value> {
        match self {
            Self::SetupCheck(input) => return manager.setup_preflight(&input),
            Self::Setup(input) => {
                manager.setup_project(input)?;
            }
            Self::Preflight(_) => return manager.project_preflight(),
            Self::Health(input) => return manager.project_health(&input.id),
            Self::CreateTemplate(input) => {
                manager.create_project_with_template(
                    input.name,
                    manager.cloud_project_parent(),
                    Some(input.template),
                )?;
            }
            Self::DesktopUpdateCheck(_) => return super::desktop::update_check(manager),
            Self::DesktopUpdateInstall(_) => anyhow::bail!("Update requires acknowledged delivery"),
            Self::DesktopShow(_) => return super::desktop::show(manager),
            Self::DesktopSave(input) => return super::desktop::save(manager, input),
            Self::DesktopAppearance(input) => return super::desktop::appearance(manager, input),
            Self::Diagnostics(_) => {
                let checks: Vec<Value> = manager.requirements().into_iter().map(|check| json!({"id":check.id,"label":check.label,"status":check.status,"detail":check.detail.chars().take(4000).collect::<String>()})).collect();
                let permissions = manager.permission_state();
                return Ok(
                    json!({"checks":checks,"permissions":{"granted":permissions.granted,"helper":permissions.helper,"defenderExclusion":permissions.defender_exclusion,"declined":permissions.declined,"pendingCount":permissions.pending.len(),"failedCount":permissions.failed.len()}}),
                );
            }
            Self::ServiceLog(input) => {
                ensure!(
                    [
                        "php", "mysql", "caddy", "composer", "tunnel", "mail", "postgres", "redis",
                        "github"
                    ]
                    .contains(&input.service.as_str()),
                    "Geçersiz servis günlüğü."
                );
                let text = manager.read_log(&input.service)?;
                let count = text.chars().count();
                let text: String = text.chars().skip(count.saturating_sub(32000)).collect();
                return Ok(json!({"service":input.service,"text":text,"truncated":count>32000}));
            }
            Self::GithubAuthStart(input) => return super::github::start(manager, input),
            Self::GithubRepositories(input) => return super::github::repositories(manager, input),
            Self::GithubBranches(input) => return super::github::branches(manager, input),
            Self::GithubAuthPoll(input) => return super::github::poll(manager, input),
            Self::GithubAuthCancel(input) => return super::github::cancel(manager, input),
            Self::GithubShow(_) => return Ok(super::github::show(manager)),
            Self::GithubClient(input) => return super::github::client(manager, input),
            Self::GithubDisconnect(input) => return super::github::disconnect(manager, input),
            Self::TunnelShow(_) => return super::tunnel::show(manager),
            Self::TunnelToken(input) => return super::tunnel::token(manager, input, false),
            Self::TunnelApply(input) => return super::tunnel::token(manager, input, true),
            Self::TunnelClear(input) => return super::tunnel::clear(manager, input),
            Self::TunnelAutoStart(input) => return super::tunnel::auto_start(manager, input),
            Self::SettingsShow(_) => return super::settings::show(manager),
            Self::SettingsSave(input) => return super::settings::save(manager, *input),
            Self::SettingsValidate(input) => return super::settings::validate(*input),
            Self::SettingsDefaults(_) => return Ok(super::settings::defaults(manager)),
            Self::SettingsPrevious(_) => return super::settings::previous(manager),
            Self::PostgresConnection(_) => return super::postgres::connection(manager),
            Self::PostgresCredentials(input) => {
                return super::postgres::credentials(manager, input)
            }
            Self::PostgresPassword(input) => return super::postgres::password(manager, input),
            Self::MysqlConnection(_) => return super::mysql::connection(manager),
            Self::MysqlCredentials(input) => return super::mysql::credentials(manager, input),
            Self::MysqlPassword(input) => return super::mysql::password(manager, input),
            Self::DatabasesShow(_) => return super::database_inventory::show(manager),
            Self::DatabasesCreate(input) => {
                return super::database_inventory::create(manager, input)
            }
            Self::DatabasesUser(input) => return super::database_inventory::user(manager, input),
            Self::DatabaseShow(input) => {
                return super::database::execute(manager, &input.id, "show")
            }
            Self::DatabaseCreate(input) => {
                return super::database::execute(manager, &input.id, "create")
            }
            Self::DatabaseBackup(input) => {
                return super::database::execute(manager, &input.id, "backup")
            }
            Self::DatabaseRestore(input) => return super::database::restore(manager, input),
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
                let record = manager.deploy_project_checked_with_token(
                    &input.id,
                    Some(&input.expected_revision),
                    input.access_token.as_deref(),
                )?;
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
                let path = manager.cloud_project_path(input.path)?;
                manager.add_project(input.name, path)?;
            }
            Self::Remove(input) => {
                uuid::Uuid::parse_str(&input.id)?;
                manager.remove_project(&input.id)?;
            }
            Self::Create(input) => {
                let parent = input
                    .parent
                    .unwrap_or_else(|| manager.cloud_project_parent());
                manager.create_project(input.name, parent)?;
            }
            Self::Import(input) => {
                manager.import_git_project(&input.url, input.name, input.branch)?;
            }
            Self::Github(input) => {
                manager.import_github_project_with_token(
                    &input.repository,
                    input.name,
                    input.branch,
                    input.access_token.as_deref(),
                )?;
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
    Ok(php_inventory_from_snapshot(&snapshot))
}

pub(super) fn php_inventory_from_snapshot(snapshot: &crate::model::Snapshot) -> Value {
    let selected = snapshot
        .packages
        .iter()
        .find(|p| p.package.id == "php")
        .map(|p| p.package.version.as_str())
        .unwrap_or_default();
    let versions: Vec<Value> = snapshot.php_versions.iter().map(|p| json!({"version":p.package.version,"installed":p.installed,"running":p.running,"repairable":p.repairable})).collect();
    json!({"selected":selected,"anyRunning":snapshot.any_running,"versions":versions})
}

fn project_page(manager: &Manager, offset: usize) -> Result<Value> {
    ensure!(offset <= 1000, "Sayfa aralığı geçersiz.");
    let snapshot = manager.snapshot()?;
    Ok(project_inventory(&snapshot.projects, offset, 50))
}

pub(super) fn project_inventory(
    states: &[crate::model::ProjectStatus],
    offset: usize,
    limit: usize,
) -> Value {
    let projects: Vec<Value> = states
        .iter()
        .skip(offset)
        .take(limit)
        .map(|state| {
            let project = &state.project;
            json!({"id":project.id,"name":project.name,"path":project.path,"host":project.host,
            "phpVersion":project.php_version,"running":state.running})
        })
        .collect();
    json!({"projects":projects,"total":states.len(),"offset":offset})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_inventory_keeps_all_projects_while_commands_remain_paged() {
        let states = (1..=51)
            .map(|number| crate::model::ProjectStatus {
                project: crate::model::Project {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("project-{number}"),
                    path: PathBuf::from(format!("C:/projects/project-{number}")),
                    host: format!("project-{number}.test"),
                    php_version: "8.4".into(),
                    workers: Vec::new(),
                    schedule: Default::default(),
                    release: Default::default(),
                },
                running: number == 51,
                pid: None,
                php_port: None,
                issue: None,
                worker_states: Vec::new(),
                schedule_running: false,
                schedule_pid: None,
                schedule_issue: None,
            })
            .collect::<Vec<_>>();
        let full = project_inventory(&states, 0, 1000);
        let page = project_inventory(&states, 0, 50);
        assert_eq!(full["projects"].as_array().unwrap().len(), 51);
        assert_eq!(full["projects"][50]["name"], "project-51");
        assert_eq!(full["projects"][50]["running"], true);
        assert_eq!(page["projects"].as_array().unwrap().len(), 50);
        assert_eq!(page["total"], 51);
    }

    #[test]
    fn folder_only_add_uses_configured_workspace_and_preserves_absolute_clients() {
        let home = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        manager.config.lock().unwrap().settings.projects_dir =
            workspace.path().to_string_lossy().into();
        let path = workspace.path().join("cloud-project");
        std::fs::create_dir_all(path.join("public")).unwrap();
        std::fs::write(path.join("public/index.php"), "<?php").unwrap();
        let result = Operation::parse(
            "projects.add",
            &json!({"name":"cloud-project","path":"cloud-project"}),
        )
        .unwrap()
        .execute(&manager)
        .unwrap();
        assert_eq!(
            PathBuf::from(result["projects"][0]["path"].as_str().unwrap()),
            dunce::canonicalize(&path).unwrap()
        );
        assert!(!home.path().join("www/cloud-project").exists());
        let error = Operation::parse("projects.add", &json!({"name":"duplicate","path":path}))
            .unwrap()
            .execute(&manager)
            .unwrap_err();
        assert!(error.is::<crate::projects::ProjectError>());
        assert!(error.to_string().contains("zaten kayıtlı"));
    }

    #[test]
    fn folder_only_add_rejects_missing_folders_traversal_and_non_laravel_roots() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        for folder in [
            "",
            ".",
            "..",
            "../outside",
            r"..\outside",
            "C:relative",
            "folder/child",
            "folder.",
            "missing",
        ] {
            let error = Operation::parse("projects.add", &json!({"name":"demo","path":folder}))
                .unwrap()
                .execute(&manager)
                .unwrap_err();
            assert!(
                error.is::<crate::projects::ProjectError>(),
                "{folder}: {error}"
            );
        }
        std::fs::create_dir(home.path().join("www/empty")).unwrap();
        let error = Operation::parse("projects.add", &json!({"name":"demo","path":"empty"}))
            .unwrap()
            .execute(&manager)
            .unwrap_err();
        assert!(error.to_string().contains("public/index.php"));
        assert!(manager.config.lock().unwrap().projects.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn relative_project_folder_cannot_escape_through_symlink() {
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        std::os::unix::fs::symlink(outside.path(), home.path().join("www/outside")).unwrap();
        assert!(manager
            .cloud_project_path("outside".into())
            .unwrap_err()
            .to_string()
            .contains("dışında"));
    }

    #[test]
    fn cloud_create_defaults_parent_and_rejects_existing_target_before_composer() {
        let home = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let operation = Operation::parse("projects.create", &json!({"name":"demo"})).unwrap();
        assert!(matches!(
            operation,
            Operation::Create(Create { parent: None, .. })
        ));
        assert_eq!(
            manager.cloud_project_parent(),
            dunce::canonicalize(home.path().join("www")).unwrap()
        );
        manager.config.lock().unwrap().settings.projects_dir =
            workspace.path().to_string_lossy().into();
        std::fs::create_dir(workspace.path().join("demo")).unwrap();
        std::fs::write(workspace.path().join("demo/keep.txt"), "keep").unwrap();
        let error = operation.execute(&manager).unwrap_err();
        assert!(
            error.to_string().contains("Hedef klasör zaten var"),
            "{error}"
        );
        assert_eq!(
            std::fs::read_to_string(workspace.path().join("demo/keep.txt")).unwrap(),
            "keep"
        );
        let legacy = Operation::parse(
            "projects.create",
            &json!({"name":"demo","parent":workspace.path()}),
        )
        .unwrap();
        assert!(matches!(
            legacy,
            Operation::Create(Create {
                parent: Some(_),
                ..
            })
        ));
    }

    #[test]
    fn service_logs_reject_paths_and_bound_unicode_output() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        assert!(Operation::ServiceLog(ServiceLog {
            service: "../config".into()
        })
        .execute(&manager)
        .is_err());
        std::fs::write(
            home.path().join("logs/redis.log"),
            format!("{}son", "x".repeat(40000)),
        )
        .unwrap();
        let result = Operation::ServiceLog(ServiceLog {
            service: "redis".into(),
        })
        .execute(&manager)
        .unwrap();
        assert_eq!(result["service"], "redis");
        assert_eq!(result["text"].as_str().unwrap().chars().count(), 32000);
        assert!(result["text"].as_str().unwrap().ends_with("son"));
        assert_eq!(result["truncated"], true);
    }
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
