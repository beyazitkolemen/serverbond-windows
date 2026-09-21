#![cfg(windows)]
use anyhow::Result;
use serverbond_core::Manager;
use std::{fs, time::Duration};

#[test]
#[ignore = "starts real PHP, MySQL and Caddy in a disposable Unicode environment"]
fn user_preferences_reach_real_runtimes_and_survive_repair() -> Result<()> {
    let home = tempfile::Builder::new()
        .prefix("ServerBond ayar Türkçe ")
        .tempdir()?;
    let manager = Manager::new(home.path().into())?;
    let mut settings = manager.snapshot()?.settings;
    let listeners = (0..4)
        .map(|_| std::net::TcpListener::bind("127.0.0.1:0"))
        .collect::<std::io::Result<Vec<_>>>()?;
    settings.web_port = listeners[0].local_addr()?.port();
    settings.mysql_port = listeners[1].local_addr()?.port();
    settings.php_port = listeners[2].local_addr()?.port();
    settings.web.https_port = listeners[3].local_addr()?.port();
    drop(listeners);
    manager.save_settings(settings)?;
    if let Some(cache) = std::env::var_os("SERVERBOND_TEST_CACHE") {
        for item in fs::read_dir(cache)? {
            let item = item?;
            if item.file_type()?.is_file() {
                fs::copy(
                    item.path(),
                    home.path().join("cache").join(item.file_name()),
                )?;
            }
        }
    }
    manager.install("all")?;
    manager.select_php("7.4.33")?;
    let path = manager.home.join("www/check");
    fs::create_dir_all(path.join("public"))?;
    fs::write(path.join(".env"), "APP_KEY=preserve-settings-test")?;
    fs::write(path.join("public/index.php"), "<?php header('Content-Type: application/json'); echo json_encode(['version'=>PHP_VERSION,'memory'=>ini_get('memory_limit'),'timezone'=>date_default_timezone_get(),'upload'=>ini_get('upload_max_filesize'),'post'=>ini_get('post_max_size'),'execution'=>ini_get('max_execution_time'),'input'=>ini_get('max_input_vars'),'precision'=>ini_get('serialize_precision'),'gd'=>extension_loaded('gd'),'opcache'=>extension_loaded('Zend OPcache')]);")?;
    let project = manager.add_project("check".into(), path.clone())?;
    manager.select_php("8.4.25")?;
    let mut settings = manager.snapshot()?.settings;
    settings.php.memory_mb = 768;
    settings.php.timezone = "UTC".into();
    settings.php.upload_mb = 80;
    settings.php.post_mb = 96;
    settings.php.execution_seconds = 35;
    settings.php.input_vars = 3000;
    settings.php.extra_ini = "serialize_precision=12".into();
    settings.php.extensions.push("gd".into());
    settings.php.opcache = true;
    let mut old = settings.php.clone();
    old.memory_mb = 256;
    old.timezone = "Europe/Berlin".into();
    settings.php_versions.insert("7.4.33".into(), old);
    settings.mysql.buffer_pool_mb = 64;
    settings.mysql.max_connections = 87;
    settings.mysql.max_packet_mb = 32;
    settings.mysql.wait_seconds = 1234;
    settings.mysql.collation = "utf8mb4_turkish_ci".into();
    settings.mysql.sql_mode = "STRICT_TRANS_TABLES,NO_ENGINE_SUBSTITUTION".into();
    settings.mysql.slow_query_log = true;
    settings.mysql.long_query_seconds = 2;
    settings.web.host_pattern = "{name}.dev.localhost".into();
    settings.web.compression = true;
    settings.web.access_log = true;
    settings.web.read_seconds = 45;
    settings.phpmyadmin.language = "en".into();
    settings.phpmyadmin.rows = 50;
    settings.phpmyadmin.login_seconds = 3600;
    let backups = home.path().join("Özel yedekler");
    fs::create_dir(&backups)?;
    settings.backups_dir = backups.to_string_lossy().into();
    manager.save_settings(settings.clone())?;
    let before = fs::read(home.path().join("config.json"))?;
    let mut invalid = settings.clone();
    invalid.php.extra_ini = "unknown_serverbond_directive=1".into();
    assert!(manager.save_settings(invalid).is_err());
    let mut invalid = settings.clone();
    invalid.php.timezone = "Mars/ServerBond".into();
    assert!(manager.save_settings(invalid).is_err());
    let mut invalid = settings.clone();
    invalid.php.extra_ini = "precision=\"unterminated".into();
    assert!(manager.save_settings(invalid).is_err());
    assert_eq!(before, fs::read(home.path().join("config.json"))?);
    manager.start("all")?;
    assert!(manager.save_settings(settings.clone()).is_err());
    let mysql = manager.mysql_query("SELECT @@innodb_buffer_pool_size, @@max_connections, @@max_allowed_packet, @@wait_timeout, @@collation_server, @@slow_query_log, @@long_query_time")?;
    assert_eq!(
        mysql,
        "67108864\t87\t33554432\t1234\tutf8mb4_turkish_ci\t1\t2.000000"
    );
    manager.create_database("check")?;
    assert_eq!(manager.mysql_query("SELECT DEFAULT_COLLATION_NAME FROM information_schema.SCHEMATA WHERE SCHEMA_NAME='check'")?, "utf8mb4_turkish_ci");
    manager.mysql_query("CREATE TABLE `check`.sample (message VARCHAR(100)); INSERT INTO `check`.sample VALUES ('Ayar ölçümü')")?;
    let backup = manager.backup_database("check")?;
    assert!(std::path::Path::new(&backup).starts_with(&backups));
    assert!(fs::read_to_string(backup)?.contains("Ayar ölçümü"));
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(15))
        .build()?;
    let request = || {
        client
            .get(format!("http://127.0.0.1:{}/", settings.web_port))
            .header("Host", "check.dev.localhost")
            .send()
    };
    let response: serde_json::Value = request()?.error_for_status()?.json()?;
    assert_eq!(response["version"], "7.4.33");
    assert_eq!(response["memory"], "256M");
    assert_eq!(response["timezone"], "Europe/Berlin");
    assert_eq!(response["upload"], "80M");
    assert_eq!(response["post"], "96M");
    assert_eq!(response["execution"], "35");
    assert_eq!(response["input"], "3000");
    assert_eq!(response["precision"], "12");
    assert_eq!(response["gd"], true);
    assert_eq!(response["opcache"], true);
    let gzip = client
        .get(format!("http://127.0.0.1:{}/", settings.web_port))
        .header("Host", "check.dev.localhost")
        .header("Accept-Encoding", "gzip")
        .send()?;
    // Tiny responses may fall below Caddy's compression threshold; inspect the validated server configuration too.
    assert!(gzip.status().is_success());
    let caddy = fs::read_to_string(home.path().join("config/Caddyfile"))?;
    assert!(caddy.contains("read_timeout 45s"));
    assert!(caddy.contains("encode zstd gzip"));
    assert!(manager.read_log("caddy")?.contains("http.log.access"));
    let pma = fs::read_to_string(manager.package_dir("phpmyadmin")?.join("config.inc.php"))?;
    assert!(pma.contains("$cfg['DefaultLang'] = 'en'"));
    assert!(pma.contains("$cfg['MaxRows'] = 50"));
    assert!(pma.contains("$cfg['LoginCookieValidity'] = 3600"));
    manager.select_project_php(&project.id, "8.4.25")?;
    let response: serde_json::Value = request()?.error_for_status()?.json()?;
    assert_eq!(response["memory"], "768M");
    assert_eq!(response["timezone"], "UTC");
    manager.stop("all")?;
    settings.phpmyadmin.enabled = false;
    manager.save_settings(settings.clone())?;
    manager.repair_php("8.4.25")?;
    manager.start("all")?;
    let response: serde_json::Value = request()?.error_for_status()?.json()?;
    assert_eq!(response["memory"], "768M");
    assert!(manager.phpmyadmin_url().is_err());
    let pma = client
        .get(format!("http://127.0.0.1:{}/", settings.web_port))
        .header("Host", "phpmyadmin.serverbond.localhost")
        .send()?
        .text()?;
    assert!(!pma.contains("pma_username"));
    assert_eq!(
        fs::read_to_string(path.join(".env"))?,
        "APP_KEY=preserve-settings-test"
    );
    manager.stop("all")?;
    drop(manager);
    let reopened = Manager::new(home.path().into())?;
    assert_eq!(reopened.snapshot()?.settings.php.memory_mb, 768);
    println!("PASS: legacy migration, PHP 7.4/8.4 profiles and extensions, actual MySQL variables, Unicode custom backup, host pattern, Caddy timeouts/logs, phpMyAdmin options/disable, invalid-setting preservation, repair/reopen persistence");
    Ok(())
}
