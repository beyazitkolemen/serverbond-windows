//! A single, restart-safe setup request shared by Cloud, desktop and local API.
//! Only directories owned by this request are eligible for cleanup.
use crate::{
    model::{validate_slug, Project},
    project_workflow::ProjectTemplate,
    projects::ProjectError,
    Manager,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupRequest {
    pub request_id: String,
    pub source: SetupSource,
    pub name: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub branch: String,
    pub php_version: String,
    pub install_dependencies: bool,
    pub composer: bool,
    pub build: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SetupSource {
    Laravel,
    Github,
    Git,
    Existing,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupJournal {
    fingerprint: String,
    status: String,
    project: Option<Project>,
}

impl SetupRequest {
    fn validate(&self) -> Result<()> {
        uuid::Uuid::parse_str(&self.request_id)?;
        validate_slug(&self.name)?;
        crate::model::php_package(&self.php_version)?;
        crate::release::validate_git_branch(&self.branch)?;
        ensure!(self.location.len() <= 2048, "Kaynak adresi çok uzun.");
        match self.source {
            SetupSource::Laravel => {
                ensure!(
                    crate::model::php_supports_laravel12(&self.php_version),
                    "Yeni Laravel için PHP 8.2 veya üzeri gerekli."
                );
            }
            SetupSource::Github => {
                crate::github::parse_github_repository(&self.location)?;
            }
            SetupSource::Git => {
                ensure!(
                    self.location.starts_with("https://"),
                    "Kurulum için HTTPS Git adresi kullanın."
                );
                crate::github::validate_git_url(&self.location)?;
            }
            SetupSource::Existing => {
                ensure!(!self.location.is_empty(), "Proje klasörü gerekli.");
            }
        }
        Ok(())
    }
    fn fingerprint(&self) -> Result<String> {
        Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(self)?)))
    }
}

impl Manager {
    pub fn setup_preflight(&self, input: &SetupRequest) -> Result<Value> {
        input.validate()?;
        let parent = self.cloud_project_parent();
        let package = crate::model::php_package(&input.php_version)?;
        let php = crate::install::validate_installation(
            &self.home.join("bin/php").join(&input.php_version),
            &package,
        )
        .is_ok();
        let existing_path = (input.source == SetupSource::Existing)
            .then(|| self.cloud_project_path(input.location.clone().into()).ok())
            .flatten()
            .and_then(|path| dunce::canonicalize(path).ok());
        let source_ok = match input.source {
            SetupSource::Existing => existing_path
                .as_ref()
                .is_some_and(|p| p.join("public/index.php").is_file()),
            _ => !parent.join(&input.name).exists(),
        };
        let config = self.config.lock().unwrap_or_else(|e| e.into_inner());
        let name_ok =
            !config.projects.iter().any(|p| {
                p.name == input.name || existing_path.as_ref().is_some_and(|path| *path == p.path)
            }) && !crate::product::is_phpmyadmin_host(&config.settings.project_host(&input.name));
        drop(config);
        let mut checks = vec![
            json!({"id":"workspace","ok": parent.is_dir() && tempfile::NamedTempFile::new_in(&parent).is_ok(),"installable":false}),
            json!({"id":"destination","ok": source_ok && name_ok,"installable":false}),
            json!({"id":"disk","ok": crate::storage::require_space(&parent, 512 * 1024 * 1024).is_ok(),"installable":false}),
            json!({"id":"php","ok":php,"installable":true}),
        ];
        if input.composer || input.source == SetupSource::Laravel {
            checks.push(json!({"id":"composer","ok":self.executable("composer").is_ok(),"installable":true}));
        }
        if matches!(input.source, SetupSource::Git | SetupSource::Github) {
            checks.push(
                json!({"id":"git","ok":crate::release::git_program().is_ok(),"installable":false}),
            );
        }
        if input.build {
            checks.push(json!({"id":"node","ok":self.node_state().installed,"installable":true}));
        }
        let ready = checks
            .iter()
            .all(|c| c["ok"] == true || (input.install_dependencies && c["installable"] == true));
        Ok(
            json!({"ready":ready,"phpVersion":input.php_version,"checks":checks,"sourceInspectionPending": input.source != SetupSource::Existing || input.composer || input.build}),
        )
    }

