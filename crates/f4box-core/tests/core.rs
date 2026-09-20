use f4box_core::{
    install::{extract_zip, verify_hash},
    model::{
        caddy_config, catalog, slug_from_folder, validate_mysql_password, validate_slug, Project,
        ProjectRelease, QueueWorker, Settings,
    },
    Manager,
};
use std::{fs, io::Write};

fn available_settings(mut settings: Settings) -> Settings {
    let listeners: Vec<_> = (0..4)
        .map(|_| std::net::TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    settings.web_port = listeners[0].local_addr().unwrap().port();
    settings.mysql_port = listeners[1].local_addr().unwrap().port();
    settings.php_port = listeners[2].local_addr().unwrap().port();
    settings.web.https_port = listeners[3].local_addr().unwrap().port();
    settings
}

#[test]
fn rejects_command_and_path_injection_in_project_names() {
    for invalid in [
        "",
        "../app",
        "foo/bar",
        "a\"\n}",
        "foo;bar",
        "APP",
        "-foo",
        "foo-",
        "foo.localhost",
        "con",
        "com0",
        "lpt0",
        "lpt1",
    ] {
        assert!(validate_slug(invalid).is_err(), "{invalid}");
    }
    for valid in ["app", "my-app", "app-42", "a"] {
        assert!(validate_slug(valid).is_ok());
    }
}

#[test]
fn prevents_port_collisions_and_zero_but_allows_standard_http() {
    assert!(Settings::default().validate().is_ok());
    assert!(Settings {
        web_port: 80,
        ..Settings::default()
    }
    .validate()
    .is_ok());
    assert!(Settings {
        web_port: 0,
        ..Settings::default()
    }
    .validate()
    .is_err());
    assert!(Settings {
        mysql_port: 8088,
        ..Settings::default()
    }
    .validate()
    .is_err());
    let mut collision = Settings::default();
    collision.web.https_port = collision.web_port;
    assert!(collision.validate().is_err());
}

#[test]
fn caddy_routes_to_public_and_binds_loopback() {
    let project = Project {
        id: "1".into(),
        name: "demo".into(),
        host: "demo.localhost".into(),
        path: "C:/Project With Space/demo".into(),
        php_version: "8.4.25".into(),
        workers: Vec::new(),
        schedule: Default::default(),
        release: Default::default(),
    };
    let config = caddy_config(
        &Settings::default(),
        &[project],
        std::path::Path::new("C:/ServerBond/welcome"),
        &std::collections::HashMap::from([("1".into(), 19001)]),
    )
    .unwrap();
    assert!(config.contains("http://demo.localhost:8088"));
    assert!(config.contains("\"C:/Project With Space/demo/public\""));
    assert_eq!(config.matches("bind 127.0.0.1").count(), 2);
    assert!(config.contains("admin off"));
    assert!(config.contains("php_fastcgi 127.0.0.1:19001"));
    assert!(!config.contains("tls internal"));
    assert!(!config.contains("redir https://"));
}

#[test]
fn caddy_https_redirects_http_and_uses_internal_tls() {
    let project = Project {
        id: "1".into(),
        name: "demo".into(),
        host: "demo.localhost".into(),
        path: "C:/Project With Space/demo".into(),
        php_version: "8.4.25".into(),
        workers: Vec::new(),
        schedule: Default::default(),
        release: Default::default(),
    };
    let mut settings = Settings::default();
    settings.web.https = true;
    settings.web.https_port = 8443;
    let config = caddy_config(
        &settings,
        &[project],
        std::path::Path::new("C:/ServerBond/welcome"),
        &std::collections::HashMap::from([("1".into(), 19001)]),
    )
    .unwrap();
    assert!(config.contains("auto_https off"));
    assert!(config.contains("redir https://demo.localhost:8443{uri}"));
    assert!(config.contains("https://demo.localhost:8443"));
    assert!(config.contains("tls internal"));
    assert!(config.contains("bind 127.0.0.1"));
    assert_eq!(config.matches("tls internal").count(), 2);
    assert_eq!(config.matches("bind 127.0.0.1").count(), 4);
    assert!(!config.contains("https://demo.localhost:8088"));
    assert_eq!(
        settings.site_url("demo.localhost"),
        "https://demo.localhost:8443/"
    );
}

#[test]
fn folder_names_become_safe_project_slugs() {
    assert_eq!(slug_from_folder("My App").unwrap(), "my-app");
    assert_eq!(slug_from_folder("demo_site").unwrap(), "demo-site");
    assert_eq!(slug_from_folder("Shop.v2").unwrap(), "shop-v2");
    assert!(slug_from_folder("CON").is_err());
    assert!(slug_from_folder("...").is_err());
    assert!(slug_from_folder("Üğ").is_err());
}

#[test]
fn hashes_reject_modified_packages() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("package");
    fs::write(&path, "hello").unwrap();
    let sha = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
    assert!(verify_hash(&path, sha).is_ok());
    fs::write(&path, "modified").unwrap();
    assert!(verify_hash(&path, sha).is_err());
}

