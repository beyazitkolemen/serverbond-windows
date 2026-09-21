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
                serde_json::json!({"path":project,"gitUrl":git_url}).to_string(),
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
            start.elapsed() < Duration::from_secs(150),
            "Cloud roundtrip timed out: {:?}",
            status.error
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    manager.cloud_disconnect().unwrap();
    if let Some(project) = project_fixture {
        assert_eq!(
            std::fs::read_to_string(project.join(".env")).unwrap(),
            "SENTINEL=preserved"
        );
        assert!(project.join("public/index.php").exists());
        assert!(home.path().join("www/remote-git/public/index.php").exists());
        assert!(
            manager.snapshot().unwrap().projects.is_empty(),
            "Cloud smoke must remove its project from the list"
        );
    }
    assert!(!home.path().join("config/cloud.dpapi").exists());
}