    pub fn setup_project(&self, input: SetupRequest) -> Result<Project> {
        let _guard = self.gate()?;
        input.validate()?;
        let directory = self.home.join("config/setup");
        std::fs::create_dir_all(&directory)?;
        let journal_path = directory.join(format!("{}.json", input.request_id));
        let fingerprint = input.fingerprint()?;
        if journal_path.exists() {
            let journal: SetupJournal =
                serde_json::from_slice(&crate::storage::read_limited(&journal_path, 64 * 1024)?)?;
            ensure!(
                journal.fingerprint == fingerprint,
                "Aynı kurulum kimliği farklı ayarlarla kullanılamaz."
            );
            if journal.status == "succeeded" {
                return journal.project.context("Kurulum kaydı eksik.");
            }
            return Err(ProjectError("Önceki kurulum tamamlanmamış. Dosyaları ve işlem sonucunu kontrol edip yeni bir kurulum başlatın.").into());
        }
        let preflight = self.setup_preflight(&input)?;
        if preflight["ready"] != true {
            return Err(ProjectError("Kurulum ön kontrolündeki eksikleri giderin.").into());
        }
        let mut journal = SetupJournal {
            fingerprint,
            status: "running".into(),
            project: None,
        };
        crate::storage::atomic_write(&journal_path, serde_json::to_vec(&journal)?)?;
        let result = self.setup_project_inner(&input);
        match &result {
            Ok(project) => {
                journal.status = "succeeded".into();
                journal.project = Some(project.clone());
            }
            Err(_) => journal.status = "failed".into(),
        }
        crate::storage::atomic_write(&journal_path, serde_json::to_vec(&journal)?)?;
        result
    }