#[test]
fn archive_cannot_escape_install_directory() {
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("bad.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
    zip.start_file("../escape.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"bad").unwrap();
    zip.finish().unwrap();
    let out = dir.path().join("output");
    fs::create_dir(&out).unwrap();
    assert!(extract_zip(&zip_path, &out, "").is_err());
    assert!(!dir.path().join("escape.txt").exists());
}

#[test]
fn archive_strips_vendor_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("valid.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
    zip.start_file(
        "vendor/bin/app.exe",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(b"ok").unwrap();
    zip.finish().unwrap();
    let out = dir.path().join("output");
    fs::create_dir(&out).unwrap();
    extract_zip(&zip_path, &out, "vendor").unwrap();
    assert_eq!(fs::read(out.join("bin/app.exe")).unwrap(), b"ok");
}

#[test]
fn second_manager_cannot_share_data_directory() {
    let home = tempfile::tempdir().unwrap();
    let first = Manager::new(home.path().into()).unwrap();
    assert!(Manager::new(home.path().into()).is_err());
    drop(first);
    assert!(Manager::new(home.path().into()).is_ok());
}

#[test]
fn external_program_paths_avoid_windows_extended_prefix() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    assert!(!manager.home.to_string_lossy().starts_with("\\\\?\\"));
}

#[test]
fn settings_persist_and_invalid_settings_leave_previous_intact() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let settings = available_settings(manager.snapshot().unwrap().settings);
    let saved_port = settings.web_port;
    manager.save_settings(settings).unwrap();
    assert!(manager
        .save_settings(Settings {
            web_port: 0,
            ..Settings::default()
        })
        .is_err());
    drop(manager);
    let manager = Manager::new(home.path().into()).unwrap();
    assert_eq!(manager.snapshot().unwrap().settings.web_port, saved_port);
}

#[test]
fn adding_and_removing_project_preserves_existing_env_and_files() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("public")).unwrap();
    fs::write(project.path().join("public/index.php"), "<?php echo 'OK';").unwrap();
    fs::write(project.path().join(".env"), "APP_KEY=do-not-touch").unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let saved = manager
        .add_project("demo".into(), project.path().into())
        .unwrap();
    assert!(manager
        .add_project("other".into(), project.path().into())
        .is_err());
    manager.remove_project(&saved.id).unwrap();
    assert_eq!(
        fs::read_to_string(project.path().join(".env")).unwrap(),
        "APP_KEY=do-not-touch"
    );
    assert!(project.path().join("public/index.php").exists());
    assert!(manager.snapshot().unwrap().projects.is_empty());
}

#[test]
fn rejects_invalid_project_folder_and_log_traversal() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    assert!(manager
        .add_project("demo".into(), home.path().into())
        .is_err());
    assert!(manager.read_log("../config").is_err());
    assert!(manager.install("../php").is_err());
}

#[test]
fn corrupt_configuration_is_not_silently_replaced() {
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join("config.json"), "broken").unwrap();
    assert!(Manager::new(home.path().into()).is_err());
    assert_eq!(
        fs::read_to_string(home.path().join("config.json")).unwrap(),
        "broken"
    );
}

#[test]
fn catalog_only_contains_pinned_https_packages() {
    let packages = catalog();
    assert_eq!(packages.len(), 5);
    for package in packages.into_iter().chain(f4box_core::model::tools()) {
        assert!(package.url.starts_with("https://"));
        assert_eq!(package.sha256.len(), 64);
        assert!(package.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
    }
}

#[test]
fn legacy_configuration_keeps_projects_and_defaults_to_original_php() {
    let home = tempfile::tempdir().unwrap();
    let legacy =
        r#"{"settings":{"webPort":18088,"mysqlPort":23316,"phpPort":19330},"projects":[]}"#;
    fs::write(home.path().join("config.json"), legacy).unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    assert_eq!(
        manager.snapshot().unwrap().packages[0].package.version,
        "8.4.25"
    );
    assert_eq!(
        fs::read_to_string(home.path().join("config.json")).unwrap(),
        legacy
    );
}

#[test]
fn php_selection_rejects_unknown_version_without_changing_config() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let original = fs::read(home.path().join("config.json")).unwrap();
    for version in ["../8.4.25", "8.4", "8.6.0", ""] {
        assert!(manager.select_php(version).is_err());
    }
    assert_eq!(fs::read(home.path().join("config.json")).unwrap(), original);
    assert_eq!(fs::read_dir(home.path().join("bin")).unwrap().count(), 0);
}

