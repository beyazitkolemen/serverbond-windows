//! ServerBond core: the `Manager` facade every front end (Tauri window, CLI,
//! local HTTP API) drives. It owns the data directory, the configuration, the
//! child processes and the operation gate; feature modules extend it with
//! `impl Manager` blocks. Nothing here talks to a window or a terminal.

pub mod api;
mod domain;
mod envfile;
mod github;
pub mod https;
pub mod install;
mod jobs;
pub mod legacy;
pub mod mail;
pub mod model;
pub mod node;
pub mod permissions;
mod phpmyadmin;
mod postgres;
pub mod preferences;
mod process;
pub mod product;
mod project_runtime;
mod projects;
mod redis;
mod release;
mod repository;
pub mod requirements;
mod resilience;
mod secrets;
mod services;
mod storage;
mod terminal;
pub mod tunnel;

pub use domain::{ComponentId, EnvironmentAction, GithubAction, ToolAction};
pub use envfile::ProjectEnv;
pub use product::NAME;
pub use release::{ProjectGitStatus, ReleaseRecord};
pub use repository::DataDir;

use anyhow::{bail, Context, Result};
use fs2::FileExt;
use model::*;
use process::ManagedChild;
use std::{
    collections::{HashMap, VecDeque},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Mutex, MutexGuard},
};

pub struct Manager {
    pub home: PathBuf,
    config: Mutex<Config>,
    processes: Mutex<HashMap<String, ManagedChild>>,
    logs: Mutex<VecDeque<String>>,
    service_errors: Mutex<HashMap<String, String>>,
    project_ports: Mutex<HashMap<String, u16>>,
    operation: Mutex<()>,
    faulted: AtomicBool,
    shutting_down: AtomicBool,
    startup_issue: Mutex<Option<String>>,
    api: api::ApiState,
    _lock: File,
}

impl Manager {
    pub fn default_home() -> PathBuf {
        product::default_home()
    }

    pub fn new(home: PathBuf) -> Result<Self> {
        Self::open_inner(home, false)
    }

    pub fn open_recovering(home: PathBuf) -> Result<Self> {
        Self::open_inner(home, true)
    }

    fn open_inner(home: PathBuf, recover: bool) -> Result<Self> {
        fs::create_dir_all(&home)?;
        let home = dunce::canonicalize(home)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(home.join("manager.lock"))?;
        lock.try_lock_exclusive().context(format!(
            "Bu veri klasörü başka bir {} penceresinde kullanılıyor.",
            product::NAME
        ))?;
        for dir in [
            "bin", "cache", "config", "data", "logs", "www", "projects", "welcome", "backups",
        ] {
            fs::create_dir_all(home.join(dir))?;
        }
        let mut existing_data = false;
        for dir in ["bin", "data", "www", "projects"] {
            existing_data |= fs::read_dir(home.join(dir))?.next().transpose()?.is_some();
        }
        let first_run = !home.join("config.json").try_exists()?
            && !home.join("config.last-good.json").try_exists()?
            && !existing_data;
        let mut startup_issue = None;
        let mut migrated = false;
        let config = if !first_run {
            match resilience::load_config(&home.join("config.json")) {
                Ok((config, changed)) => {
                    migrated = changed;
                    config
                }
                Err(error) if recover => {
                    startup_issue = Some(format!("Yapılandırma açılamadı: {error:#}. Dosyalarınız korunuyor; kurtarma tamamlanana kadar yeni işlemler engellendi."));
                    Config::default()
                }
                Err(error) => return Err(error),
            }
        } else {
            let mut config = Config::default();
            // Choose available initial ports without touching other developer tools.
            let mut selected_ports = std::collections::HashSet::new();
            selected_ports.insert(config.settings.web.https_port);
            selected_ports.insert(config.settings.mail.smtp_port);
            selected_ports.insert(config.settings.mail.web_port);
            for port in [
                &mut config.settings.web_port,
                &mut config.settings.mysql_port,
                &mut config.settings.php_port,
            ] {
                while selected_ports.contains(port) || services::port_free(*port).is_err() {
                    *port = port
                        .checked_add(1)
                        .context("Kullanılabilir port bulunamadı.")?;
                }
                selected_ports.insert(*port);
            }
            config.settings.validate()?;
            config
        };
        fs::write(
            home.join("welcome/index.html"),
            format!(
                "<!doctype html><html lang=\"tr\"><meta charset=\"utf-8\"><title>{name}</title><style>body{{font:20px system-ui;max-width:640px;margin:12vh auto;padding:24px;color:#20282f}}strong{{color:#008653}}</style><h1><strong>{name}</strong> çalışıyor.</h1><p>Bu Windows makinesinde Laravel üretimi hazır. Uygulamadan projenizi ekleyin.</p></html>",
                name = product::NAME
            ),
        )?;
        let manager = Self {
            home,
            config: Mutex::new(config),
            processes: Mutex::new(HashMap::new()),
            logs: Mutex::new(VecDeque::new()),
            service_errors: Mutex::new(HashMap::new()),
            project_ports: Mutex::new(HashMap::new()),
            operation: Mutex::new(()),
            faulted: AtomicBool::new(false),
            shutting_down: AtomicBool::new(false),
            startup_issue: Mutex::new(startup_issue),
            api: api::ApiState::default(),
            _lock: lock,
        };
        manager.log(if manager.recovery_issue().is_some() {
            "Yapılandırma kurtarma gerekiyor. Mevcut dosyalar korundu; yeni işlemler engellendi."
                .to_string()
        } else {
            format!("{} hazır. Kurulum başlatılabilir.", product::NAME)
        });
        if manager.recovery_issue().is_none() {
            if first_run || migrated {
                let initial = manager
                    .config
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                manager.save_config(&initial)?;
            } else if !manager.home.join("config.last-good.json").is_file() {
                storage::atomic_write(
                    &manager.home.join("config.last-good.json"),
                    storage::read_limited(&manager.home.join("config.json"), 2 * 1024 * 1024)?,
                )?;
            }
        }
        Ok(manager)
    }

