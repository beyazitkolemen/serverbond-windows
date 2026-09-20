use f4box_core::{model::Settings, preferences::PhpSettings, Manager};
use std::fs;

#[test]
fn old_three_port_settings_migrate_without_changing_behavior() {
    let s: Settings =
        serde_json::from_str(r#"{"webPort":18088,"mysqlPort":13316,"phpPort":19002}"#).unwrap();
    s.validate().unwrap();
    assert_eq!(s.web_port, 18088);
    assert_eq!(s.php.memory_mb, 512);
    assert_eq!(s.project_host("demo"), "demo.localhost");
    assert!(s.phpmyadmin.enabled);
    assert!(!s.start_on_launch);
    assert!(!s.web.https);
    assert_eq!(s.web.https_port, 8443);
}

#[test]
fn rejects_invalid_preferences_and_ini_overrides() {
    for ini in [
        "extension_dir=C:/other",
        "memory_limit=1M",
        "[PATH=/]\nshort_open_tag=On",
        "bad syntax",
        "serialize_precision=-1\nserialize_precision=5",
        "opcache.enable=1",
    ] {
        assert!(
            PhpSettings {
                extra_ini: ini.into(),
                ..Default::default()
            }
            .validate()
            .is_err(),
            "{ini}"
        );
    }
    for pattern in [
        "{name}.test",
        "{name}.{name}.localhost",
        "{name}\nlocalhost",
        "demo.localhost",
        "toolongprefix12345678{name}.localhost",
    ] {
        let mut s = Settings::default();
        s.web.host_pattern = pattern.into();
        assert!(s.validate().is_err(), "{pattern}");
    }
    let mut s = Settings::default();
    s.php.post_mb = 1;
    assert!(s.validate().is_err());
    let mut s = Settings::default();
    s.mysql.sql_mode = "UNKNOWN_MODE".into();
    assert!(s.validate().is_err());
    let mut s = Settings::default();
    s.php_versions.insert("9.0.0".into(), Default::default());
    assert!(s.validate().is_err());
    let s = Settings {
        backups_dir: "relative/path".into(),
        ..Default::default()
    };
    assert!(s.validate().is_err());
    assert!(serde_json::from_str::<Settings>(r#"{"rootPassword":"not-a-preference"}"#).is_err());
}

#[test]
fn settings_backup_import_and_host_change_preserve_project_files() {
    let dir = tempfile::tempdir().unwrap();
    let manager = Manager::new(dir.path().into()).unwrap();
    let project = dir.path().join("www/demo");
    fs::create_dir_all(project.join("public")).unwrap();
    fs::write(project.join("public/index.php"), "<?php echo 'ok';").unwrap();
    fs::write(project.join(".env"), "APP_URL=http://preserve.localhost").unwrap();
    manager.add_project("demo".into(), project.clone()).unwrap();
    let mut available = manager.snapshot().unwrap().settings;
    let listeners: Vec<_> = (0..4)
        .map(|_| std::net::TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    available.web_port = listeners[0].local_addr().unwrap().port();
    available.mysql_port = listeners[1].local_addr().unwrap().port();
    available.php_port = listeners[2].local_addr().unwrap().port();
    available.web.https_port = listeners[3].local_addr().unwrap().port();
    drop(listeners);
    manager.save_settings(available).unwrap();
    let original = manager.export_settings().unwrap();
    let mut changed = manager.snapshot().unwrap().settings;
    changed.web.host_pattern = "{name}.dev.localhost".into();
    changed.php_versions.insert(
        "7.4.33".into(),
        PhpSettings {
            memory_mb: 256,
            ..Default::default()
        },
    );
    changed.php.memory_mb = 768;
    manager.save_settings(changed).unwrap();
    assert_eq!(manager.previous_settings().unwrap().php.memory_mb, 512);
    let snapshot = manager.snapshot().unwrap();
    assert_eq!(snapshot.projects[0].project.host, "demo.dev.localhost");
    assert_eq!(snapshot.settings.php_for("7.4.33").memory_mb, 256);
    assert_eq!(snapshot.settings.php_for("8.4.25").memory_mb, 768);
    assert_eq!(
        fs::read_to_string(project.join(".env")).unwrap(),
        "APP_URL=http://preserve.localhost"
    );
    let before = fs::read(dir.path().join("config.json")).unwrap();
    assert!(manager.import_settings(r#"{"webPort":0}"#).is_err());
    assert_eq!(before, fs::read(dir.path().join("config.json")).unwrap());
    manager.import_settings(&original).unwrap();
    assert_eq!(
        manager.snapshot().unwrap().projects[0].project.host,
        "demo.localhost"
    );
    let exported = manager.export_settings().unwrap();
    assert!(!exported.contains("password"));
    assert!(!exported.contains("APP_URL"));
    drop(manager);
    assert_eq!(
        Manager::new(dir.path().into())
            .unwrap()
            .snapshot()
            .unwrap()
            .settings
            .php
            .memory_mb,
        512
    );
}

#[test]
fn inaccessible_destination_never_replaces_saved_preferences() {
    let dir = tempfile::tempdir().unwrap();
    let manager = Manager::new(dir.path().into()).unwrap();
    let before = fs::read(dir.path().join("config.json")).unwrap();
    let mut settings = manager.snapshot().unwrap().settings;
    settings.backups_dir = dir.path().join("does-not-exist").to_string_lossy().into();
    assert!(manager.save_settings(settings).is_err());
    assert_eq!(before, fs::read(dir.path().join("config.json")).unwrap());
}