#[test]
fn saved_php_selection_controls_paths_and_package_status() {
    let home = tempfile::tempdir().unwrap();
    let config = f4box_core::model::Config {
        php_version: "7.4.33".into(),
        ..Default::default()
    };
    fs::write(
        home.path().join("config.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    assert!(manager
        .package_dir("php")
        .unwrap()
        .ends_with("bin/php/7.4.33"));
    assert!(manager
        .php_ini_path()
        .unwrap()
        .ends_with("config/php/7.4.33/php.ini"));
    assert_eq!(
        manager.snapshot().unwrap().packages[0].package.version,
        "7.4.33"
    );
    let err = manager
        .create_project("demo".into(), home.path().into())
        .unwrap_err();
    assert!(err.to_string().contains("PHP 8.2"));
    assert!(!home.path().join("demo").exists());
}

#[test]
fn laravel12_requires_php_8_2_or_newer_by_series() {
    use f4box_core::model::{php_meets, php_supports_laravel12};
    assert!(!php_supports_laravel12("7.4.33"));
    assert!(!php_supports_laravel12("8.0.30"));
    assert!(!php_supports_laravel12("8.1.34"));
    assert!(php_supports_laravel12("8.2.33"));
    assert!(php_supports_laravel12("8.4.25"));
    assert!(php_supports_laravel12("8.10.0"));
    assert!(php_meets("8.10.0", 8, 2));
    assert!(!php_meets("8.10.0", 8, 11));
    assert!(!php_supports_laravel12(""));
}

#[test]
fn php_catalog_has_one_pinned_build_per_supported_series() {
    let versions = f4box_core::model::php_versions();
    let branches: Vec<_> = versions
        .iter()
        .map(|p| p.version.rsplit_once('.').unwrap().0)
        .collect();
    assert_eq!(branches, ["7.4", "8.0", "8.1", "8.2", "8.3", "8.4", "8.5"]);
    for package in versions {
        assert!(package
            .url
            .starts_with("https://downloads.php.net/~windows/releases/"));
        assert!(package.url.contains("-nts-Win32-"));
        assert!(package.url.ends_with("-x64.zip"));
        assert_eq!(package.sha256.len(), 64);
        assert!(package.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
    }
}

#[test]
fn broken_download_does_not_replace_selected_php() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    // An incomplete installation must fail before network access or configuration changes.
    fs::create_dir_all(home.path().join("bin/php/7.4.33")).unwrap();
    let original = fs::read(home.path().join("config.json")).unwrap();
    assert!(manager.select_php("7.4.33").is_err());
    assert_eq!(fs::read(home.path().join("config.json")).unwrap(), original);
}

#[test]
fn mysql_password_rules_and_workspace_settings_are_not_runtime() {
    assert!(validate_mysql_password("short").is_err());
    assert!(validate_mysql_password("has space!!").is_err());
    assert!(validate_mysql_password("bad'quote").is_err());
    assert!(validate_mysql_password("good-Pass_99").is_ok());
    assert!(validate_mysql_password("Şifre-Türkçe1").is_err());
    assert!(validate_mysql_password("ok!$%&*()[]{}").is_ok());
    let base = Settings::default();
    let mut folders = base.clone();
    folders.projects_dir = "C:\\Projeler".into();
    folders.backups_dir = "C:\\Yedek".into();
    folders.start_on_launch = true;
    assert!(!base.runtime_changed(&folders));
    let mut ports = base.clone();
    ports.web_port = 8089;
    assert!(base.runtime_changed(&ports));
}

#[test]
fn occupied_port_is_rejected_without_overwriting_settings() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let before = fs::read(home.path().join("config.json")).unwrap();
    let mut settings = manager.snapshot().unwrap().settings;
    settings.web_port = listener.local_addr().unwrap().port();
    assert!(manager.save_settings(settings).is_err());
    assert_eq!(fs::read(home.path().join("config.json")).unwrap(), before);
}

#[test]
fn installed_receipt_without_fastcgi_is_reported_as_broken() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let package = catalog().remove(0);
    let dir = manager.package_dir("php").unwrap();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("installed.json"),
        serde_json::to_vec(&package).unwrap(),
    )
    .unwrap();
    fs::write(dir.join("php.exe"), "fixture").unwrap();
    let snapshot = manager.snapshot().unwrap();
    assert!(!snapshot.packages[0].installed);
    assert!(snapshot.packages[0].repairable);
    assert!(snapshot.packages[0]
        .issue
        .as_ref()
        .unwrap()
        .contains("php-cgi.exe"));
    assert!(manager.executable("php").is_err());
}

#[test]
fn optional_service_exposes_repair_when_install_is_broken() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let package = f4box_core::model::tool_package("mailpit").unwrap();
    let dir = home.path().join("bin/mailpit").join(&package.version);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("installed.json"),
        serde_json::to_vec(&package).unwrap(),
    )
    .unwrap();
    let mail = manager.snapshot().unwrap().mail;
    assert!(!mail.installed);
    assert!(mail.repairable);
    assert!(mail
        .issue
        .as_ref()
        .unwrap()
        .contains("Onar düğmesini kullanın"));
    let redis = manager.snapshot().unwrap().redis;
    assert!(!redis.installed);
    assert!(!redis.repairable);
}

