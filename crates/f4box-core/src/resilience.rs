use crate::{
    model::{php_package, validate_slug, Config},
    storage, Manager,
};
use anyhow::{bail, Context, Result};
use std::{
    collections::HashSet,
    path::Path,
    sync::{atomic::Ordering, MutexGuard, TryLockError},
};

const RESTART: &str = "Beklenmeyen iç hata algılandı. Yeni işlemler durduruldu; F4Box'tan Çıkış yapıp uygulamayı yeniden açın.";

pub(crate) fn load_config(path: &Path) -> Result<(Config, bool)> {
    let mut config: Config = serde_json::from_slice(&storage::read_limited(path, 2 * 1024 * 1024)?)
        .context("config.json okunamadı; mevcut dosya korunuyor.")?;
    config.settings.validate()?;
    php_package(&config.php_version)?;
    if config.projects.len() > 1000 {
        bail!("Proje sayısı 1000 sınırını aşıyor.");
    }
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    let mut migrated = false;
    for project in &mut config.projects {
        validate_slug(&project.name)?;
        uuid::Uuid::parse_str(&project.id).context("Geçersiz proje kimliği.")?;
        if !ids.insert(project.id.to_lowercase()) || !names.insert(project.name.clone()) {
            bail!("Yinelenen proje kimliği veya adı bulundu.");
        }
        if project.path.as_os_str().is_empty() || !project.path.is_absolute() {
            bail!("Proje yolu mutlak ve dolu olmalı.");
        }
        if project.php_version.is_empty() {
            project.php_version = config.php_version.clone();
            migrated = true;
        }
        php_package(&project.php_version)?;
        crate::jobs::validate_project_jobs(&project.workers, &project.schedule)?;
        if project.host != config.settings.project_host(&project.name)
            || project.host == "phpmyadmin.f4box.localhost"
        {
            bail!("Geçersiz veya ayrılmış proje alan adı.");
        }
    }
    Ok((config, migrated))
}

impl Manager {
    pub fn restart_required(&self) -> bool {
        self.faulted.load(Ordering::Acquire)
            || self.operation.is_poisoned()
            || self.config.is_poisoned()
            || self.processes.is_poisoned()
            || self.project_ports.is_poisoned()
    }

