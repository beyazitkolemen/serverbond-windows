use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path, time::Duration};

use crate::{
    model::{php_package, php_versions, Settings},
    process::{command, ManagedChild},
    Manager,
};

pub const EXTENSIONS: &[&str] = &[
    "curl",
    "fileinfo",
    "mbstring",
    "openssl",
    "pdo_mysql",
    "mysqli",
    "sodium",
    "pdo_sqlite",
    "sqlite3",
    "intl",
    "zip",
    "bcmath",
    "gd",
    "exif",
    "soap",
    "sockets",
    "ftp",
    "gettext",
    "imap",
    "ldap",
    "odbc",
    "pdo_odbc",
    "pgsql",
    "pdo_pgsql",
    "shmop",
    "tidy",
    "xsl",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct PhpSettings {
    pub timezone: String,
    pub memory_mb: i32,
    pub upload_mb: u32,
    pub post_mb: u32,
    pub execution_seconds: u32,
    pub input_seconds: i32,
    pub input_vars: u32,
    pub display_errors: bool,
    pub log_errors: bool,
    pub opcache: bool,
    pub opcache_mb: u32,
    pub extensions: Vec<String>,
    pub extra_ini: String,
}
impl Default for PhpSettings {
    fn default() -> Self {
        Self {
            timezone: "Europe/Istanbul".into(),
            memory_mb: 512,
            upload_mb: 64,
            post_mb: 64,
            execution_seconds: 120,
            input_seconds: -1,
            input_vars: 1000,
            display_errors: true,
            log_errors: true,
            opcache: false,
            opcache_mb: 128,
            extensions: EXTENSIONS[..11].iter().map(|s| (*s).into()).collect(),
            extra_ini: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct MysqlSettings {
    pub buffer_pool_mb: u32,
    pub max_connections: u32,
    pub max_packet_mb: u32,
    pub wait_seconds: u32,
    pub collation: String,
    pub sql_mode: String,
    pub slow_query_log: bool,
    pub long_query_seconds: u32,
}
impl Default for MysqlSettings {
    fn default() -> Self {
        Self { buffer_pool_mb: 128, max_connections: 151, max_packet_mb: 64, wait_seconds: 28800, collation: "utf8mb4_unicode_ci".into(), sql_mode: "ONLY_FULL_GROUP_BY,STRICT_TRANS_TABLES,NO_ZERO_IN_DATE,NO_ZERO_DATE,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION".into(), slow_query_log: false, long_query_seconds: 10 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct WebSettings {
    pub host_pattern: String,
    pub compression: bool,
    pub access_log: bool,
    pub read_seconds: u32,
    pub connect_seconds: u32,
}
impl Default for WebSettings {
    fn default() -> Self {
        Self {
            host_pattern: "{name}.localhost".into(),
            compression: false,
            access_log: false,
            read_seconds: 120,
            connect_seconds: 3,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct PmaSettings {
    pub enabled: bool,
    pub language: String,
    pub rows: u32,
    pub login_seconds: u32,
}
impl Default for PmaSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            language: "tr".into(),
            rows: 25,
            login_seconds: 1440,
        }
    }
}

fn range(value: u32, min: u32, max: u32, name: &str) -> Result<()> {
    if !(min..=max).contains(&value) {
        bail!("{name}: {min}–{max} arasında olmalı.");
    }
    Ok(())
}

impl PhpSettings {
    pub fn validate(&self) -> Result<()> {
        if self.timezone.is_empty()
            || self.timezone.len() > 80
            || !self
                .timezone
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"/_+-".contains(&c))
        {
            bail!("PHP saat dilimi geçersiz.");
        }
        if self.memory_mb != -1 && !(32..=32768).contains(&self.memory_mb) {
            bail!("PHP bellek: sınırsız için -1 veya 32–32768 MB girin.");
        }
        range(self.upload_mb, 1, 8192, "Yükleme (MB)")?;
        range(
            self.post_mb,
            self.upload_mb,
            8192,
            "POST limiti (yükleme limitinden küçük olamaz)",
        )?;
        range(self.execution_seconds, 0, 86400, "PHP çalışma süresi")?;
        if !(-1..=86400).contains(&self.input_seconds) {
            bail!("PHP giriş süresi -1–86400 arasında olmalı.");
        }
        range(self.input_vars, 100, 100000, "Giriş değişkenleri")?;
        range(self.opcache_mb, 8, 4096, "OPcache bellek (MB)")?;
        let mut seen = std::collections::HashSet::new();
        for extension in &self.extensions {
            if !EXTENSIONS.contains(&extension.as_str()) || !seen.insert(extension) {
                bail!("Bilinmeyen veya yinelenen PHP uzantısı: {extension}");
            }
        }
        self.extra_lines()?;
        Ok(())
    }

    fn extra_lines(&self) -> Result<Vec<(&str, &str)>> {
        if self.extra_ini.len() > 16384 {
            bail!("Ek PHP ayarları 16 KB sınırını aşıyor.");
        }
        let reserved = [
            "extension",
            "zend_extension",
            "extension_dir",
            "date.timezone",
            "memory_limit",
            "upload_max_filesize",
            "post_max_size",
            "max_execution_time",
            "max_input_time",
            "max_input_vars",
            "display_errors",
            "log_errors",
            "cgi.fix_pathinfo",
            "fastcgi.impersonate",
            "session.save_path",
        ];
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for line in self
            .extra_ini
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with(';'))
        {
            let (key, value) = line
                .split_once('=')
                .context("Ek PHP ayarlarını anahtar=değer biçiminde yazın.")?;
            let key = key.trim();
            let value = value.trim();
            if key.is_empty()
                || !key
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._".contains(&c))
                || reserved.contains(&key)
                || key.starts_with("opcache.")
                || !seen.insert(key)
                || value.chars().any(char::is_control)
                || value.is_empty()
            {
                bail!("Ek PHP ayarı geçersiz veya formdan yönetiliyor: {key}");
            }
            result.push((key, value));
        }
        Ok(result)
    }

    pub(crate) fn render(&self, ext: &Path) -> Result<String> {
        self.validate()?;
        let on = |b| if b { "On" } else { "Off" };
        let mut text = format!("; F4Box tarafından üretilir. Kalıcı değişiklikleri Ayarlar ekranından yapın.\n[PHP]\nextension_dir=\"${{F4BOX_PHP_EXT}}\"\ndate.timezone={}\nmemory_limit={}\nupload_max_filesize={}M\npost_max_size={}M\nmax_execution_time={}\nmax_input_time={}\nmax_input_vars={}\ndisplay_errors={}\nlog_errors={}\nvariables_order=EGPCS\nerror_reporting=E_ALL\ncgi.fix_pathinfo=1\nfastcgi.impersonate=0\nexpose_php=Off\n", self.timezone, if self.memory_mb == -1 { "-1".into() } else { format!("{}M", self.memory_mb) }, self.upload_mb, self.post_mb, self.execution_seconds, self.input_seconds, self.input_vars, on(self.display_errors), on(self.log_errors));
        for extension in &self.extensions {
            if ext.join(format!("php_{extension}.dll")).is_file() {
                text.push_str(&format!("extension={extension}\n"));
            } else if extension == "gd" && ext.join("php_gd2.dll").is_file() {
                // PHP 7.4 calls the Windows library gd2; the module name is gd.
                text.push_str("extension=gd2\n");
            }
        }
        if self.opcache && ext.join("php_opcache.dll").is_file() {
            text.push_str("zend_extension=opcache\n");
        }
        // Since PHP 8.5 OPcache is built in. Configure it without loading a DLL;
        // explicitly disable it too when the user's switch is off.
        text.push_str(&format!("opcache.enable={}\nopcache.enable_cli={}\nopcache.memory_consumption={}\nopcache.validate_timestamps=1\nopcache.revalidate_freq=0\n", u8::from(self.opcache), u8::from(self.opcache), self.opcache_mb));
        for (key, value) in self.extra_lines()? {
            text.push_str(&format!("{key}={value}\n"));
        }
        Ok(text)
    }
}

impl Settings {
    pub fn php_for(&self, version: &str) -> PhpSettings {
        self.php_versions.get(version).unwrap_or(&self.php).clone()
    }

    pub fn project_host(&self, name: &str) -> String {
        self.web.host_pattern.replace("{name}", name)
    }

    pub(crate) fn validate_preferences(&self) -> Result<()> {
        self.php.validate()?;
        for (version, profile) in &self.php_versions {
            php_package(version)?;
            profile.validate()?;
        }
        range(
            self.mysql.buffer_pool_mb,
            8,
            32768,
            "MySQL InnoDB bellek (MB)",
        )?;
        range(
            self.mysql.max_connections,
            10,
            5000,
            "MySQL bağlantı sayısı",
        )?;
        range(self.mysql.max_packet_mb, 1, 1024, "MySQL paket (MB)")?;
        range(
            self.mysql.wait_seconds,
            1,
            31536000,
            "MySQL bağlantı zaman aşımı",
        )?;
        range(self.mysql.long_query_seconds, 0, 3600, "Yavaş sorgu eşiği")?;
        if ![
            "utf8mb4_unicode_ci",
            "utf8mb4_0900_ai_ci",
            "utf8mb4_general_ci",
            "utf8mb4_bin",
            "utf8mb4_turkish_ci",
        ]
        .contains(&self.mysql.collation.as_str())
        {
            bail!("Desteklenmeyen MySQL karşılaştırma düzeni.");
        }
        let modes = [
            "ONLY_FULL_GROUP_BY",
            "STRICT_TRANS_TABLES",
            "STRICT_ALL_TABLES",
            "NO_ZERO_IN_DATE",
            "NO_ZERO_DATE",
            "ERROR_FOR_DIVISION_BY_ZERO",
            "NO_ENGINE_SUBSTITUTION",
            "ANSI_QUOTES",
            "NO_AUTO_VALUE_ON_ZERO",
            "NO_BACKSLASH_ESCAPES",
            "PIPES_AS_CONCAT",
            "REAL_AS_FLOAT",
            "IGNORE_SPACE",
            "NO_UNSIGNED_SUBTRACTION",
            "NO_DIR_IN_CREATE",
            "TIME_TRUNCATE_FRACTIONAL",
            "ALLOW_INVALID_DATES",
            "ANSI",
            "TRADITIONAL",
        ];
        if !self.mysql.sql_mode.is_empty()
            && self.mysql.sql_mode.split(',').any(|s| !modes.contains(&s))
        {
            bail!("MySQL SQL modları virgülle ayrılmış geçerli büyük harfli adlar olmalı.");
        }
        let host = self.project_host(&"a".repeat(48));
        if self.web.host_pattern.matches("{name}").count() != 1
            || !host.ends_with(".localhost")
            || host.len() > 190
            || host.split('.').any(|s| {
                s.is_empty()
                    || s.len() > 63
                    || s.starts_with('-')
                    || s.ends_with('-')
                    || !s
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
            })
        {
            bail!("Adres kalıbı bir {{name}} içermeli ve .localhost ile bitmeli. Örnek: {{name}}.dev.localhost");
        }
        range(self.web.read_seconds, 1, 86400, "Web yanıt süresi")?;
        range(self.web.connect_seconds, 1, 120, "FastCGI bağlantı süresi")?;
        if ![
            "tr", "en", "de", "fr", "es", "it", "pt", "ru", "ar", "ja", "zh_CN",
        ]
        .contains(&self.phpmyadmin.language.as_str())
        {
            bail!("Desteklenmeyen phpMyAdmin dili.");
        }
        range(self.phpmyadmin.rows, 10, 1000, "phpMyAdmin satır sayısı")?;
        range(
            self.phpmyadmin.login_seconds,
            60,
            86400,
            "phpMyAdmin oturum süresi",
        )?;
        for path in [&self.projects_dir, &self.backups_dir] {
            if !path.is_empty()
                && (!Path::new(path).is_absolute() || path.chars().any(char::is_control))
            {
                bail!("Klasör yolu mutlak bir yol olmalı.");
            }
        }
        Ok(())
    }
}

impl Manager {
    pub(crate) fn validate_php_profile(&self, version: &str, settings: &PhpSettings) -> Result<()> {
        let root = self.home.join("bin/php").join(version);
        if !root.join("php.exe").is_file() {
            return Ok(());
        }
        let mut ini = tempfile::NamedTempFile::new_in(self.home.join("config"))?;
        use std::io::Write;
        ini.write_all(settings.render(&root.join("ext"))?.as_bytes())?;
        ini.flush()?;
        let required = serde_json::to_string(&settings.extensions)?;
        let extra_keys: Vec<_> = settings
            .extra_lines()?
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        let keys = serde_json::to_string(&extra_keys)?;
        let script = format!("$required=json_decode('{required}',true); foreach($required as $ext){{if(!extension_loaded($ext)){{fwrite(STDERR,'Uzantı yüklenemedi: '.$ext);exit(1);}}}} $keys=json_decode('{keys}',true); foreach($keys as $key){{if(ini_get($key)===false){{fwrite(STDERR,'Bilinmeyen PHP ayarı: '.$key);exit(1);}}}} if(!in_array(ini_get('date.timezone'),timezone_identifiers_list(DateTimeZone::ALL_WITH_BC),true)){{fwrite(STDERR,'Geçersiz saat dilimi');exit(1);}} echo 'F4BOX_OK';");
        let mut cmd = command(root.join("php.exe"));
        cmd.arg("-c")
            .arg(ini.path())
            .arg("-d")
            .arg(format!(
                "extension_dir=\"{}\"",
                crate::portable_path(&root.join("ext"))
            ))
            .args(["-r", &script])
            .env("F4BOX_PHP_EXT", root.join("ext"))
            .env("PHP_INI_SCAN_DIR", "");
        let output = ManagedChild::output(cmd, Duration::from_secs(20))?;
        if !output.status.success()
            || !output.stderr.is_empty()
            || String::from_utf8_lossy(&output.stdout).trim() != "F4BOX_OK"
        {
            bail!(
                "PHP {version} ayarları uygulanamadı: {} {}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            );
        }
        Ok(())
    }

    pub(crate) fn validate_installed_preferences(&self, settings: &Settings) -> Result<()> {
        for p in php_versions() {
            self.validate_php_profile(&p.version, &settings.php_for(&p.version))?;
        }
        for value in [&settings.projects_dir, &settings.backups_dir]
            .into_iter()
            .filter(|s| !s.is_empty())
        {
            if !Path::new(value).is_dir() {
                bail!("Klasör bulunamadı: {value}");
            }
            tempfile::NamedTempFile::new_in(value)
                .with_context(|| format!("Klasöre yazılamıyor: {value}"))?;
        }
        Ok(())
    }

    pub fn previous_settings(&self) -> Result<Settings> {
        Ok(serde_json::from_slice(
            &crate::storage::read_limited(
                &self.home.join("config/settings.previous.json"),
                256 * 1024,
            )
            .context("Önceki ayar yedeği henüz yok.")?,
        )?)
    }
    pub fn export_settings(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(
            &self
                .config
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .settings,
        )?)
    }
    pub fn import_settings(&self, json: &str) -> Result<()> {
        if json.len() > 256 * 1024 {
            bail!("Ayar dosyası 256 KB sınırını aşıyor.");
        }
        self.save_settings(serde_json::from_str(json).context("Ayar dosyası geçersiz.")?)
    }
    pub fn default_settings(&self) -> Settings {
        let s = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .clone();
        Settings {
            web_port: s.web_port,
            mysql_port: s.mysql_port,
            php_port: s.php_port,
            ..Settings::default()
        }
    }
}

pub type PhpProfiles = BTreeMap<String, PhpSettings>;