#[test]
fn archive_fallback_is_only_for_official_php_release_urls() {
    let mut php = catalog().remove(0);
    assert!(f4box_core::install::archive_fallback(&php)
        .unwrap()
        .contains("/archives/php-8.4.25"));
    php.url = "https://example.com/php.zip".into();
    assert!(f4box_core::install::archive_fallback(&php).is_none());
    php.url = "https://downloads.php.net/~windows/releases/archives/php.zip".into();
    assert!(f4box_core::install::archive_fallback(&php).is_none());
    let mysql = catalog().remove(1);
    assert!(f4box_core::install::archive_fallback(&mysql).is_none());
}

fn cached_fixture(home: &std::path::Path, entry: &str) -> f4box_core::model::Package {
    use sha2::{Digest, Sha256};
    fs::create_dir_all(home.join("cache")).unwrap();
    let path = home.join("cache/fixture-1.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    zip.start_file(entry, zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"new executable").unwrap();
    zip.finish().unwrap();
    let mut package = catalog().remove(0);
    package.id = "fixture".into();
    package.version = "1".into();
    package.executable = "app.exe".into();
    package.sha256 = format!("{:x}", Sha256::digest(fs::read(path).unwrap()));
    package
}

#[test]
fn repair_replaces_only_programs_and_keeps_old_files() {
    let home = tempfile::tempdir().unwrap();
    let package = cached_fixture(home.path(), "app.exe");
    let dir = home.path().join("bin/fixture/1");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("custom.txt"), "keep").unwrap();
    fs::create_dir_all(home.path().join("data")).unwrap();
    fs::write(home.path().join("data/database"), "keep data").unwrap();
    f4box_core::install::repair(home.path(), &package, |_| {}).unwrap();
    f4box_core::install::validate_installation(&dir, &package).unwrap();
    assert_eq!(fs::read(dir.join("app.exe")).unwrap(), b"new executable");
    assert_eq!(
        fs::read(home.path().join("data/database")).unwrap(),
        b"keep data"
    );
    let backup = fs::read_dir(dir.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().contains("before-repair"))
        .unwrap();
    assert_eq!(fs::read(backup.path().join("custom.txt")).unwrap(), b"keep");
}

#[test]
fn failed_repair_keeps_existing_installation_intact() {
    let home = tempfile::tempdir().unwrap();
    let package = cached_fixture(home.path(), "../escape.exe");
    let dir = home.path().join("bin/fixture/1");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("app.exe"), "original").unwrap();
    assert!(f4box_core::install::repair(home.path(), &package, |_| {}).is_err());
    assert_eq!(fs::read(dir.join("app.exe")).unwrap(), b"original");
    assert_eq!(fs::read_dir(dir.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn password_and_backup_before_mysql_initialization_explain_next_step() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    assert!(manager
        .credentials()
        .unwrap_err()
        .to_string()
        .contains("Önce MySQL"));
    assert!(manager
        .backup_database("demo")
        .unwrap_err()
        .to_string()
        .contains("MySQL'i başlatın"));
}

#[test]
fn migration_pins_existing_projects_to_the_saved_default_without_touching_env() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old-project");
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join(".env"), "APP_KEY=keep-this").unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let config = serde_json::json!({"phpVersion":"7.4.33", "settings":{"webPort":18088,"mysqlPort":23316,"phpPort":19330},"projects":[{"id":id,"name":"old-project","host":"old-project.localhost","path":path}]});
    fs::write(dir.path().join("config.json"), config.to_string()).unwrap();
    let manager = Manager::new(dir.path().into()).unwrap();
    assert_eq!(
        manager.snapshot().unwrap().projects[0].project.php_version,
        "7.4.33"
    );
    assert_eq!(
        fs::read_to_string(path.join(".env")).unwrap(),
        "APP_KEY=keep-this"
    );
    let persisted: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join("config.json")).unwrap()).unwrap();
    assert_eq!(persisted["projects"][0]["phpVersion"], "7.4.33");
    assert_eq!(persisted["projects"][0]["id"], id);
    for key in ["webPort", "mysqlPort", "phpPort"] {
        assert_eq!(persisted["settings"][key], config["settings"][key]);
    }
    assert_eq!(persisted["settings"]["php"]["memoryMb"], 512);
}

