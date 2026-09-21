use serverbond_core::{preferences::SettingsConflict, Manager};
use std::{fs, net::TcpListener};

#[test]
fn checked_settings_preserve_local_edits_and_previous_configuration() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let (mut settings, first_revision) = manager.settings_with_revision().unwrap();
    // Keep distinct free ports reserved until the fixture is ready to save.
    let listeners: Vec<_> = (0..3)
        .map(|_| TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    settings.web_port = listeners[0].local_addr().unwrap().port();
    settings.mysql_port = listeners[1].local_addr().unwrap().port();
    settings.php_port = listeners[2].local_addr().unwrap().port();
    drop(listeners);
    let (_, revision) = manager
        .save_settings_checked(settings, &first_revision)
        .unwrap();
    let (mut stale, _) = manager.settings_with_revision().unwrap();
    let mut local = stale.clone();
    local.start_on_launch = !local.start_on_launch;
    manager.save_settings(local).unwrap();
    let config_before = fs::read(home.path().join("config.json")).unwrap();
    let previous_before = fs::read(home.path().join("config/settings.previous.json")).unwrap();
    stale.php.memory_mb = 768;
    let error = manager.save_settings_checked(stale, &revision).unwrap_err();
    assert!(error.is::<SettingsConflict>());
    assert_eq!(
        config_before,
        fs::read(home.path().join("config.json")).unwrap()
    );
    assert_eq!(
        previous_before,
        fs::read(home.path().join("config/settings.previous.json")).unwrap()
    );

    let (mut fresh, current) = manager.settings_with_revision().unwrap();
    fresh.php.memory_mb = 768;
    let (saved, next) = manager.save_settings_checked(fresh, &current).unwrap();
    assert_ne!(next, current);
    assert_eq!(saved.php.memory_mb, 768);
    assert!(saved.start_on_launch);
    assert_eq!(manager.previous_settings().unwrap().php.memory_mb, 512);
    drop(manager);
    let reopened = Manager::new(home.path().into()).unwrap();
    assert_eq!(reopened.settings_with_revision().unwrap().1, next);
    let (mut invalid, current) = reopened.settings_with_revision().unwrap();
    invalid.web_port = 0;
    let before = fs::read(home.path().join("config.json")).unwrap();
    assert!(reopened.save_settings_checked(invalid, &current).is_err());
    assert_eq!(before, fs::read(home.path().join("config.json")).unwrap());
    assert_eq!(reopened.settings_with_revision().unwrap().1, current);

    let manager = std::sync::Arc::new(reopened);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let attempts: Vec<_> = [1024, 1536]
        .into_iter()
        .map(|memory| {
            let manager = manager.clone();
            let barrier = barrier.clone();
            let expected = current.clone();
            std::thread::spawn(move || {
                let (mut candidate, _) = manager.settings_with_revision().unwrap();
                candidate.php.memory_mb = memory;
                barrier.wait();
                manager.save_settings_checked(candidate, &expected)
            })
        })
        .collect();
    let outcomes: Vec<_> = attempts
        .into_iter()
        .map(|attempt| attempt.join().unwrap())
        .collect();
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    let winner = outcomes.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(manager.settings_with_revision().unwrap().1, winner.1);
    assert_eq!(
        manager.settings_with_revision().unwrap().0.php.memory_mb,
        winner.0.php.memory_mb
    );
}