    fn gate(&self) -> Result<MutexGuard<'_, ()>> {
        let guard = self.cleanup_gate()?;
        if self
            .shutting_down
            .load(std::sync::atomic::Ordering::Acquire)
        {
            bail!("{} kapanıyor; yeni işlem başlatılamaz.", product::NAME);
        }
        if let Some(issue) = self.recovery_issue() {
            bail!(issue);
        }
        Ok(guard)
    }

    pub fn log(&self, message: impl AsRef<str>) {
        let line = format!(
            "[{}] {}",
            chrono::Local::now().format("%H:%M:%S"),
            message.as_ref().chars().take(4096).collect::<String>()
        );
        let mut logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        {
            logs.push_back(line.clone());
            if logs.len() > 250 {
                logs.pop_front();
            }
        }
        let log_path = self.home.join(format!("logs/{}.log", product::LOG_ID));
        if log_path.metadata().is_ok_and(|m| m.len() > 4 * 1024 * 1024) {
            let previous = self
                .home
                .join(format!("logs/{}.previous.log", product::LOG_ID));
            if !previous.exists() || fs::remove_file(&previous).is_ok() {
                let _ = fs::rename(&log_path, previous);
            }
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
            let _ = writeln!(file, "{line}");
        }
    }

    fn package(&self, id: &str) -> Result<Package> {
        selected_catalog(
            &self
                .config
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .php_version,
        )?
        .into_iter()
        .find(|p| p.id == id)
        .context("Bilinmeyen bileşen.")
    }

    pub fn package_dir(&self, id: &str) -> Result<PathBuf> {
        let package = self.package(id)?;
        Ok(self.home.join("bin").join(id).join(package.version))
    }

    pub fn executable(&self, id: &str) -> Result<PathBuf> {
        let package = self.package(id)?;
        let directory = self.package_dir(id)?;
        install::validate_installation(&directory, &package).with_context(|| {
            format!("{id} kullanıma hazır değil. Bileşenler ekranını kontrol edin.")
        })?;
        let path = directory.join(package.executable);
        Ok(path)
    }

    pub fn tool_executable(&self, id: &str) -> Result<PathBuf> {
        let package = tool_package(id)?;
        let directory = self.home.join("bin").join(id).join(&package.version);
        install::validate_installation(&directory, &package)
            .with_context(|| format!("{} kullanıma hazır değil.", package.name))?;
        Ok(directory.join(package.executable))
    }

    pub(crate) fn install_tool(&self, id: &str) -> Result<()> {
        let package = tool_package(id)?;
        self.check_install_requirements()?;
        install::install(&self.home, &package, |line| self.log(line))?;
        self.refresh_permissions_quietly();
        Ok(())
    }

    pub(crate) fn repair_tool(&self, id: &str) -> Result<()> {
        let package = tool_package(id)?;
        install::repair(&self.home, &package, |line| self.log(line))?;
        self.service_errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        self.refresh_permissions_quietly();
        Ok(())
    }

    fn package_status(&self, package: Package, process: Option<&ManagedChild>) -> PackageStatus {
        let health = install::health(&self.home, &package);
        PackageStatus {
            installed: health.installed,
            running: process.is_some(),
            pid: process.map(|p| p.child.id()),
            repairable: health.repairable,
            issue: health.issue.or_else(|| {
                self.service_errors
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(&package.id)
                    .cloned()
            }),
            package,
        }
    }

    pub(crate) fn tool_health(&self, id: &str) -> install::InstallHealth {
        let package = tool_package(id).expect("embedded tool package");
        let mut health = install::health(&self.home, &package);
        if health.issue.is_none() {
            health.issue = self
                .service_errors
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(id)
                .cloned();
        }
        health
    }

    /// Drop children that have exited. Queue workers that finished on purpose
    /// (status 0 after a reasonable uptime) are returned so the caller can start
    /// them again; everything else is reported as an unexpected stop.
    fn reap_exited_children(&self) -> Vec<String> {
        let mut planned = Vec::new();
        let mut processes = self.processes.lock().unwrap_or_else(|e| e.into_inner());
        processes.retain(|id, p| {
            let detail = match p.child.try_wait() {
                Ok(None) => return true,
                Ok(Some(exit))
                    if exit.success()
                        && jobs::is_queue_service_id(id)
                        && p.spawned_at.elapsed() >= jobs::PLANNED_EXIT_MIN_UPTIME =>
                {
                    planned.push(id.clone());
                    return false;
                }
                Ok(Some(exit)) => exit.to_string(),
                Err(error) => error.to_string(),
            };
            let message = format!("{id} beklenmedik şekilde kapandı ({detail}). Günlükleri kontrol edip yeniden başlatın.");
            self.log(&message);
            self.service_errors
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(id.clone(), message);
            false
        });
        planned
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        let planned = self.reap_exited_children();
        if !planned.is_empty()
            && !self
                .shutting_down
                .load(std::sync::atomic::Ordering::Acquire)
            && self.recovery_issue().is_none()
        {
            self.respawn_planned_workers(&planned);
        }
        let config = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let processes = self.processes.lock().unwrap_or_else(|e| e.into_inner());
        let packages = selected_catalog(&config.php_version)?
            .into_iter()
            .map(|package| {
                let process = processes.get(&package.id);
                self.package_status(package, process)
            })
            .collect();
        let tunnel = self.tunnel_state_with(&processes, config.settings.tunnel.auto_start);
        let mail = self.mail_state_with(&processes, &config.settings.mail);
        let postgres = self.postgres_state_with(&processes, &config.settings.postgres);
        let redis = self.redis_state_with(&processes, &config.settings.redis);
        let github = self.github_state();
        let node = self.node_state();
        Ok(Snapshot {
            packages,
            php_versions: php_versions()
                .into_iter()
                .map(|package| {
                    let process = processes
                        .get("php")
                        .filter(|_| package.version == config.php_version)
                        .or_else(|| {
                            config
                                .projects
                                .iter()
                                .filter(|p| p.php_version == package.version)
                                .find_map(|p| processes.get(&Self::project_service_id(&p.id)))
                        });
                    self.package_status(package, process)
                })
                .collect(),
            settings: config.settings,
            projects: config
                .projects
                .into_iter()
                .map(|project| {
                    // One guard per project: std mutexes are not reentrant.
                    let errors = self
                        .service_errors
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    let id = Self::project_service_id(&project.id);
                    let running = processes.contains_key(&id);
                    let pid = processes.get(&id).map(|p| p.child.id());
                    let issue = errors.get(&id).cloned().or_else(|| {
                        let package = php_package(&project.php_version).ok()?;
                        install::validate_installation(
                            &self.home.join("bin/php").join(&project.php_version),
                            &package,
                        )
                        .err()
                        .map(|e| format!("PHP {}: {e:#}", project.php_version))
                    });
                    let php_port = self
                        .project_ports
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(&project.id)
                        .copied();
                    let mut worker_states = Vec::with_capacity(project.workers.len());
                    for worker in &project.workers {
                        let mut pids = Vec::new();
                        let mut worker_issue = None;
                        for index in 0..worker.processes {
                            let sid = crate::jobs::queue_service_id(&project.id, &worker.id, index);
                            if let Some(child) = processes.get(&sid) {
                                pids.push(child.child.id());
                            }
                            if let Some(error) = errors.get(&sid) {
                                worker_issue = Some(error.clone());
                            }
                        }
                        worker_states.push(WorkerState {
                            id: worker.id.clone(),
                            running: pids.len() as u8,
                            pids,
                            issue: worker_issue,
                        });
                    }
                    let schedule_id = crate::jobs::schedule_service_id(&project.id);
                    let schedule_running = processes.contains_key(&schedule_id);
                    let schedule_pid = processes.get(&schedule_id).map(|p| p.child.id());
                    let schedule_issue = errors
                        .get(&schedule_id)
                        .or_else(|| errors.get(&format!("jobs-{}", project.id)))
                        .cloned();
                    drop(errors);
                    ProjectStatus {
                        project,
                        running,
                        pid,
                        php_port,
                        issue,
                        worker_states,
                        schedule_running,
                        schedule_pid,
                        schedule_issue,
                    }
                })
                .collect(),
            tunnel,
            mail,
            postgres,
            redis,
            github,
            node,
            permissions: self.permission_state(),
            logs: self
                .logs
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .cloned()
                .collect(),
            home: self.home.clone(),
            busy: self.is_busy(),
            recovery_issue: self.recovery_issue(),
            restart_required: self.restart_required(),
            any_running: !processes.is_empty(),
        })
    }

    pub fn install(&self, id: &str) -> Result<()> {
        let _guard = self.gate()?;
        self.check_install_requirements()?;
        let mut packages = if id == "all" {
            selected_catalog(
                &self
                    .config
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .php_version,
            )?
        } else {
            vec![self.package(id)?]
        };
        if id == "all" {
            let config = self
                .config
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            for project in config.projects {
                if !packages
                    .iter()
                    .any(|p| p.id == "php" && p.version == project.php_version)
                {
                    packages.push(php_package(&project.php_version)?);
                }
            }
        }
        for package in packages {
            install::install(&self.home, &package, |line| self.log(line))?;
        }
        if self.executable("php").is_ok() {
            self.write_php_config()?;
        }
        if id == "phpmyadmin" || id == "all" {
            self.write_phpmyadmin_config()?;
            if self
                .processes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get_mut("caddy")
                .is_some_and(|p| p.alive())
            {
                self.stop_service("caddy")?;
                self.start_caddy().context("phpMyAdmin kuruldu; web sunucusu yeniden başlatılamadı. Bileşenler ekranından tekrar başlatın.")?;
            }
        }
        self.log("Seçilen bileşenlerin kurulumu tamamlandı.");
        self.refresh_permissions_quietly();
        Ok(())
    }

    pub fn repair(&self, id: &str) -> Result<()> {
        let _guard = self.gate()?;
        let package = self.package(id)?;
        if self.snapshot()?.any_running {
            bail!("Onarmadan önce çalışan ortamı durdurun. MySQL verileri korunur.");
        }
        install::repair(&self.home, &package, |line| self.log(line))?;
        if id == "php" {
            self.write_php_config()?;
        }
        if id == "phpmyadmin" {
            self.write_phpmyadmin_config()?;
        }
        self.service_errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        self.log(format!(
            "{} onarıldı. Veri ve proje klasörleri korundu.",
            package.name
        ));
        self.refresh_permissions_quietly();
        Ok(())
    }

    fn save_config(&self, config: &Config) -> Result<()> {
        let layout = DataDir::new(&self.home);
        let path = layout.config();
        let bytes = serde_json::to_vec_pretty(config)?;
        if bytes.len() > 2 * 1024 * 1024 || config.projects.len() > 1000 {
            bail!("Yapılandırma boyutu veya proje sayısı sınırı aşıldı.");
        }
        let previous = if path.exists() {
            resilience::load_config(&path)?;
            storage::read_limited(&path, 2 * 1024 * 1024)?
        } else {
            bytes.clone()
        };
        storage::atomic_write(&layout.last_good(), previous)?;
        storage::atomic_write(&path, bytes)?;
        *self.config.lock().unwrap_or_else(|e| e.into_inner()) = config.clone();
        Ok(())
    }

    pub fn save_settings(&self, settings: Settings) -> Result<()> {
        let _guard = self.gate()?;
        settings.validate()?;
        let current = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .clone();
        let runtime = current.runtime_changed(&settings);
        let running = self.snapshot()?.any_running;
        if running && runtime {
            bail!("Portları ve sunucu ayarlarını değiştirmeden önce ortamı durdurun.");
        }
        if !running {
            for port in [settings.web_port, settings.mysql_port, settings.php_port] {
                services::port_free(port)?;
            }
            if settings.web.https {
                services::port_free(settings.web.https_port)?;
            }
            // The mail catcher only binds when it is started, so a busy port is worth
            // reporting here only when the environment will start it on its own.
            if settings.mail.auto_start {
                for port in [settings.mail.smtp_port, settings.mail.web_port] {
                    services::port_free(port)?;
                }
            }
            if settings.postgres.auto_start {
                services::port_free(settings.postgres.port)?;
            }
            if settings.redis.auto_start {
                services::port_free(settings.redis.port)?;
            }
        }
        let mut config = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        self.validate_installed_preferences(&settings)?;
        for project in &mut config.projects {
            project.host = settings.project_host(&project.name);
            if product::is_phpmyadmin_host(&project.host) {
                bail!("Proje adresi phpMyAdmin adresiyle çakışıyor.");
            }
        }
        // Keep the last successfully applied preferences independently of generated files.
        let previous = serde_json::to_vec_pretty(
            &self
                .config
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .settings,
        )?;
        let mut backup = tempfile::NamedTempFile::new_in(self.home.join("config"))?;
        backup.write_all(&previous)?;
        backup.as_file().sync_all()?;
        backup.persist(self.home.join("config/settings.previous.json"))?;
        config.settings = settings;
        self.save_config(&config)?;
        // Folder/launch preferences may be saved while project PHP workers run;
        // their ports must survive or the next Caddy restart cannot route them.
        if !running {
            self.project_ports
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
        }
        self.log(if runtime {
            "Sunucu ayarları kaydedildi."
        } else {
            "Çalışma alanı tercihleri kaydedildi."
        });
        Ok(())
    }

    pub fn read_log(&self, id: &str) -> Result<String> {
        if !Self::is_managed_log(id) {
            bail!("Geçersiz günlük.");
        }
        let path = self.home.join("logs").join(format!("{id}.log"));
        if !path.exists() {
            return Ok("Henüz günlük kaydı yok.".into());
        }
        use std::io::{Read, Seek, SeekFrom};
        let mut file = File::open(path)?;
        let size = file.metadata()?.len();
        file.seek(SeekFrom::Start(size.saturating_sub(64 * 1024)))?;
        let mut bytes = Vec::new();
        file.take(64 * 1024).read_to_end(&mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn credentials(&self) -> Result<String> {
        if !self.home.join("config/mysql-ready").is_file() {
            bail!("Bağlantı parolası henüz hazır değil. Önce MySQL'i başlatın.");
        }
        secrets::read(&self.home.join("config/mysql-password.dpapi"))
    }

    pub fn open_project(&self, id: &str) -> Result<()> {
        let config = self.config.lock().unwrap_or_else(|e| e.into_inner());
        let project = config
            .projects
            .iter()
            .find(|p| p.id == id)
            .context("Proje bulunamadı.")?;
        let url = config.settings.site_url(&project.host);
        drop(config);
        process::command("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(url)
            .spawn()?;
        Ok(())
    }

    pub fn open_home(&self) -> Result<()> {
        process::command("explorer.exe").arg(&self.home).spawn()?;
        Ok(())
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        let _ = self.stop_inner();
    }
}

fn portable_path(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches("\\\\?\\")
        .replace('\\', "/")
}