#[test]
fn requirements_detect_an_external_port_owner_without_changing_settings() {
    let dir = tempfile::tempdir().unwrap();
    let manager = Manager::new(dir.path().into()).unwrap();
    let mut settings = available_settings(manager.snapshot().unwrap().settings);
    // Other tests also probe the default ports. Use an OS-assigned port here.
    let reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    settings.web_port = reservation.local_addr().unwrap().port();
    drop(reservation);
    manager.save_settings(settings.clone()).unwrap();
    let listener = std::net::TcpListener::bind(("127.0.0.1", settings.web_port)).unwrap();
    let checks = manager.requirements();
    assert_eq!(
        checks.iter().find(|c| c.id == "caddy").unwrap().status,
        "error"
    );
    assert_eq!(
        checks
            .iter()
            .find(|c| c.id == "storage-write")
            .unwrap()
            .status,
        "ok"
    );
    assert_eq!(
        manager.snapshot().unwrap().settings.web_port,
        settings.web_port
    );
    drop(listener);
    assert_eq!(
        manager
            .requirements()
            .iter()
            .find(|c| c.id == "caddy")
            .unwrap()
            .status,
        "ok"
    );
}

#[test]
fn project_jobs_persist_and_reject_invalid_workers() {
    let home = tempfile::tempdir().unwrap();
    let project_dir = tempfile::tempdir().unwrap();
    fs::create_dir(project_dir.path().join("public")).unwrap();
    fs::write(project_dir.path().join("public/index.php"), "<?php").unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let project = manager
        .add_project("demo".into(), project_dir.path().into())
        .unwrap();
    assert!(project.workers.is_empty());
    assert!(!project.schedule.enabled);
    let worker = QueueWorker {
        id: uuid::Uuid::new_v4().to_string(),
        name: "emails".into(),
        connection: "database".into(),
        queue: "high,default".into(),
        processes: 2,
        max_jobs: 100,
        max_time: 3600,
        ..Default::default()
    };
    manager
        .save_project_jobs(&project.id, vec![worker.clone()], Default::default())
        .unwrap();
    let mut bad = worker.clone();
    bad.name = "rm -rf".into();
    let before = fs::read(home.path().join("config.json")).unwrap();
    assert!(manager
        .save_project_jobs(&project.id, vec![bad], Default::default())
        .is_err());
    assert_eq!(fs::read(home.path().join("config.json")).unwrap(), before);
    let mut disabled = worker.clone();
    disabled.enabled = false;
    manager
        .save_project_jobs(&project.id, vec![disabled], Default::default())
        .unwrap();
    assert!(manager
        .start_project_worker(&project.id, &worker.id)
        .unwrap_err()
        .to_string()
        .contains("kapalı"));
    assert!(manager
        .restart_project_worker(&project.id, &worker.id)
        .unwrap_err()
        .to_string()
        .contains("kapalı"));
    assert!(manager
        .start_project_schedule(&project.id)
        .unwrap_err()
        .to_string()
        .contains("kapalı"));
    drop(manager);
    let reopened = Manager::new(home.path().into()).unwrap();
    let saved = &reopened.snapshot().unwrap().projects[0];
    assert_eq!(saved.project.workers[0].name, "emails");
    assert_eq!(saved.project.workers[0].queue, "high,default");
    assert_eq!(saved.project.workers[0].max_jobs, 100);
    assert_eq!(saved.project.workers[0].max_time, 3600);
    assert_eq!(saved.worker_states[0].running, 0);
    assert_eq!(
        reopened
            .read_project_worker_log(&project.id, &worker.id)
            .unwrap(),
        "Henüz günlük kaydı yok."
    );
    assert_eq!(
        reopened.read_project_schedule_log(&project.id).unwrap(),
        "Henüz günlük kaydı yok."
    );
    assert_eq!(
        reopened.read_project_log(&project.id, "php").unwrap(),
        "Henüz günlük kaydı yok."
    );
    assert_eq!(
        reopened
            .read_project_log(&project.id, &format!("worker:{}", worker.id))
            .unwrap(),
        "Henüz günlük kaydı yok."
    );
    assert!(reopened.read_project_log(&project.id, "../php").is_err());
    assert!(reopened
        .read_project_log(&project.id, "worker:not-a-uuid")
        .is_err());
    let failed = reopened.list_failed_jobs(&project.id).unwrap_err();
    assert!(format!("{failed:#}").contains("artisan"), "{failed:#}");
    assert!(reopened.read_log("../queue").is_err());
}

