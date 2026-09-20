use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub url: String,
    pub sha256: String,
    pub archive: bool,
    pub prefix: String,
    pub executable: String,
    pub license: String,
    pub source: String,
}

pub fn catalog() -> Vec<Package> {
    serde_json::from_str(include_str!("../catalog.json")).expect("embedded package catalog")
}

pub fn php_versions() -> Vec<Package> {
    serde_json::from_str(include_str!("../php-versions.json")).expect("embedded PHP catalog")
}

/// Optional helpers that are installed on demand and never block the environment.
pub fn tools() -> Vec<Package> {
    serde_json::from_str(include_str!("../tools.json")).expect("embedded tool catalog")
}

pub fn tool_package(id: &str) -> Result<Package> {
    tools()
        .into_iter()
        .find(|p| p.id == id)
        .context("Bu yardımcı araç katalogda bulunmuyor.")
}

pub fn php_package(version: &str) -> Result<Package> {
    php_versions()
        .into_iter()
        .find(|p| p.version == version)
        .context("Bu PHP sürümü katalogda bulunmuyor.")
}

pub fn default_php_version() -> String {
    catalog()
        .into_iter()
        .find(|p| p.id == "php")
        .expect("default PHP")
        .version
}

pub fn selected_catalog(version: &str) -> Result<Vec<Package>> {
    let php = php_package(version)?;
    Ok(catalog()
        .into_iter()
        .map(|p| if p.id == "php" { php.clone() } else { p })
        .collect())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct QueueWorker {
    pub id: String,
    pub name: String,
    pub connection: String,
    pub queue: String,
    pub processes: u8,
    pub timeout: u32,
    pub sleep: u32,
    pub max_tries: u32,
    pub memory: u32,
    pub backoff: u32,
    pub enabled: bool,
    pub auto_start: bool,
}

impl Default for QueueWorker {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "default".into(),
            connection: "default".into(),
            queue: "default".into(),
            processes: 1,
            timeout: 60,
            sleep: 3,
            max_tries: 1,
            memory: 128,
            backoff: 0,
            enabled: true,
            auto_start: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSchedule {
    pub enabled: bool,
    pub auto_start: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub host: String,
    #[serde(default)]
    pub php_version: String,
    #[serde(default)]
    pub workers: Vec<QueueWorker>,
    #[serde(default)]
    pub schedule: ProjectSchedule,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerState {
    pub id: String,
    pub running: u8,
    pub pids: Vec<u32>,
    pub issue: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatus {
    #[serde(flatten)]
    pub project: Project,
    pub running: bool,
    pub pid: Option<u32>,
    pub php_port: Option<u16>,
    pub issue: Option<String>,
    pub worker_states: Vec<WorkerState>,
    pub schedule_running: bool,
    pub schedule_pid: Option<u32>,
    pub schedule_issue: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub web_port: u16,
    pub mysql_port: u16,
    pub php_port: u16,
    pub php: crate::preferences::PhpSettings,
    pub php_versions: crate::preferences::PhpProfiles,
    pub mysql: crate::preferences::MysqlSettings,
    pub web: crate::preferences::WebSettings,
    pub phpmyadmin: crate::preferences::PmaSettings,
    pub tunnel: crate::preferences::TunnelSettings,
    pub mail: crate::preferences::MailSettings,
    pub projects_dir: String,
    pub backups_dir: String,
    pub start_on_launch: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            web_port: 8088,
            mysql_port: 13306,
            php_port: 19000,
            php: Default::default(),
            php_versions: Default::default(),
            mysql: Default::default(),
            web: Default::default(),
            phpmyadmin: Default::default(),
            tunnel: Default::default(),
            mail: Default::default(),
            projects_dir: String::new(),
            backups_dir: String::new(),
            start_on_launch: false,
        }
    }
}

impl Settings {
    pub fn reserved_ports(&self) -> [u16; 6] {
        [
            self.web_port,
            self.mysql_port,
            self.php_port,
            self.mail.smtp_port,
            self.mail.web_port,
            self.web.https_port,
        ]
    }

    pub fn site_url(&self, host: &str) -> String {
        if self.web.https {
            format!("https://{}:{}/", host, self.web.https_port)
        } else {
            format!("http://{}:{}/", host, self.web_port)
        }
    }

    pub fn validate(&self) -> Result<()> {
        let ports = self.reserved_ports();
        if ports.contains(&0) {
            bail!("Portlar 1–65535 arasında olmalı.");
        }
        let mut unique = ports;
        unique.sort_unstable();
        if unique.windows(2).any(|pair| pair[0] == pair[1]) {
            bail!("Her bileşen için farklı bir port seçin.");
        }
        self.validate_preferences()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_php_version")]
    pub php_version: String,
    pub settings: Settings,
    pub projects: Vec<Project>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            php_version: default_php_version(),
            settings: Settings::default(),
            projects: Vec::new(),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageStatus {
    #[serde(flatten)]
    pub package: Package,
    pub installed: bool,
    pub running: bool,
    pub pid: Option<u32>,
    pub repairable: bool,
    pub issue: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub packages: Vec<PackageStatus>,
    pub php_versions: Vec<PackageStatus>,
    pub settings: Settings,
    pub projects: Vec<ProjectStatus>,
    pub tunnel: crate::tunnel::TunnelState,
    pub mail: crate::mail::MailState,
    pub node: crate::node::NodeState,
    pub permissions: crate::permissions::PermissionState,
    pub logs: Vec<String>,
    pub home: PathBuf,
    pub busy: bool,
    pub any_running: bool,
    pub recovery_issue: Option<String>,
    pub restart_required: bool,
}

pub fn validate_slug(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 48
        || name.starts_with('-')
        || name.ends_with('-')
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        bail!("Proje adı 1–48 karakter olmalı; küçük harf, rakam ve arada tire kullanın.");
    }
    let reserved = [
        "con", "prn", "aux", "nul", "com0", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
        "com8", "com9", "lpt0", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8",
        "lpt9",
    ];
    if reserved.contains(&name) {
        bail!("Bu ad Windows tarafından ayrılmış.");
    }
    Ok(())
}

pub fn database_name(name: &str) -> String {
    name.replace('-', "_")
}

/// Folder names become project slugs: lowercase, digits, hyphens.
pub fn slug_from_folder(name: &str) -> Result<String> {
    let mut slug = String::new();
    for c in name.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
        } else if matches!(c, '-' | '_' | ' ' | '.') && !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-').to_string();
    let slug = if slug.len() > 48 {
        let mut cut = slug[..48].to_string();
        while cut.ends_with('-') {
            cut.pop();
        }
        cut
    } else {
        slug
    };
    validate_slug(&slug)?;
    Ok(slug)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredProject {
    pub name: String,
    pub path: PathBuf,
    pub host: String,
}

/// Compare a `major.minor.patch` PHP version against a minimum series.
pub fn php_meets(version: &str, major: u32, minor: u32) -> bool {
    let mut parts = version.split('.');
    let parsed_major = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .unwrap_or(0);
    let parsed_minor = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .unwrap_or(0);
    (parsed_major, parsed_minor) >= (major, minor)
}

pub fn php_supports_laravel12(version: &str) -> bool {
    php_meets(version, 8, 2)
}

pub fn site_block(settings: &Settings, host: &str, body: &str) -> String {
    let body = body.trim_end();
    if settings.web.https {
        format!(
            "http://{host}:{} {{\n  bind 127.0.0.1\n  redir https://{host}:{}{{uri}}\n}}\nhttps://{host}:{} {{\n  bind 127.0.0.1\n  tls internal\n{body}\n}}\n",
            settings.web_port, settings.web.https_port, settings.web.https_port
        )
    } else {
        format!(
            "http://{host}:{} {{\n  bind 127.0.0.1\n{body}\n}}\n",
            settings.web_port
        )
    }
}

pub fn caddy_config(
    settings: &Settings,
    projects: &[Project],
    welcome: &std::path::Path,
    project_ports: &std::collections::HashMap<String, u16>,
) -> Result<String> {
    let mut text = String::from("{\n  admin off\n  auto_https off\n}\n");
    text.push_str(&site_block(
        settings,
        "localhost",
        &format!("  root * {}\n  file_server\n", quote_path(welcome)),
    ));
    for project in projects {
        let port = project_ports
            .get(&project.id)
            .ok_or_else(|| anyhow::anyhow!("{} için PHP portu hazır değil.", project.name))?;
        text.push_str(&site_block(
            settings,
            &project.host,
            &format!(
                "  root * {}\n  php_fastcgi 127.0.0.1:{}\n  file_server\n  @private path /.env /.env/* /.git /.git/*\n  respond @private 404\n",
                quote_path(&project.path.join("public")),
                port
            ),
        ));
    }
    let options = format!(
        "{}{}",
        if settings.web.compression {
            "  encode zstd gzip\n"
        } else {
            ""
        },
        if settings.web.access_log {
            "  log\n"
        } else {
            ""
        }
    );
    text = text.replace("  file_server\n", &format!("{options}  file_server\n"));
    for port in project_ports.values() {
        text = text.replace(&format!("  php_fastcgi 127.0.0.1:{port}\n"), &format!("  php_fastcgi 127.0.0.1:{port} {{\n    dial_timeout {}s\n    read_timeout {}s\n  }}\n", settings.web.connect_seconds, settings.web.read_seconds));
    }
    Ok(text)
}

fn quote_path(path: &std::path::Path) -> String {
    // JSON quoted strings are also valid Caddyfile quoted tokens. Normalize Windows separators.
    serde_json::to_string(&crate::portable_path(path)).unwrap()
}
