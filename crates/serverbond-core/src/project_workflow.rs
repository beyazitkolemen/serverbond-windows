//! Bounded preflight and project health observations; templates never edit .env.
use crate::{
    model::{Project, ProjectSchedule, QueueWorker},
    Manager,
};
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::Path, time::Duration};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectTemplate {
    pub php_version: String,
    pub database: bool,
    pub queue: bool,
    pub schedule: bool,
    pub branch: String,
    pub migrate: bool,
}
impl ProjectTemplate {
    pub fn validate(&self) -> Result<()> {
        crate::model::php_package(&self.php_version)?;
        ensure!(
            crate::model::php_supports_laravel12(&self.php_version),
            "Laravel 12 için PHP 8.2 veya üzeri gerekli."
        );
        let mut project = Project {
            id: uuid::Uuid::new_v4().to_string(),
            name: "check".into(),
            path: Default::default(),
            host: "check.localhost".into(),
            php_version: self.php_version.clone(),
            workers: vec![],
            schedule: Default::default(),
            release: Default::default(),
        };
        self.apply(&mut project);
        crate::release::validate_project_release(&project.release)?;
        crate::jobs::validate_project_jobs(&project.workers, &project.schedule)?;
        Ok(())
    }
    pub fn apply(&self, project: &mut Project) {
        project.php_version = self.php_version.clone();
        project.schedule = ProjectSchedule {
            enabled: self.schedule,
            auto_start: false,
        };
        if self.queue {
            project.workers = vec![QueueWorker {
                id: uuid::Uuid::new_v4().to_string(),
                auto_start: false,
                ..Default::default()
            }];
        }
        project.release.branch = self.branch.clone();
        project.release.migrate = self.migrate;
    }
}
impl Manager {
    pub(crate) fn project_preflight_at(&self, parent: &Path, version: &str) -> Result<Value> {
        let package = crate::model::php_package(version)?;
        let php = crate::model::php_supports_laravel12(version)
            && crate::install::validate_installation(
                &self.home.join("bin/php").join(version),
                &package,
            )
            .is_ok();
        // A temporary file tests the actual account's write permission and is removed on drop.
        let writable = parent.is_dir() && tempfile::NamedTempFile::new_in(parent).is_ok();
        let composer = self.executable("composer").is_ok();
        let git = crate::release::git_program().is_ok();
        let state = self.snapshot()?;
        let web = state
            .packages
            .iter()
            .any(|p| p.package.id == "caddy" && p.running)
            || crate::services::port_free(state.settings.web_port).is_ok();
        Ok(
            json!({"ready":php && composer && writable,"phpVersion":version,"checks":[
                {"id":"php","ok":php,"required":true}, {"id":"composer","ok":composer,"required":true},
                {"id":"workspace","ok":writable,"required":true}, {"id":"git","ok":git,"required":false},
                {"id":"webPort","ok":web,"required":false}
            ]}),
        )
    }
    pub(crate) fn project_preflight(&self) -> Result<Value> {
        self.project_preflight_at(&self.cloud_project_parent(), &self.package("php")?.version)
    }
    pub(crate) fn project_health(&self, id: &str) -> Result<Value> {
        uuid::Uuid::parse_str(id)?;
        let state = self.snapshot()?;
        let project = state
            .projects
            .iter()
            .find(|p| p.project.id == id)
            .context("Proje bulunamadı.")?;
        // Always loopback, no proxy or redirects: project metadata cannot initiate arbitrary requests.
        let web = reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(3))
            .build()?
            .get(format!("http://127.0.0.1:{}/", state.settings.web_port))
            .header("Host", &project.project.host)
            .send()
            .ok()
            .map(|r| r.status().as_u16());
        let workers_expected: u32 = project
            .project
            .workers
            .iter()
            .filter(|w| w.enabled)
            .map(|w| u32::from(w.processes))
            .sum();
        let workers_running: u32 = project
            .worker_states
            .iter()
            .map(|w| u32::from(w.running))
            .sum();
        Ok(
            json!({"projectId":id,"phpRunning":project.running,"httpStatus":web,
            "mysqlRunning":state.packages.iter().any(|p| p.package.id=="mysql" && p.running),
            "workersExpected":workers_expected,"workersRunning":workers_running,
            "scheduleEnabled":project.project.schedule.enabled,"scheduleRunning":project.schedule_running,
            "hasIssue":project.issue.is_some() || project.schedule_issue.is_some() || project.worker_states.iter().any(|w| w.issue.is_some()),
            "checkedAt":chrono::Utc::now().to_rfc3339()}),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preflight_blocks_missing_dependencies_without_creating_project() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let report = manager.project_preflight().unwrap();
        assert_eq!(report["ready"], false);
        assert!(manager
            .create_project("new-project".into(), manager.cloud_project_parent())
            .is_err());
        assert!(!manager.cloud_project_parent().join("new-project").exists());
    }
    #[test]
    fn template_rejects_invalid_release_and_leaves_autostart_off() {
        let mut template = ProjectTemplate {
            php_version: String::new(),
            database: false,
            queue: true,
            schedule: true,
            branch: "main".into(),
            migrate: true,
        };
        // Resolve a supported PHP release from the manager rather than hard coding catalog updates.
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        template.php_version = manager.package("php").unwrap().version;
        assert!(template.validate().is_ok());
        let root = home.path().join("www/template-test");
        std::fs::create_dir_all(root.join("public")).unwrap();
        std::fs::write(root.join("public/index.php"), "<?php").unwrap();
        std::fs::write(root.join(".env"), "DB_DATABASE=preserved").unwrap();
        let mut project = manager
            .add_project("template-test".into(), root.clone())
            .unwrap();
        let original = manager.package("php").unwrap().version;
        template.apply(&mut project);
        assert_eq!(project.workers.len(), 1);
        assert!(!project.workers[0].auto_start);
        assert!(!project.schedule.auto_start);
        assert_eq!(manager.package("php").unwrap().version, original);
        assert_eq!(
            std::fs::read_to_string(root.join(".env")).unwrap(),
            "DB_DATABASE=preserved"
        );
        template.branch = "--unsafe".into();
        assert!(template.validate().is_err());
    }
}