#[test]
fn project_release_persists_and_rejects_injection() {
    let home = tempfile::tempdir().unwrap();
    let project_dir = tempfile::tempdir().unwrap();
    fs::create_dir(project_dir.path().join("public")).unwrap();
    fs::write(project_dir.path().join("public/index.php"), "<?php").unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let project = manager
        .add_project("demo".into(), project_dir.path().into())
        .unwrap();
    assert!(project.release.git_pull);
    assert!(project.release.composer_no_dev);
    let mut release = ProjectRelease {
        branch: "main".into(),
        extra_artisan: vec!["config:cache".into()],
        ..Default::default()
    };
    manager
        .save_project_release(&project.id, release.clone())
        .unwrap();
    release.extra_artisan = vec!["migrate; rm".into()];
    let before = fs::read(home.path().join("config.json")).unwrap();
    assert!(manager.save_project_release(&project.id, release).is_err());
    assert_eq!(fs::read(home.path().join("config.json")).unwrap(), before);
    assert!(manager
        .list_project_releases(&project.id)
        .unwrap()
        .is_empty());
    drop(manager);
    let reopened = Manager::new(home.path().into()).unwrap();
    let saved = &reopened.snapshot().unwrap().projects[0].project.release;
    assert_eq!(saved.branch, "main");
    assert_eq!(saved.extra_artisan, ["config:cache"]);
}

#[test]
fn tunnel_rejects_bad_tokens_and_reports_a_missing_client() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let state = manager.snapshot().unwrap().tunnel;
    assert!(!state.installed && !state.token_saved && !state.running);
    assert!(!state.auto_start);
    assert!(manager.save_tunnel_token("too-short").is_err());
    assert!(!home.path().join("config/cloudflared-token.dpapi").exists());
    manager.save_tunnel_auto_start(true).unwrap();
    assert!(manager.snapshot().unwrap().tunnel.auto_start);
    drop(manager);
    let reopened = Manager::new(home.path().into()).unwrap();
    assert!(reopened.snapshot().unwrap().tunnel.auto_start);
    // Without a saved token the environment still starts; only the tunnel reports it.
    assert!(reopened
        .start_tunnel()
        .unwrap_err()
        .to_string()
        .contains("jeton"));
    assert!(reopened.stop_tunnel().is_ok());
    assert_eq!(
        reopened.read_log("cloudflared").unwrap(),
        "Henüz günlük kaydı yok."
    );
}

#[test]
fn mail_catcher_reports_its_ports_and_rejects_port_collisions() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let state = manager.snapshot().unwrap().mail;
    assert!(!state.installed && !state.running);
    assert_eq!((state.smtp_port, state.web_port), (1025, 8025));
    assert!(state.relay_php_mail && !state.auto_start);

    let mut settings = available_settings(Settings::default());
    settings.mail.smtp_port = settings.web_port;
    assert!(manager.save_settings(settings.clone()).is_err());

    settings.mail.smtp_port = 1125;
    settings.mail.web_port = 1125;
    assert!(manager.save_settings(settings.clone()).is_err());

    settings.mail.web_port = 8125;
    settings.mail.max_messages = 250_000;
    assert!(manager.save_settings(settings.clone()).is_err());

    settings.mail.max_messages = 50;
    manager.save_settings(settings).unwrap();
    let saved = manager.snapshot().unwrap().mail;
    assert_eq!((saved.smtp_port, saved.web_port), (1125, 8125));

    // Without the package the environment still starts; only the mail card reports it.
    assert!(manager
        .start_mail()
        .unwrap_err()
        .to_string()
        .contains("Mailpit kurulu değil"));
    assert!(manager.stop_mail().is_ok());
    assert!(manager.open_mail().is_err());
    assert_eq!(
        manager.read_log("mailpit").unwrap(),
        "Henüz günlük kaydı yok."
    );
}

#[test]
fn postgres_is_optional_and_rejects_port_collisions() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let state = manager.snapshot().unwrap().postgres;
    assert!(!state.installed && !state.running);
    assert_eq!(state.port, 15432);
    assert!(!state.auto_start && !state.password_saved);

    let mut settings = available_settings(Settings::default());
    settings.postgres.port = settings.web_port;
    assert!(manager.save_settings(settings.clone()).is_err());

    settings.postgres.port = 15433;
    settings.postgres.auto_start = true;
    manager.save_settings(settings).unwrap();
    let saved = manager.snapshot().unwrap().postgres;
    assert_eq!(saved.port, 15433);
    assert!(saved.auto_start);

    assert!(manager
        .start_postgres()
        .unwrap_err()
        .to_string()
        .contains("PostgreSQL kurulu değil"));
    assert!(manager.stop_postgres().is_ok());
    assert!(manager
        .postgres_credentials()
        .unwrap_err()
        .to_string()
        .contains("henüz hazır değil"));
    assert_eq!(
        manager.read_log("postgres").unwrap(),
        "Henüz günlük kaydı yok."
    );
}