    fn setup_project_inner(&self, input: &SetupRequest) -> Result<Project> {
        self.cloud_stage("preflight");
        let php = crate::model::php_package(&input.php_version)?;
        if input.install_dependencies {
            self.check_install_requirements()?;
            self.cloud_stage("dependencies");
            if crate::install::validate_installation(
                &self.home.join("bin/php").join(&input.php_version),
                &php,
            )
            .is_err()
            {
                self.install_package(&php, false)?;
            }
            if (input.composer || input.source == SetupSource::Laravel)
                && self.executable("composer").is_err()
            {
                self.install_package(&self.package("composer")?, false)?;
            }
            if input.build && !self.node_state().installed {
                self.install_tool(crate::node::ID)?;
            }
        }
        let parent = self.cloud_project_parent();
        let staging = if input.source == SetupSource::Existing {
            None
        } else {
            Some(
                tempfile::Builder::new()
                    .prefix(".serverbond-setup-")
                    .tempdir_in(&parent)?,
            )
        };
        let path = match &staging {
            Some(dir) => dir.path().join("source"),
            None => self.cloud_project_path(input.location.clone().into())?,
        };
        let mut project = Project {
            id: uuid::Uuid::new_v4().to_string(),
            name: input.name.clone(),
            path: path.clone(),
            host: format!("{}.localhost", input.name),
            php_version: input.php_version.clone(),
            workers: vec![],
            schedule: Default::default(),
            release: Default::default(),
        };
        if input.source == SetupSource::Laravel {
            std::fs::create_dir(&path)?;
            self.cloud_stage("composer");
            let mut cmd = self.project_php_command(&project)?;
            cmd.arg(self.executable("composer")?).args([
                "create-project",
                "--prefer-dist",
                "--no-interaction",
                "--no-progress",
                "--no-install",
                "--no-scripts",
                "laravel/laravel:^12.0",
                ".",
            ]);
            self.setup_run(
                cmd,
                &input.php_version,
                "Composer proje indirme işlemi başarısız.",
            )?;
        } else if matches!(input.source, SetupSource::Github | SetupSource::Git) {
            self.cloud_stage("clone");
            let url = if input.source == SetupSource::Github {
                let repo = crate::github::parse_github_repository(&input.location)?;
                format!("https://github.com/{}/{}.git", repo.owner, repo.name)
            } else {
                input.location.clone()
            };
            self.clone_git_repository(&url, &path, &input.branch)
                .map_err(|_| {
                    ProjectError("Depo alınamadı. GitHub erişimini ve seçili dalı kontrol edin.")
                })?;
        }
        if !path.join("public/index.php").is_file() {
            return Err(
                ProjectError("Kaynak Laravel kökü değil; public/index.php bulunamadı.").into(),
            );
        }
        if input.composer || input.source == SetupSource::Laravel {
            self.cloud_stage("composer");
            if path.join("composer.lock").is_file() {
                let mut check = self.project_php_command(&project)?;
                check.arg(self.executable("composer")?).args([
                    "check-platform-reqs",
                    "--lock",
                    "--no-dev",
                ]);
                self.setup_run(
                    check,
                    &input.php_version,
                    "Seçili PHP veya uzantılar composer.lock gereksinimlerini karşılamıyor.",
                )?;
            }
            let mut cmd = self.project_php_command(&project)?;
            cmd.arg(self.executable("composer")?)
                .args(crate::release::composer_install_args(true));
            self.setup_run(
                cmd,
                &input.php_version,
                "Composer kurulumu başarısız. Seçili PHP ve uzantıları kontrol edin.",
            )?;
        }
        if input.build {
            self.cloud_stage("build");
            self.build_project_assets(&project, Duration::from_secs(600))?;
        }
        self.cloud_stage("register");
        if staging.is_some() {
            let destination = parent.join(&input.name);
            ensure!(
                !destination.exists(),
                "Hedef klasör başka bir işlem tarafından oluşturulmuş."
            );
            std::fs::rename(&path, &destination)?;
            project.path = destination;
        }
        // Register with explicit settings: never silently create a name-derived database.
        let template = ProjectTemplate {
            php_version: input.php_version.clone(),
            database: false,
            queue: false,
            schedule: false,
            branch: input.branch.clone(),
            migrate: false,
            build: input.build,
        };
        let registered = self.register_project(input.name.clone(), project.path, Some(&template))
            .map_err(|_| ProjectError("Kaynak hazırlandı ancak proje kaydı tamamlanamadı. Hedef klasör korundu; klasörden ekleyebilir veya Windows tanılamasını inceleyebilirsiniz."))?;
        self.cloud_stage("completed");
        Ok(registered)
    }

    pub(crate) fn build_project_assets(&self, project: &Project, timeout: Duration) -> Result<()> {
        ensure!(
            project.path.join("package-lock.json").is_file(),
            "Frontend build için package-lock.json gerekli."
        );
        let package: Value = serde_json::from_slice(&crate::storage::read_limited(
            &project.path.join("package.json"),
            256 * 1024,
        )?)?;
        ensure!(
            package["scripts"]["build"].is_string(),
            "package.json build komutu bulunamadı."
        );
        let node_dir = self.tool_directory(crate::node::ID)?;
        let deadline = Instant::now() + timeout;
        for args in [vec!["ci", "--no-audit", "--no-fund"], vec!["run", "build"]] {
            let mut cmd = crate::process::command(node_dir.join("node.exe"));
            cmd.current_dir(&project.path)
                .arg(node_dir.join("node_modules/npm/bin/npm-cli.js"))
                .args(args);
            self.setup_run_timeout(
                cmd,
                &project.php_version,
                "Frontend build başarısız. Kurulum günlüğünü kontrol edin.",
                deadline
                    .checked_duration_since(Instant::now())
                    .context("Derleme zaman sınırı aşıldı.")?,
            )?;
        }
        Ok(())
    }

