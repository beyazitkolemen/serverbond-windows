//! Project registry: add an existing Laravel root, create a new one with
//! Composer, discover roots in the workspace, import folders, remove
//! entries, and keep `{name}.localhost` hosts unique.

use crate::{
    model::{slug_from_folder, validate_slug, DiscoveredProject, Project},
    process::ManagedChild,
    Manager,
};
use anyhow::{bail, Context, Result};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

/// Runtime, bağımlılık ve çerçeve klasörleri proje kökü değildir.
const SKIP_FOLDERS: &[&str] = &[
    ".git",
    ".idea",
    ".vscode",
    "backups",
    "bin",
    "bootstrap",
    "build",
    "cache",
    "config",
    "data",
    "database",
    "dist",
    "logs",
    "node_modules",
    "public",
    "resources",
    "storage",
    "target",
    "tests",
    "vendor",
    "welcome",
];

fn is_skippable_folder(name: &str) -> bool {
    name.starts_with('.')
        || SKIP_FOLDERS
            .iter()
            .any(|skip| skip.eq_ignore_ascii_case(name))
}

fn is_laravel_root(path: &Path) -> bool {
    path.join("public/index.php").is_file()
}

fn list_dirs(path: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<_> = fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    entries.sort();
    entries
}

fn folder_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
}

struct Candidate {
    path: PathBuf,
    folder: String,
    parent: Option<String>,
    depth: u8,
}

fn collect_candidates(root: &Path, registered: &HashSet<PathBuf>, out: &mut Vec<Candidate>) {
    for path in list_dirs(root) {
        let Some(folder) = folder_name(&path) else {
            continue;
        };
        if is_skippable_folder(&folder) {
            continue;
        }
        let Ok(path) = dunce::canonicalize(&path) else {
            continue;
        };
        if registered.contains(&path) {
            continue;
        }
        if is_laravel_root(&path) {
            out.push(Candidate {
                path,
                folder,
                parent: None,
                depth: 1,
            });
            continue;
        }
        for child in list_dirs(&path) {
            let Some(child_name) = folder_name(&child) else {
                continue;
            };
            let Ok(child) = dunce::canonicalize(&child) else {
                continue;
            };
            if registered.contains(&child)
                || !is_laravel_root(&child)
                || is_skippable_folder(&child_name)
            {
                continue;
            }
            out.push(Candidate {
                path: child,
                folder: child_name,
                parent: Some(folder.clone()),
                depth: 2,
            });
        }
    }
}

fn discover_slug(folder: &str, parent: Option<&str>, used: &HashSet<String>) -> Option<String> {
    let base = slug_from_folder(folder).ok()?;
    if !used.contains(&base) {
        return Some(base);
    }
    let parent = parent?;
    let org = slug_from_folder(parent).ok()?;
    let qualified = format!("{org}-{base}");
    if qualified.len() <= 48 && validate_slug(&qualified).is_ok() && !used.contains(&qualified) {
        Some(qualified)
    } else {
        None
    }
}

impl Manager {
    pub fn default_projects_dir(&self) -> PathBuf {
        self.home.join("www")
    }

    pub fn project_scan_roots(&self) -> Vec<PathBuf> {
        let projects_dir = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .projects_dir
            .clone();
        if !projects_dir.is_empty() {
            return vec![PathBuf::from(projects_dir)];
        }
        let mut roots = vec![self.default_projects_dir()];
        let legacy = self.home.join("projects");
        if legacy.is_dir() {
            roots.push(legacy);
        }
        roots
    }

    pub fn add_project(&self, name: String, path: PathBuf) -> Result<Project> {
        let _guard = self.gate()?;
        self.add_project_inner(name, path)
    }

    pub(crate) fn add_project_inner(&self, name: String, path: PathBuf) -> Result<Project> {
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
            release: Default::default(),
        };
        if crate::product::is_phpmyadmin_host(&project.host) {
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
        let mut used: HashSet<String> = config
            .projects
            .iter()
            .map(|project| project.name.clone())
            .collect();
        let registered: HashSet<PathBuf> = config
            .projects
            .iter()
            .map(|project| project.path.clone())
            .collect();
        let mut found = Vec::new();
        let mut candidates = Vec::new();
        for root in self.project_scan_roots() {
            if root.is_dir() {
                collect_candidates(&root, &registered, &mut candidates);
            }
        }
        candidates.sort_by(|a, b| a.depth.cmp(&b.depth).then_with(|| a.path.cmp(&b.path)));
        let mut seen = HashSet::new();
        for candidate in candidates {
            if !seen.insert(candidate.path.clone()) {
                continue;
            }
            if config.projects.len() + found.len() >= 1000 {
                break;
            }
            let Some(name) = discover_slug(&candidate.folder, candidate.parent.as_deref(), &used)
            else {
                continue;
            };
            let host = config.settings.project_host(&name);
            if crate::product::is_phpmyadmin_host(&host) {
                continue;
            }
            used.insert(name.clone());
            found.push(DiscoveredProject {
                name,
                path: candidate.path,
                host,
            });
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
        // Resolve the same names the discovery dialog presented, independent of
        // the user's selection order or whether they selected the whole list.
        let discovered: HashMap<_, _> = self
            .discover_projects()?
            .into_iter()
            .map(|project| (project.path, project.name))
            .collect();
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
            let name = match discovered.get(&path) {
                Some(name) => name.clone(),
                None => slug_from_folder(folder)?,
            };
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
                release: Default::default(),
            };
            if crate::product::is_phpmyadmin_host(&project.host) {
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