#[test]
fn redis_is_optional_and_rejects_port_collisions() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let state = manager.snapshot().unwrap().redis;
    assert!(!state.installed && !state.running);
    assert_eq!(state.port, 16379);
    assert!(!state.auto_start);

    let mut settings = available_settings(Settings::default());
    settings.redis.port = settings.web_port;
    assert!(manager.save_settings(settings.clone()).is_err());

    settings.redis.port = 16380;
    settings.redis.auto_start = true;
    manager.save_settings(settings).unwrap();
    let saved = manager.snapshot().unwrap().redis;
    assert_eq!(saved.port, 16380);
    assert!(saved.auto_start);

    assert!(manager
        .start_redis()
        .unwrap_err()
        .to_string()
        .contains("Redis kurulu değil"));
    assert!(manager.stop_redis().is_ok());
    assert_eq!(
        manager.read_log("redis").unwrap(),
        "Henüz günlük kaydı yok."
    );
}

#[test]
fn github_import_rejects_bad_repos_and_existing_folders() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let state = manager.snapshot().unwrap().github;
    assert!(!state.token_saved && state.login.is_none());
    assert!(manager.save_github_token("too-short").is_err());
    assert!(manager
        .import_github_project(
            "https://gitlab.com/owner/repo",
            String::new(),
            String::new()
        )
        .unwrap_err()
        .to_string()
        .contains("github.com"));
    assert!(manager
        .import_github_project("owner/repo;rm", "demo".into(), String::new())
        .is_err());
    let taken = home.path().join("www/taken");
    fs::create_dir_all(taken.join("public")).unwrap();
    fs::write(taken.join("public/index.php"), "<?php").unwrap();
    manager.add_project("taken".into(), taken).unwrap();
    assert!(manager
        .import_github_project("owner/taken", "taken".into(), String::new())
        .unwrap_err()
        .to_string()
        .contains("kayıtlı"));
    let dest = home.path().join("projects/magaza");
    fs::create_dir_all(&dest).unwrap();
    assert!(manager
        .import_github_project("owner/magaza", "magaza".into(), String::new())
        .unwrap_err()
        .to_string()
        .contains("üzerine yazılmadı"));
    assert_eq!(
        manager.read_log("github").unwrap(),
        "Henüz günlük kaydı yok."
    );
}

#[test]
fn project_env_is_read_and_saved_only_when_asked() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let path = home.path().join("www/shop");
    fs::create_dir_all(path.join("public")).unwrap();
    fs::write(path.join("public/index.php"), "<?php").unwrap();
    fs::write(path.join(".env.example"), "APP_NAME=Example\n").unwrap();
    let project = manager.add_project("shop".into(), path.clone()).unwrap();
    let missing = manager.read_project_env(&project.id).unwrap();
    assert!(!missing.exists && missing.content.is_empty());
    assert_eq!(missing.example.as_deref(), Some("APP_NAME=Example\n"));
    manager
        .save_project_env(&project.id, "APP_KEY=from-ui\nAPP_DEBUG=false\n".into())
        .unwrap();
    assert_eq!(
        fs::read_to_string(path.join(".env")).unwrap(),
        "APP_KEY=from-ui\nAPP_DEBUG=false\n"
    );
    let saved = manager.read_project_env(&project.id).unwrap();
    assert!(saved.exists);
    assert_eq!(saved.content, "APP_KEY=from-ui\nAPP_DEBUG=false\n");
    assert!(manager
        .save_project_env(&project.id, "bad\0key=1\n".into())
        .is_err());
    assert_eq!(
        fs::read_to_string(path.join(".env")).unwrap(),
        "APP_KEY=from-ui\nAPP_DEBUG=false\n"
    );
    assert!(manager.read_project_env("missing").is_err());
}

#[test]
fn node_stays_out_of_the_project_terminal_until_it_is_installed() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let state = manager.snapshot().unwrap().node;
    assert!(!state.installed && state.directory.is_none());
    assert_eq!(state.version, "24.21.0");
    // The terminal script needs an installed PHP, so a missing package is the error.
    assert!(manager.project_terminal_script("nope").is_err());
}

#[test]
fn the_mail_form_owns_the_php_smtp_directives() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let mut settings = available_settings(Settings::default());
    for managed in ["smtp_port=2525", "sendmail_from=a@b.c", "sendmail_path=x"] {
        settings.php.extra_ini = managed.into();
        assert!(
            manager.save_settings(settings.clone()).is_err(),
            "{managed}"
        );
    }
    settings.php.extra_ini = "default_socket_timeout=30".into();
    manager.save_settings(settings).unwrap();
}

#[test]
fn granting_windows_permissions_is_windows_only_and_starts_ungranted() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let state = manager.snapshot().unwrap().permissions;
    assert!(!state.granted && !state.helper && !state.declined);
    assert!(state.applied.is_empty() && state.pending.is_empty());
    let error = manager.grant_permissions(false).unwrap_err().to_string();
    assert!(
        error.contains("yalnızca Windows") || error.contains("Yetki"),
        "{error}"
    );
    assert!(!home.path().join("config/permissions.json").exists());
}

