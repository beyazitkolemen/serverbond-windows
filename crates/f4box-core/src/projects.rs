use crate::{
    model::{slug_from_folder, validate_slug, DiscoveredProject, Project},
    process::ManagedChild,
    Manager,
};
use anyhow::{bail, Context, Result};
use std::{fs, path::PathBuf, time::Duration};

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
            workers: Vec::new(),
            schedule: Default::default(),
        };
        if project.host == "phpmyadmin.f4box.localhost" {
            bail!("Bu adres phpMyAdmin için ayrılmış.");
        }
        let mut config = original.clone();
        config.projects.push(project.clone());
        self.apply_project_config(&original, &config)?;
        self.maybe_create_project_database(&project.name);
        self.log(format!("Proje eklendi: {}", project.name));
        Ok(project)
    }

    pub fn discover_projects(&self) -> Result<Vec<DiscoveredProject>> {
        let config = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let root = if config.settings.projects_dir.is_empty() {
            self.home.join("www")
        } else {
            PathBuf::from(&config.settings.projects_dir)
        };
        if !root.is_dir() {
            return Ok(Vec::new());
        }
        let mut found = Vec::new();
        let mut entries: Vec<_> = fs::read_dir(&root)?.filter_map(Result::ok).collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let Ok(path) = dunce::canonicalize(entry.path()) else {
                continue;
            };
            if !path.is_dir() || !path.join("public/index.php").is_file() {
                continue;
            }
            let Some(folder) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let Ok(name) = slug_from_folder(folder) else {
                continue;
            };
            if config
                .projects
                .iter()
                .any(|project| project.name == name || project.path == path)
                || found
                    .iter()
                    .any(|item: &DiscoveredProject| item.name == name)
            {
                continue;
            }
            let host = config.settings.project_host(&name);
            if host == "phpmyadmin.f4box.localhost" {
                continue;
            }
            found.push(DiscoveredProject { name, path, host });
            if config.projects.len() + found.len() >= 1000 {
                break;
            }
        }
        Ok(found)
    }

    pub fn import_projects(&self, paths: Vec<PathBuf>) -> Result<Vec<Project>> {
        let _guard = self.gate()?;
        if paths.is_empty() {
            bail!("Eklenecek proje seçilmedi.");
        }
        if paths.len() > 1000 {
            bail!("En fazla 1000 proje aktarılabilir.");
        }
        let original = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let mut config = original.clone();
        let mut added = Vec::new();
        for raw in paths {
            let path = dunce::canonicalize(raw).context("Proje klasörü bulunamadı.")?;
            if !path.join("public/index.php").is_file() {
                bail!(
                    "Proje kökünde public/index.php bulunmalı. Laravel projesinin kök klasörünü seçin."
                );
            }
            let folder = path
                .file_name()
                .and_then(|value| value.to_str())
                .context("Klasör adı okunamadı.")?;
            let name = slug_from_folder(folder)?;
            if config
                .projects
                .iter()
                .any(|project| project.name == name || project.path == path)
            {
                bail!("Bu proje veya alan adı zaten kayıtlı: {name}");
            }
            let project = Project {
                id: uuid::Uuid::new_v4().to_string(),
                host: config.settings.project_host(&name),
                name,
                path,
                php_version: config.php_version.clone(),
                workers: Vec::new(),
                schedule: Default::default(),
            };
            if project.host == "phpmyadmin.f4box.localhost" {
                bail!("Bu adres phpMyAdmin için ayrılmış.");
            }
            config.projects.push(project.clone());
            added.push(project);
            if config.projects.len() > 1000 {
                bail!("Yapılandırma boyutu veya proje sayısı sınırı aşıldı.");
            }
        }
        self.apply_project_config(&original, &config)?;
        for project in &added {
            self.maybe_create_project_database(&project.name);
            self.log(format!("Proje eklendi: {}", project.name));
        }
        Ok(added)
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
        if !crate::model::php_supports_laravel12(&version) {
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