    fn setup_run(
        &self,
        cmd: std::process::Command,
        version: &str,
        message: &'static str,
    ) -> Result<()> {
        self.setup_run_timeout(cmd, version, message, Duration::from_secs(600))
    }

    fn setup_run_timeout(
        &self,
        mut cmd: std::process::Command,
        version: &str,
        message: &'static str,
        timeout: Duration,
    ) -> Result<()> {
        // Scripts run with exactly the selected project's PHP, not the global default.
        let php_dir = self.home.join("bin/php").join(version);
        let mut paths = vec![php_dir.clone()];
        cmd.env("PHPRC", self.write_php_config_for(version)?)
            .env("PHP_INI_SCAN_DIR", "")
            .env("SERVERBOND_PHP_EXT", php_dir.join("ext"));
        // project_php_command already supplies PHPRC; PATH must include its executable directory.
        if let Some(parent) = Path::new(cmd.get_program()).parent().filter(|p| p.is_dir()) {
            paths.push(parent.to_path_buf());
        }
        if let Ok(node) = self.tool_directory(crate::node::ID) {
            paths.push(node);
        }
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        cmd.env("PATH", std::env::join_paths(paths)?)
            .env("COMPOSER_NO_INTERACTION", "1");
        let mut child =
            crate::process::ManagedChild::spawn(cmd, &self.home.join("logs/setup.log"))?;
        child
            .wait_timeout(timeout)
            .map_err(|_| ProjectError(message))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(manager: &Manager) -> SetupRequest {
        SetupRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            source: SetupSource::Existing,
            name: "shop".into(),
            location: "shop".into(),
            branch: "main".into(),
            php_version: manager.package("php").unwrap().version,
            install_dependencies: false,
            composer: false,
            build: false,
        }
    }
    #[test]
    fn preflight_rejects_missing_sources_and_does_not_register_or_install() {
        let home = tempfile::tempdir().unwrap();
        let m = Manager::new(home.path().into()).unwrap();
        let input = request(&m);
        assert_eq!(m.setup_preflight(&input).unwrap()["ready"], false);
        assert!(m.setup_project(input).is_err());
        assert!(m.snapshot().unwrap().projects.is_empty());
    }
    #[test]
    fn setup_identity_cannot_be_reused_with_changed_parameters() {
        let home = tempfile::tempdir().unwrap();
        let m = Manager::new(home.path().into()).unwrap();
        let mut input = request(&m);
        std::fs::create_dir_all(home.path().join("config/setup")).unwrap();
        let journal = SetupJournal {
            fingerprint: input.fingerprint().unwrap(),
            status: "failed".into(),
            project: None,
        };
        crate::storage::atomic_write(
            &home
                .path()
                .join(format!("config/setup/{}.json", input.request_id)),
            serde_json::to_vec(&journal).unwrap(),
        )
        .unwrap();
        input.name = "other".into();
        assert!(m
            .setup_project(input)
            .unwrap_err()
            .to_string()
            .contains("farklı"));
    }

    #[test]
    fn preflight_rejects_registered_folder_under_another_name_before_mutation() {
        let home = tempfile::tempdir().unwrap();
        let m = Manager::new(home.path().into()).unwrap();
        let root = m.cloud_project_parent().join("shop");
        std::fs::create_dir_all(root.join("public")).unwrap();
        std::fs::write(root.join("public/index.php"), "<?php").unwrap();
        std::fs::write(root.join(".env"), "DB_DATABASE=keep_this").unwrap();
        m.add_project("registered".into(), root.clone()).unwrap();
        let mut input = request(&m);
        input.install_dependencies = true;
        let report = m.setup_preflight(&input).unwrap();
        assert_eq!(report["ready"], false);
        assert!(m.setup_project(input).unwrap_err().is::<ProjectError>());
        assert_eq!(m.snapshot().unwrap().projects.len(), 1);
        assert_eq!(
            std::fs::read_to_string(root.join(".env")).unwrap(),
            "DB_DATABASE=keep_this"
        );
        assert!(!home.path().join("logs/setup.log").exists());
    }
}