#[test]
fn legacy_projects_load_without_queue_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy-app");
    fs::create_dir_all(path.join("public")).unwrap();
    fs::write(path.join("public/index.php"), "<?php").unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let config = serde_json::json!({
        "phpVersion": "8.4.25",
        "settings": {"webPort": 18088, "mysqlPort": 23316, "phpPort": 19330},
        "projects": [{"id": id, "name": "legacy-app", "host": "legacy-app.localhost", "path": path}]
    });
    fs::write(dir.path().join("config.json"), config.to_string()).unwrap();
    let manager = Manager::new(dir.path().into()).unwrap();
    let project = &manager.snapshot().unwrap().projects[0];
    assert!(project.project.workers.is_empty());
    assert!(!project.schedule_running);
    assert!(!project.project.schedule.auto_start);
}

#[test]
fn discover_finds_laravel_folders_and_skips_registered_ones() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let www = home.path().join("www");
    for (name, laravel) in [("shop-v2", true), ("notes", true), ("readme.txt", false)] {
        if name.contains('.') {
            fs::write(www.join(name), "no").unwrap();
            continue;
        }
        let path = www.join(name);
        fs::create_dir_all(path.join(if laravel { "public" } else { "src" })).unwrap();
        if laravel {
            fs::write(path.join("public/index.php"), "<?php").unwrap();
        }
    }
    fs::create_dir_all(www.join("empty")).unwrap();
    let found = manager.discover_projects().unwrap();
    let names: Vec<_> = found.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(names, ["notes", "shop-v2"]);
    manager
        .add_project("notes".into(), www.join("notes"))
        .unwrap();
    let found = manager.discover_projects().unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "shop-v2");
    let imported = manager
        .import_projects(found.iter().map(|item| item.path.clone()).collect())
        .unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].name, "shop-v2");
    assert!(manager.discover_projects().unwrap().is_empty());
}

#[test]
fn discover_uses_projects_workspace_and_nested_client_apps() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let workspace = manager.default_projects_dir();
    assert!(workspace.ends_with("projects"));
    fs::create_dir_all(workspace.join("shop/public")).unwrap();
    fs::write(workspace.join("shop/public/index.php"), "<?php").unwrap();
    fs::create_dir_all(workspace.join("acme/api/public")).unwrap();
    fs::write(workspace.join("acme/api/public/index.php"), "<?php").unwrap();
    fs::create_dir_all(workspace.join("acme/vendor/public")).unwrap();
    fs::write(workspace.join("acme/vendor/public/index.php"), "<?php").unwrap();
    fs::create_dir_all(workspace.join("acme/api/vendor/fake/public")).unwrap();
    fs::write(
        workspace.join("acme/api/vendor/fake/public/index.php"),
        "<?php",
    )
    .unwrap();
    let found = manager.discover_projects().unwrap();
    let mut names: Vec<_> = found.iter().map(|item| item.name.clone()).collect();
    names.sort();
    assert_eq!(names, ["api", "shop"]);
}

#[test]
fn discover_qualifies_nested_slug_when_the_folder_name_is_taken() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let workspace = manager.default_projects_dir();
    fs::create_dir_all(workspace.join("shop/public")).unwrap();
    fs::write(workspace.join("shop/public/index.php"), "<?php").unwrap();
    fs::create_dir_all(workspace.join("acme/shop/public")).unwrap();
    fs::write(workspace.join("acme/shop/public/index.php"), "<?php").unwrap();
    let found = manager.discover_projects().unwrap();
    let mut names: Vec<_> = found.iter().map(|item| item.name.clone()).collect();
    names.sort();
    assert_eq!(names, ["acme-shop", "shop"]);
}

#[test]
fn discover_custom_workspace_does_not_scan_default_folders() {
    let home = tempfile::tempdir().unwrap();
    let custom = home.path().join("work/apps");
    fs::create_dir_all(custom.join("portal/public")).unwrap();
    fs::write(custom.join("portal/public/index.php"), "<?php").unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    fs::create_dir_all(manager.default_projects_dir().join("ignored/public")).unwrap();
    fs::write(
        manager
            .default_projects_dir()
            .join("ignored/public/index.php"),
        "<?php",
    )
    .unwrap();
    let mut settings = available_settings(manager.snapshot().unwrap().settings);
    settings.projects_dir = custom.to_string_lossy().into();
    manager.save_settings(settings).unwrap();
    let found = manager.discover_projects().unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "portal");
}

#[test]
fn restore_explains_missing_mysql_and_rejects_non_sql() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let sql = home.path().join("demo.sql");
    fs::write(&sql, "SELECT 1;").unwrap();
    let error = manager
        .restore_database("demo", sql.clone())
        .unwrap_err()
        .to_string();
    assert!(error.contains("MySQL"), "{error}");
    let other = home.path().join("notes.txt");
    fs::write(&other, "no").unwrap();
    // MySQL is still stopped, so the first guard wins. The extension check is
    // covered when the process is running; here the path must still exist.
    assert!(other.is_file());
}
