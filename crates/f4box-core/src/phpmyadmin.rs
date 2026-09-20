use crate::{model::Settings, portable_path, product, secrets, Manager};
use anyhow::{bail, Result};
use std::{fs, io::Write, path::Path};

const HOST: &str = product::PMA_HOST;

fn configuration(settings: &Settings, secret: &str) -> String {
    format!(
        r#"<?php
// Managed by {name}. MySQL credentials are entered on the login page.
$cfg['blowfish_secret'] = hex2bin('{secret}');
$cfg['Servers'][1]['auth_type'] = 'cookie';
$cfg['Servers'][1]['verbose'] = '{name} MySQL';
$cfg['Servers'][1]['host'] = '127.0.0.1';
$cfg['Servers'][1]['port'] = '{port}';
$cfg['Servers'][1]['AllowNoPassword'] = false;
$cfg['AllowArbitraryServer'] = false;
$cfg['TempDir'] = getenv('{tmp}');
$cfg['SessionSavePath'] = getenv('{sessions}');
$cfg['DefaultLang'] = '{language}';
$cfg['MaxRows'] = {rows};
$cfg['LoginCookieValidity'] = {login};
ini_set('session.gc_maxlifetime', '{login}');
$cfg['VersionCheck'] = false;
"#,
        name = product::NAME,
        tmp = product::PMA_TMP_ENV,
        sessions = product::PMA_SESSIONS_ENV,
        port = settings.mysql_port,
        language = settings.phpmyadmin.language,
        rows = settings.phpmyadmin.rows,
        login = settings.phpmyadmin.login_seconds
    )
}

pub(crate) fn caddy_route(settings: &Settings, root: &Path) -> String {
    let root = serde_json::to_string(&portable_path(root)).unwrap();
    let route = crate::model::site_block(
        settings,
        HOST,
        &format!("  root * {root}\n  @private path_regexp (?i)^/(config[^/]*\\.php|installed\\.json|composer\\.[^/]*|package\\.json|yarn\\.lock|setup|libraries|templates|vendor|doc|sql|\\.git|\\.env)(/|$)\n  respond @private 404\n  @otherPHP {{\n    path_regexp (?i)\\.php(/|$)\n    not path /index.php /url.php /js/messages.php\n  }}\n  respond @otherPHP 404\n  php_fastcgi 127.0.0.1:{}\n  file_server\n", settings.php_port),
    );
    let route = route.replace(
        "  file_server\n",
        &format!(
            "{}{}  file_server\n",
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
        ),
    );
    route.replace(
        &format!("  php_fastcgi 127.0.0.1:{}\n", settings.php_port),
        &format!(
            "  php_fastcgi 127.0.0.1:{} {{\n    dial_timeout {}s\n    read_timeout {}s\n  }}\n",
            settings.php_port, settings.web.connect_seconds, settings.web.read_seconds
        ),
    )
}

impl Manager {
    pub(crate) fn write_phpmyadmin_config(&self) -> Result<()> {
        self.executable("phpmyadmin")?;
        for name in ["tmp", "sessions"] {
            fs::create_dir_all(self.home.join("data/phpmyadmin").join(name))?;
        }
        let secret_path = self.home.join("config/phpmyadmin-secret.dpapi");
        if !secret_path.exists() {
            let value = format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            );
            secrets::save(&secret_path, &value)?;
        }
        let secret = secrets::read(&secret_path)?;
        if secret.len() != 64 || !secret.bytes().all(|c| c.is_ascii_hexdigit()) {
            bail!("phpMyAdmin oturum anahtarı geçersiz.");
        }
        let settings = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .clone();
        let root = self.package_dir("phpmyadmin")?;
        let mut file = tempfile::NamedTempFile::new_in(&root)?;
        file.write_all(configuration(&settings, &secret).as_bytes())?;
        file.as_file().sync_all()?;
        file.persist(root.join("config.inc.php"))?;
        Ok(())
    }

    pub fn phpmyadmin_url(&self) -> Result<String> {
        if !self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .phpmyadmin
            .enabled
        {
            bail!("phpMyAdmin Ayarlar ekranından devre dışı bırakılmış.");
        }
        let snapshot = self.snapshot()?;
        self.executable("phpmyadmin")?;
        if !["php", "mysql", "caddy"].iter().all(|id| {
            snapshot
                .packages
                .iter()
                .any(|p| p.package.id == *id && p.running)
        }) {
            bail!("phpMyAdmin için önce PHP, MySQL ve web sunucusunu başlatın.");
        }
        Ok(snapshot.settings.site_url(HOST))
    }

    pub fn open_phpmyadmin(&self) -> Result<()> {
        let url = self.phpmyadmin_url()?;
        crate::process::command("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", &url])
            .spawn()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_uses_cookie_auth_and_configured_mysql_port_without_credentials() {
        let config = configuration(
            &Settings {
                mysql_port: 23316,
                ..Settings::default()
            },
            &"ab".repeat(32),
        );
        assert!(config.contains("['port'] = '23316'"));
        assert!(config.contains("['auth_type'] = 'cookie'"));
        assert!(config.contains("['AllowNoPassword'] = false"));
        assert!(!config.contains("['password']"));
        assert!(!config.contains("['user']"));
        assert!(config.contains(&format!("getenv('{}')", product::PMA_SESSIONS_ENV)));
    }
    #[test]
    fn route_is_loopback_only_and_protects_config_and_internal_files() {
        let route = caddy_route(
            &Settings::default(),
            Path::new("C:/Türkçe ServerBond/phpMyAdmin"),
        );
        assert!(route.contains(&format!("http://{}:8088", product::PMA_HOST)));
        assert!(route.contains("bind 127.0.0.1"));
        assert!(route.contains("php_fastcgi 127.0.0.1:19000"));
        assert!(route.contains("root * \"C:/Türkçe ServerBond/phpMyAdmin\""));
        assert!(route.contains("path_regexp (?i)"));
        assert!(route.contains("not path /index.php /url.php /js/messages.php"));
        assert!(route.contains("respond @private 404"));
        assert!(!route.contains("tls internal"));
    }
    #[test]
    fn https_route_redirects_and_keeps_loopback() {
        let mut settings = Settings::default();
        settings.web.https = true;
        let route = caddy_route(&settings, Path::new("C:/ServerBond/phpMyAdmin"));
        assert!(route.contains(&format!("redir https://{}:8443{{uri}}", product::PMA_HOST)));
        assert!(route.contains(&format!("https://{}:8443", product::PMA_HOST)));
        assert!(route.contains("tls internal"));
        assert!(route.contains("bind 127.0.0.1"));
    }
}
