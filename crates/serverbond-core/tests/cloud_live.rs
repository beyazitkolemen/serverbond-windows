//! Opt-in integration with a running local Laravel + Reverb, using an isolated home.
#![cfg(windows)]
use serverbond_core::Manager;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
#[test]
#[ignore = "Requires local Cloud/Reverb and SERVERBOND_CLOUD_SMOKE_CODE_FILE"]
fn live_reverb_device_roundtrip() {
    std::env::set_var("SERVERBOND_CLOUD_ALLOW_HTTP", "1");
    let code = std::fs::read_to_string(std::env::var("SERVERBOND_CLOUD_SMOKE_CODE_FILE").unwrap())
        .unwrap();
    let url = std::env::var("SERVERBOND_CLOUD_SMOKE_URL").unwrap();
    let home = tempfile::tempdir().unwrap();
    let manager = Arc::new(Manager::new(home.path().to_path_buf()).unwrap());
    if std::env::var("SMOKE_LOGS").as_deref() == Ok("1") {
        std::fs::write(
            home.path().join("logs/redis.log"),
            "Cloud günlük testi: <script>throw new Error('unsafe')</script>\nTürkçe kayıt ✓",
        )
        .unwrap();
    }
    if std::env::var("SMOKE_SETTINGS").as_deref() == Ok("1") {
        let listeners: Vec<_> = (0..3)
            .map(|_| std::net::TcpListener::bind("127.0.0.1:0").unwrap())
            .collect();
        let mut settings = manager.snapshot().unwrap().settings;
        settings.web_port = listeners[0].local_addr().unwrap().port();
        settings.mysql_port = listeners[1].local_addr().unwrap().port();
        settings.php_port = listeners[2].local_addr().unwrap().port();
        drop(listeners);
        manager.save_settings(settings).unwrap();
    }
    if std::env::var("SMOKE_POSTGRES").as_deref() == Ok("1") {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut settings = manager.snapshot().unwrap().settings;
        settings.postgres.port = listener.local_addr().unwrap().port();
        drop(listener);
        manager.save_settings(settings).unwrap();
    }
    if std::env::var("SMOKE_DATABASE").as_deref() == Ok("1") {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut settings = manager.snapshot().unwrap().settings;
        settings.mysql_port = listener.local_addr().unwrap().port();
        drop(listener);
        manager.save_settings(settings).unwrap();
    }
    if let Some(cache) = std::env::var_os("SERVERBOND_TEST_CACHE") {
        for entry in std::fs::read_dir(cache).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                std::fs::copy(
                    entry.path(),
                    home.path().join("cache").join(entry.file_name()),
                )
                .unwrap();
            }
        }
    }
    let project_fixture = std::env::var("SERVERBOND_CLOUD_SMOKE_PROJECT_FILE")
        .ok()
        .map(|file| {
            let project = home.path().join("www/remote-sample");
            std::fs::create_dir_all(project.join("public")).unwrap();
            std::fs::write(project.join("public/index.php"), "<?php echo 'cloud';").unwrap();
            std::fs::write(project.join("artisan"), "<?php").unwrap();
            std::fs::write(project.join(".env"), "SENTINEL=preserved").unwrap();
            let source = home.path().join("source-repo");
            std::fs::create_dir_all(source.join("public")).unwrap();
            std::fs::write(source.join("public/index.php"), "<?php echo 'git';").unwrap();
            // Minimal Artisan protocol fixture: real PHP processes, no queue backend.
            std::fs::write(source.join("artisan"), "<?php if (in_array($argv[1] ?? '', ['queue:work', 'schedule:work'], true)) { while (true) { usleep(200000); } } echo 'fixture ok';").unwrap();
            for args in [
                vec!["init", "--initial-branch=main"],
                vec!["add", "."],
                vec![
                    "-c",
                    "user.name=Cloud Test",
                    "-c",
                    "user.email=cloud@example.test",
                    "-c",
                    "commit.gpgSign=false",
                    "-c",
                    "core.hooksPath=NUL",
                    "commit",
                    "-m",
                    "Fixture",
                ],
            ] {
                use std::os::windows::process::CommandExt;
                let result = std::process::Command::new("git")
                    .args(args)
                    .current_dir(&source)
                    .creation_flags(0x08000000)
                    .output()
                    .unwrap();
                assert!(
                    result.status.success(),
                    "Could not prepare local Git fixture"
                );
            }
            let git_url = reqwest::Url::from_directory_path(&source)
                .unwrap()
                .to_string();
            std::fs::write(
                file,
                serde_json::json!({"path":project,"gitUrl":git_url,"source":source}).to_string(),
            )
            .unwrap();
            project
        });
    manager.cloud_pair(&url, code.trim()).unwrap();
    assert!(manager.cloud_status().unwrap().paired);
    let secret = std::fs::read(home.path().join("config/cloud.dpapi")).unwrap();
    assert!(!String::from_utf8_lossy(&secret).contains("device_id"));
    manager.start_cloud();
    let start = Instant::now();
    let mut success = false;
    loop {
        for file in std::fs::read_dir(home.path().join("config"))
            .unwrap()
            .flatten()
        {
            if file
                .file_name()
                .to_string_lossy()
                .starts_with("cloud-commands-")
            {
                let journal: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(file.path()).unwrap()).unwrap();
                success |= journal
                    .as_object()
                    .unwrap()
                    .values()
                    .any(|v| v == "succeeded");
            }
        }
        let status = manager.cloud_status().unwrap();
        if success && !status.paired {
            break;
        }
        assert!(
            start.elapsed()
                < Duration::from_secs(
                    if std::env::var("SMOKE_PHP").as_deref() == Ok("1")
                        || std::env::var("SMOKE_JOBS").as_deref() == Ok("1")
                        || std::env::var("SMOKE_DATABASE").as_deref() == Ok("1")
                        || std::env::var("SMOKE_POSTGRES").as_deref() == Ok("1")
                        || std::env::var("SMOKE_GITHUB_IMPORT").as_deref() == Ok("1")
                    {
                        600
                    } else {
                        150
                    }
                ),
            "Cloud roundtrip timed out: {:?}",
            status.error
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    manager.cloud_disconnect().unwrap();
    if std::env::var("SMOKE_GITHUB_IMPORT").as_deref() == Ok("1") {
        let snapshot = manager.snapshot().unwrap();
        let project = &snapshot
            .projects
            .iter()
            .find(|p| p.project.name == "github-live-preview")
            .expect("Cloud GitHub project must be registered")
            .project;
        assert!(
            project.path.starts_with(home.path()),
            "Clone must stay in isolated test home"
        );
        assert!(project.path.join("public/index.php").is_file());
        assert!(project.path.join("artisan").is_file());
        let output = std::process::Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&project.path)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            project.release.branch
        );
        let output = std::process::Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(&project.path)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "https://github.com/laravel/laravel.git"
        );
    }
    if std::env::var("SMOKE_POSTGRES").as_deref() == Ok("1") {
        manager.stop_postgres().unwrap();
    }
    if std::env::var("SMOKE_DATABASE").as_deref() == Ok("1") {
        assert_eq!(
            manager
                .mysql_query("SELECT value FROM remote_git.cloud_probe WHERE id=1")
                .unwrap(),
            "restored-marker"
        );
        manager.stop("mysql").unwrap();
    }
    if std::env::var("SMOKE_PHP").as_deref() == Ok("1") {
        assert_eq!(
            manager
                .snapshot()
                .unwrap()
                .packages
                .iter()
                .find(|p| p.package.id == "php")
                .unwrap()
                .package
                .version,
            "8.3.33"
        );
        assert!(home.path().join("bin/php/8.3.33/php.exe").exists());
        if project_fixture.is_some() {
            assert!(home.path().join("bin/php/8.4.25/php.exe").exists());
        }
    }
    if let Some(project) = project_fixture {
        assert_eq!(
            std::fs::read_to_string(project.join(".env")).unwrap(),
            "SENTINEL=preserved"
        );
        assert!(project.join("public/index.php").exists());
        assert_eq!(
            std::fs::read_to_string(home.path().join("www/remote-git/public/index.php")).unwrap(),
            "<?php echo 'deployed';"
        );
        assert!(
            manager.snapshot().unwrap().projects.is_empty(),
            "Cloud smoke must remove its project from the list"
        );
    }
    assert!(!home.path().join("config/cloud.dpapi").exists());
}
