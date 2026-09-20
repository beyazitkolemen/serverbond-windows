use crate::{
    model::{validate_slug, Project},
    process::ManagedChild,
    Manager,
};
use anyhow::{bail, Context, Result};
use std::{path::PathBuf, time::Duration};

impl Manager {
    pub fn add_project(&self, name: String, path: PathBuf) -> Result<Project> {
        let _guard = self.gate()?;
        self.add_project_inner(name, path)
    }

    fn add_project_inner(&self, name: String, path: PathBuf) -> Result<Project> {
        validate_slug(&name)?;
        let path = dunce::canonicalize(path).context("Proje klasörü bulunamadı.")?;
        if !path.join("public/index.php").is_file() {
            bail!(
                "Proje kökünde public/index.php bulunmalı. Laravel projesinin kök klasörünü seçin."
            );
        }
        let original = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if original
            .projects
            .iter()
            .any(|p| p.name == name || p.path == path)
        {
            bail!("Bu proje veya alan adı zaten kayıtlı.");
        }
        let project = Project {
            id: uuid::Uuid::new_v4().to_string(),
            host: original.settings.project_host(&name),
            name,
            path,
            php_version: original.php_version.clone(),
        };
        if project.host == "phpmyadmin.f4box.localhost" {
            bail!("Bu adres phpMyAdmin için ayrılmış.");
        }
        let mut config = original.clone();
        config.projects.push(project.clone());
        self.apply_project_config(&original, &config)?;
        self.log(format!("Proje eklendi: {}", project.name));
        Ok(project)
    }

    pub fn remove_project(&self, id: &str) -> Result<()> {
        let _guard = self.gate()?;
        let original = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let mut config = original.clone();
        config.projects.retain(|p| p.id != id);
        if config.projects.len() == original.projects.len() {
            bail!("Proje bulunamadı.");
        }
        self.apply_project_config(&original, &config)?;
        self.log("Proje listeden kaldırıldı. Proje dosyaları ve veritabanı korundu.");
        Ok(())
    }

    pub fn create_project(&self, name: String, parent: PathBuf) -> Result<Project> {
        let _guard = self.gate()?;
        let version = self.package("php")?.version;
        if version.starts_with("7.") || version.starts_with("8.0.") || version.starts_with("8.1.") {
            bail!("Yeni Laravel 12 projesi için PHP 8.2 veya üzerini seçin. Eski PHP sürümleriyle mevcut projelerinizi ekleyebilirsiniz.");
        }
        validate_slug(&name)?;
        if self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .projects
            .iter()
            .any(|p| p.name == name)
        {
            bail!("Bu proje adı zaten kayıtlı.");
        }
        let parent = dunce::canonicalize(parent).context("Üst klasör bulunamadı.")?;
        let destination = parent.join(&name);
        if destination.exists() {
            bail!("Hedef klasör zaten var; mevcut dosyaların üzerine yazılmadı.");
        }
        let composer = self.executable("composer")?;
        self.write_php_config()?;
        self.log(format!(
            "Laravel projesi oluşturuluyor: {name}. Composer günlüğünden takip edebilirsiniz."
        ));
        let mut cmd = self.php_command()?;
        cmd.arg(composer)
            .args([
                "create-project",
                "--prefer-dist",
                "--no-interaction",
                "--no-progress",
                "laravel/laravel:^12.0",
            ])
            .arg(&destination);
        cmd.current_dir(&parent);
        let mut paths = vec![self.package_dir("php")?];
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        cmd.env("PATH", std::env::join_paths(paths)?)
            .env("PHPRC", self.php_ini_path()?)
            .env("PHP_INI_SCAN_DIR", "");
        let mut child = ManagedChild::spawn(cmd, &self.home.join("logs/composer.log"))?;
        child.wait_timeout(Duration::from_secs(900)).context("Laravel oluşturulamadı. Composer günlüğünü kontrol edin. Oluşan dosyalar inceleme için korundu.")?;
        self.add_project_inner(name, destination)
    }
}