    pub fn shutdown(&self) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        self.shutting_down.store(true, Ordering::Release);
        self.stop_inner()
    }

    pub fn is_busy(&self) -> bool {
        matches!(self.operation.try_lock(), Err(TryLockError::WouldBlock))
    }

    // Catch unwinding at command boundaries. Never resume mutations after a panic.
    pub fn contain<T>(&self, work: impl FnOnce() -> Result<T>) -> Result<T> {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
            Ok(result) => result,
            Err(_) => {
                self.faulted.store(true, Ordering::Release);
                self.log(RESTART);
                bail!(RESTART)
            }
        }
    }

    pub(crate) fn cleanup_gate(&self) -> Result<MutexGuard<'_, ()>> {
        match self.operation.try_lock() {
            Ok(guard) => Ok(guard),
            Err(TryLockError::Poisoned(error)) => Ok(error.into_inner()),
            Err(TryLockError::WouldBlock) => {
                bail!("Başka bir işlem devam ediyor. Tamamlanmasını bekleyin.")
            }
        }
    }

    pub fn recovery_issue(&self) -> Option<String> {
        if self.restart_required() {
            return Some(RESTART.into());
        }
        self.startup_issue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn recover_configuration(&self) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        if self.restart_required() {
            bail!(RESTART);
        }
        if self
            .startup_issue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_none()
        {
            bail!("Yapılandırma kurtarma gerekmiyor.");
        }
        let (config, _) = load_config(&self.home.join("config.last-good.json"))
            .context("Geçerli bir yapılandırma yedeği bulunamadı. Mevcut dosyalar korunuyor.")?;
        let damaged = self.home.join("config.json");
        // Preserve even oversized/corrupt input without reading it into memory.
        if damaged.exists() {
            let mut source = std::fs::File::open(&damaged)?;
            let preserved = self
                .home
                .join(format!("config.corrupt-{}.json", uuid::Uuid::new_v4()));
            storage::require_space(&self.home, source.metadata()?.len())?;
            let mut copy = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&preserved)?;
            std::io::copy(&mut source, &mut copy)?;
            copy.sync_all()?;
            drop(source);
            drop(copy);
        }
        storage::atomic_write(&damaged, serde_json::to_vec_pretty(&config)?)?;
        *self.config.lock().unwrap_or_else(|e| e.into_inner()) = config;
        *self.startup_issue.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.log("Son geçerli yapılandırma geri yüklendi. Varsa bozuk dosya ayrıca korundu.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shutdown_blocks_new_mutations_but_busy_shutdown_does_not_latch() {
        let dir = tempfile::tempdir().unwrap();
        let manager = Manager::new(dir.path().into()).unwrap();
        let operation = manager.gate().unwrap();
        assert!(manager.shutdown().is_err());
        drop(operation);
        assert!(manager.gate().is_ok());
        manager.shutdown().unwrap();
        assert!(manager
            .start("all")
            .unwrap_err()
            .to_string()
            .contains("kapanıyor"));
        assert!(manager
            .restart()
            .unwrap_err()
            .to_string()
            .contains("kapanıyor"));
        manager.stop("all").unwrap();
        assert!(!manager.snapshot().unwrap().any_running);
    }
    #[test]
    fn panic_does_not_poison_diagnostics_or_prevent_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let manager = Manager::new(dir.path().into()).unwrap();
        let result: Result<()> = manager.contain(|| {
            let _operation = manager.gate()?;
            let _config = manager.config.lock().unwrap();
            panic!("injected panic");
        });
        assert!(result.is_err());
        let state = manager.snapshot().unwrap();
        assert!(state.restart_required);
        assert!(!state.busy);
        assert!(manager.start("all").is_err());
        manager.stop("all").unwrap();
    }
    #[test]
    fn corrupt_startup_can_be_recovered_without_discarding_original() {
        let dir = tempfile::tempdir().unwrap();
        let manager = Manager::new(dir.path().into()).unwrap();
        let expected = std::fs::read(dir.path().join("config.json")).unwrap();
        drop(manager);
        std::fs::write(dir.path().join("config.json"), b"broken json").unwrap();
        assert!(Manager::new(dir.path().into()).is_err());
        let manager = Manager::open_recovering(dir.path().into()).unwrap();
        assert!(manager.snapshot().unwrap().recovery_issue.is_some());
        assert!(manager.start("all").is_err());
        manager.recover_configuration().unwrap();
        assert!(manager.recovery_issue().is_none());
        assert_eq!(
            std::fs::read(dir.path().join("config.json")).unwrap(),
            expected
        );
        let copy = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .find(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("config.corrupt-")
            })
            .unwrap();
        assert_eq!(std::fs::read(copy.path()).unwrap(), b"broken json");
    }
    #[test]
    fn bad_backup_never_overwrites_broken_config() {
        let dir = tempfile::tempdir().unwrap();
        drop(Manager::new(dir.path().into()).unwrap());
        std::fs::write(dir.path().join("config.json"), b"original damaged").unwrap();
        std::fs::write(dir.path().join("config.last-good.json"), b"bad backup").unwrap();
        let manager = Manager::open_recovering(dir.path().into()).unwrap();
        assert!(manager.recover_configuration().is_err());
        assert_eq!(
            std::fs::read(dir.path().join("config.json")).unwrap(),
            b"original damaged"
        );
        assert!(manager.recovery_issue().is_some());
    }

    #[test]
    fn missing_config_uses_recovery_instead_of_silent_reset() {
        let dir = tempfile::tempdir().unwrap();
        drop(Manager::new(dir.path().into()).unwrap());
        std::fs::remove_file(dir.path().join("config.json")).unwrap();
        let manager = Manager::open_recovering(dir.path().into()).unwrap();
        assert!(manager.recovery_issue().is_some());
        assert!(!dir.path().join("config.json").exists());
        manager.recover_configuration().unwrap();
        assert!(manager.recovery_issue().is_none());
    }

    #[test]
    fn existing_data_without_config_or_backup_is_never_treated_as_new_installation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("data/mysql-8.4")).unwrap();
        std::fs::write(dir.path().join("data/mysql-8.4/keep.ibd"), b"keep").unwrap();
        let manager = Manager::open_recovering(dir.path().into()).unwrap();
        assert!(manager.recovery_issue().is_some());
        assert!(manager.start("mysql").is_err());
        assert!(!dir.path().join("config.json").exists());
        assert!(!dir.path().join("config.last-good.json").exists());
        assert_eq!(
            std::fs::read(dir.path().join("data/mysql-8.4/keep.ibd")).unwrap(),
            b"keep"
        );
    }

    #[test]
    fn missing_mysql_data_never_regenerates_password_or_initializes_database() {
        let dir = tempfile::tempdir().unwrap();
        let manager = Manager::new(dir.path().into()).unwrap();
        let ready = dir.path().join("config/mysql-ready");
        let password = dir.path().join("config/mysql-password.dpapi");
        std::fs::write(&ready, b"8.4").unwrap();
        std::fs::write(&password, b"preserve encrypted original").unwrap();
        assert!(manager
            .start("mysql")
            .unwrap_err()
            .to_string()
            .contains("veri klasörü kayıp"));
        assert!(!dir.path().join("data/mysql-8.4").exists());
        assert_eq!(
            std::fs::read(password).unwrap(),
            b"preserve encrypted original"
        );
        assert_eq!(std::fs::read(ready).unwrap(), b"8.4");
        assert!(!manager.snapshot().unwrap().busy);
    }

    #[test]
    fn existing_database_without_password_is_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let manager = Manager::new(dir.path().into()).unwrap();
        let data = dir.path().join("data/mysql-8.4");
        std::fs::create_dir(&data).unwrap();
        std::fs::write(data.join("keep.ibd"), b"important database").unwrap();
        assert!(manager
            .start("mysql")
            .unwrap_err()
            .to_string()
            .contains("tutarsız"));
        assert_eq!(
            std::fs::read(data.join("keep.ibd")).unwrap(),
            b"important database"
        );
        assert!(!dir.path().join("config/mysql-password.dpapi").exists());
    }

    #[test]
    fn duplicate_projects_are_rejected_without_rewriting_config() {
        let dir = tempfile::tempdir().unwrap();
        drop(Manager::new(dir.path().into()).unwrap());
        let path = dir.path().join("config.json");
        let (mut config, _) = load_config(&path).unwrap();
        let project = crate::model::Project {
            id: uuid::Uuid::new_v4().to_string(),
            name: "demo".into(),
            host: "demo.localhost".into(),
            path: dir.path().join("demo"),
            php_version: config.php_version.clone(),
            workers: Vec::new(),
            schedule: Default::default(),
        };
        config.projects = vec![project.clone(), project];
        let bytes = serde_json::to_vec(&config).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(Manager::new(dir.path().into()).is_err());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }

    #[cfg(windows)]
    #[test]
    fn locked_config_preserves_disk_and_memory_when_save_fails() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        let manager = Manager::new(dir.path().into()).unwrap();
        let path = dir.path().join("config.json");
        let original = std::fs::read(&path).unwrap();
        // Permit reads and writes but deny deletion/atomic replacement, as AV/editor locks can.
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&path)
            .unwrap();
        let mut changed = manager.config.lock().unwrap().clone();
        changed.settings.start_on_launch = true;
        assert!(manager.save_config(&changed).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert!(!manager.snapshot().unwrap().settings.start_on_launch);
        drop(lock);
        manager.save_config(&changed).unwrap();
        assert!(manager.snapshot().unwrap().settings.start_on_launch);
    }
}
