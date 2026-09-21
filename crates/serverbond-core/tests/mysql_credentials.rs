//! Real password rotation in an isolated MySQL instance, never the user's server.
#![cfg(windows)]
use serverbond_core::Manager;

#[test]
#[ignore = "Downloads MySQL and starts a disposable instance"]
fn mysql_rotation_stages_secrets_and_preserves_query_and_backup_behavior() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mut settings = manager.snapshot().unwrap().settings;
    settings.mysql_port = listener.local_addr().unwrap().port();
    drop(listener);
    manager.save_settings(settings).unwrap();
    if let Some(cache) = std::env::var_os("SERVERBOND_TEST_CACHE") {
        for entry in std::fs::read_dir(cache).unwrap().flatten() {
            if entry.file_type().unwrap().is_file() {
                std::fs::copy(
                    entry.path(),
                    home.path().join("cache").join(entry.file_name()),
                )
                .unwrap();
            }
        }
    }
    manager.install("mysql").unwrap();
    manager.start("mysql").unwrap();
    manager.create_database("rotation-check").unwrap();
    manager.mysql_query("CREATE TABLE rotation_check.probe (value VARCHAR(60)); INSERT INTO rotation_check.probe VALUES ('İstanbul')").unwrap();
    let previous = manager.credentials().unwrap();
    let next = format!("Rotated-{}", uuid::Uuid::new_v4().simple());
    let staged = home.path().join("config/mysql-password.dpapi.next");
    std::fs::create_dir(&staged).unwrap();
    assert!(manager.change_mysql_password(&next).is_err());
    assert_eq!(manager.credentials().unwrap(), previous);
    assert_eq!(manager.mysql_query("SELECT 1").unwrap(), "1");
    std::fs::remove_dir(&staged).unwrap();
    manager.change_mysql_password(&next).unwrap();
    assert_eq!(manager.credentials().unwrap(), next);
    assert!(!staged.exists());
    let encrypted = std::fs::read(home.path().join("config/mysql-password.dpapi")).unwrap();
    assert!(!String::from_utf8_lossy(&encrypted).contains(&next));
    assert_eq!(
        manager
            .mysql_query("SELECT value FROM rotation_check.probe")
            .unwrap(),
        "İstanbul"
    );
    let backup = manager.backup_database("rotation-check").unwrap();
    manager
        .mysql_query("DELETE FROM rotation_check.probe")
        .unwrap();
    manager
        .restore_database("rotation-check", backup.into())
        .unwrap();
    assert_eq!(
        manager
            .mysql_query("SELECT value FROM rotation_check.probe")
            .unwrap(),
        "İstanbul"
    );
    assert!(
        !std::fs::read_to_string(home.path().join("logs/serverbond.log"))
            .unwrap()
            .contains(&next)
    );
    manager.stop("mysql").unwrap();
}
