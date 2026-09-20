use crate::{
    model::{Config, Project},
    portable_path,
    process::command,
    Manager,
};
use anyhow::{Context, Result};
use std::net::TcpListener;

impl Manager {
    pub(crate) fn project_service_id(id: &str) -> String {
        format!("php-project-{id}")
    }

    pub(crate) fn is_project_service_id(id: &str) -> bool {
        id.strip_prefix("php-project-")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
    }

    pub(crate) fn rollback_new_services(&self, baseline: &[String]) {
        let mut ids = self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .filter(|id| !baseline.contains(id))
            .cloned()
            .collect::<Vec<_>>();
        ids.sort_by_key(|id| if id == "caddy" { 0 } else { 1 });
        for id in ids {
            let _ = self.stop_service(&id);
        }
    }

    pub(crate) fn stop_project_workers(&self) -> Result<()> {
        let ids = self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .filter(|id| Self::is_project_service_id(id))
            .cloned()
            .collect::<Vec<_>>();
        for id in ids {
            self.stop_service(&id)?;
        }
        Ok(())
    }

    pub(crate) fn start_project_workers(&self) -> Result<()> {
        let projects = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .projects
            .clone();
        let before = self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for project in projects {
            if let Err(error) = self.start_project_php(&project) {
                self.rollback_new_services(&before);
                return Err(error);
            }
        }
        Ok(())
    }

    fn start_project_php(&self, project: &Project) -> Result<()> {
        let id = Self::project_service_id(&project.id);
        if self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&id)
            .is_some_and(|p| p.alive())
        {
            return Ok(());
        }
        let package = crate::model::php_package(&project.php_version)?;
        let directory = self.home.join("bin/php").join(&project.php_version);
        crate::install::validate_installation(&directory, &package)
            .with_context(|| format!("{} projesinin PHP {} kurulumu eksik. Projenin PHP seçimini uygulayın veya tüm bileşenleri kurun.", project.name, project.php_version))?;
        let ini = self.write_php_config_for(&project.php_version)?;
        let existing = self
            .project_ports
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&project.id)
            .copied();
        let port = match existing {
            Some(port) => port,
            None => {
                // Reserve an OS-assigned loopback port while checking the other configured ports.
                let settings = self
                    .config
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .settings
                    .clone();
                let mut reservations = Vec::new();
                loop {
                    let listener = TcpListener::bind(("127.0.0.1", 0))?;
                    let candidate = listener.local_addr()?.port();
                    let reserved = self
                        .project_ports
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .values()
                        .any(|p| *p == candidate);
                    if !reserved && !settings.reserved_ports().contains(&candidate) {
                        break candidate;
                    }
                    reservations.push(listener);
                }
            }
        };
        let mut cmd = command(directory.join("php-cgi.exe"));
        cmd.arg("-c")
            .arg(ini)
            .arg("-d")
            .arg(format!(
                "extension_dir=\"{}\"",
                portable_path(&directory.join("ext"))
            ))
            .arg("-b")
            .arg(format!("127.0.0.1:{port}"))
            .env("PHP_FCGI_MAX_REQUESTS", "0")
            .env("F4BOX_PHP_EXT", directory.join("ext"))
            .env("PHP_INI_SCAN_DIR", "");
        self.spawn_service(&id, cmd, port)
            .with_context(|| format!("{} projesinin PHP süreci başlatılamadı", project.name))?;
        self.project_ports
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(project.id.clone(), port);
        Ok(())
    }

    pub fn select_project_php(&self, id: &str, version: &str) -> Result<()> {
        let _guard = self.gate()?;
        self.select_project_php_inner(id, version)
    }

    pub fn repair_project_php(&self, id: &str, version: &str) -> Result<()> {
        let _guard = self.gate()?;
        if !self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .projects
            .iter()
            .any(|p| p.id == id)
        {
            anyhow::bail!("Proje bulunamadı.");
        }
        if self.snapshot()?.any_running {
            anyhow::bail!("PHP onarımı için önce çalışan ortamı durdurun.");
        }
        self.check_install_requirements()?;
        let package = crate::model::php_package(version)?;
        crate::install::repair(&self.home, &package, |line| self.log(line))?;
        self.select_project_php_inner(id, version)
    }

    fn select_project_php_inner(&self, id: &str, version: &str) -> Result<()> {
        let original = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let mut updated = original.clone();
        let project = updated
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .context("Proje bulunamadı.")?;
        self.prepare_php(version)?;
        project.php_version = version.into();
        self.apply_project_config(&original, &updated)?;
        self.log(format!(
            "Proje PHP sürümü kaydedildi: {version}. Açık terminalleri yeniden açın."
        ));
        Ok(())
    }

    pub(crate) fn apply_project_config(&self, original: &Config, updated: &Config) -> Result<()> {
        self.snapshot()?;
        let baseline = self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let web_running = baseline.iter().any(|id| id == "caddy");
        let php_running = baseline
            .iter()
            .any(|id| id == "php" || Self::is_project_service_id(id));
        let changed = original
            .projects
            .iter()
            .filter(|old| {
                updated
                    .projects
                    .iter()
                    .any(|new| new.id == old.id && new.php_version != old.php_version)
            })
            .collect::<Vec<_>>();
        self.save_config(updated)?;
        let result: Result<()> = (|| {
            for project in &changed {
                self.stop_jobs_for_project(&project.id)?;
                self.stop_service(&Self::project_service_id(&project.id))?;
            }
            if php_running || web_running {
                self.start_project_workers()?;
                self.start_autostart_jobs();
            }
            if web_running {
                self.stop_service("caddy")?;
                self.start_caddy()?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.rollback_new_services(&baseline);
            for project in &changed {
                self.stop_service(&Self::project_service_id(&project.id))?;
            }
            self.save_config(original)
                .context("Önceki proje ayarları geri yüklenemedi.")?;
            let restore = (|| -> Result<()> {
                // Restore only workers that were alive before this operation.
                for project in &original.projects {
                    if baseline.contains(&Self::project_service_id(&project.id)) {
                        self.start_project_php(project)?;
                    }
                }
                if web_running {
                    self.stop_service("caddy")?;
                    self.start_caddy()?;
                }
                Ok(())
            })();
            restore.with_context(|| {
                format!("Proje değişikliği başarısız ({error:#}); eski ortam başlatılamadı")
            })?;
            return Err(error.context("Proje değişikliği geri alındı."));
        }
        for project in &original.projects {
            if !updated.projects.iter().any(|p| p.id == project.id) {
                let key = Self::project_service_id(&project.id);
                self.stop_jobs_for_project(&project.id)?;
                self.stop_service(&key)?;
                self.project_ports
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&project.id);
                self.service_errors
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&key);
            }
        }
        Ok(())
    }
}
