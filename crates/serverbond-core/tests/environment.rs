#![cfg(windows)]

use anyhow::Result;
use serverbond_core::Manager;
use std::{fs, time::Duration};

#[test]
#[ignore = "installs real packages and initializes a disposable MySQL instance"]
fn mysql_php_web_and_backups_survive_restart_and_repair() -> Result<()> {
    let home = tempfile::Builder::new()
        .prefix("ServerBond Türkçe integration ")
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
    // Optional verified download cache speeds up local runs, never shares database data.
    if let Some(cache) = std::env::var_os("SERVERBOND_TEST_CACHE") {
        for item in fs::read_dir(cache)? {
            let item = item?;
            if item.file_type()?.is_file() {
                fs::copy(
                    item.path(),
                    manager.home.join("cache").join(item.file_name()),
                )?;
            }
        }
    }
    manager.install("all")?;
    let path = manager.home.join("www/laravel-shape");
    fs::create_dir_all(path.join("public"))?;
    fs::write(path.join(".env"), "APP_KEY=preserve-this-value")?;
    fs::write(path.join("public/index.php"), "<?php header('Content-Type: application/json'); echo json_encode(['php'=>PHP_VERSION, 'uri'=>$_SERVER['REQUEST_URI']]);")?;
    let project = manager.add_project("integration".into(), path.clone())?;
    manager.start("all")?;
    manager.create_database("integration")?;
    manager.mysql_query("CREATE TABLE integration.records (id INT PRIMARY KEY, value VARCHAR(100)); INSERT INTO integration.records VALUES (1, 'İstanbul ölçüm')")?;
    assert_eq!(
        manager.mysql_query("SELECT value FROM integration.records WHERE id=1")?,
        "İstanbul ölçüm"
    );
    let backup = manager.backup_database("integration")?;
    let second = manager.backup_database("integration")?;
    assert_ne!(backup, second);
    assert!(fs::read_to_string(backup)?.contains("İstanbul ölçüm"));
    let port = manager.snapshot()?.settings.web_port;
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()?;
    let response: serde_json::Value = client
        .get(format!("http://127.0.0.1:{port}/nested?check=1"))
        .header("Host", format!("integration.localhost:{port}"))
        .send()?
        .error_for_status()?
        .json()?;
    assert_eq!(response["uri"], "/nested?check=1");
    assert_eq!(response["php"], "8.4.25");
    let pma_host = format!("phpmyadmin.serverbond.localhost:{port}");
    let login = client
        .get(format!("http://127.0.0.1:{port}/"))
        .header("Host", &pma_host)
        .send()?
        .error_for_status()?
        .text()?;
    assert!(
        login.contains("pma_username"),
        "phpMyAdmin login form missing"
    );
    assert!(!login.contains("Fatal error"));
    assert!(manager.phpmyadmin_url()?.contains(&pma_host));
    for path in [
        "config.inc.php",
        "CONFIG.INC.PHP",
        "installed.json",
        "setup/index.php",
        "vendor/autoload.php",
        "sql/create_tables.sql",
    ] {
        let response = client
            .get(format!("http://127.0.0.1:{port}/{path}"))
            .header("Host", &pma_host)
            .send()?;
        assert_eq!(response.status().as_u16(), 404, "{path} must be private");
    }
    let mixed_case_login = client
        .get(format!("http://127.0.0.1:{port}/INDEX.PHP"))
        .header("Host", &pma_host)
        .send()?
        .error_for_status()?
        .text()?;
    assert!(mixed_case_login.contains("pma_username"));
    assert!(!mixed_case_login.contains("<?php"));
    let messages = client
        .get(format!("http://127.0.0.1:{port}/js/messages.php?lang=tr"))
        .header("Host", &pma_host)
        .send()?
        .error_for_status()?
        .text()?;
    assert!(messages.contains("Messages"));
    let pma_config = fs::read_to_string(manager.package_dir("phpmyadmin")?.join("config.inc.php"))?;
    assert!(!pma_config.contains(&manager.credentials()?));
    assert!(pma_config.contains(&manager.snapshot()?.settings.mysql_port.to_string()));
    let mysql_pid = manager
        .snapshot()?
        .packages
        .iter()
        .find(|p| p.package.id == "mysql")
        .unwrap()
        .pid;
    manager.select_php("7.4.33")?;
    assert_eq!(
        manager
            .snapshot()?
            .packages
            .iter()
            .find(|p| p.package.id == "mysql")
            .unwrap()
            .pid,
        mysql_pid
    );
    assert_eq!(
        manager.mysql_query("SELECT COUNT(*) FROM integration.records")?,
        "1"
    );
    manager.stop("all")?;
    // A verified binary repair must not reinitialize or erase the database.
    manager.repair("mysql")?;
    manager.start("all")?;
    assert_eq!(
        manager.mysql_query("SELECT value FROM integration.records WHERE id=1")?,
        "İstanbul ölçüm"
    );
    manager.remove_project(&project.id)?;
    assert_eq!(
        fs::read_to_string(path.join(".env"))?,
        "APP_KEY=preserve-this-value"
    );
    manager.stop("all")?;
    let saved_home = manager.home.clone();
    drop(manager);
    let reopened = Manager::new(saved_home)?;
    reopened.start("all")?;
    assert_eq!(
        reopened.mysql_query("SELECT COUNT(*) FROM integration.records")?,
        "1"
    );
    reopened.stop("all")?;
    println!("PASS: MySQL initialization, Unicode SQL data, unique backups, FastCGI routing, PHP switch without MySQL restart, database preservation after repair/restart, .env preservation");
    Ok(())
}
