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
    assert!(!home.path().join("config/cloud.dpapi").exists());
}
