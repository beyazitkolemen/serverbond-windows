//! Explicit network smoke test, isolated from the user's installation.
#[test]
#[ignore = "downloads the pinned cloudflared executable into a temporary directory"]
fn launch_downloads_verified_cloudflared_once_without_starting_a_tunnel() {
    let home = tempfile::tempdir().unwrap();
    let manager = serverbond_core::Manager::new(home.path().into()).unwrap();
    manager.prepare_launch_tools().unwrap();
    let installed = manager.snapshot().unwrap().tunnel;
    assert!(installed.installed);
    assert!(!installed.running);
    assert!(!installed.token_saved);
    let package = serverbond_core::model::tool_package("cloudflared").unwrap();
    let executable = manager.tool_executable("cloudflared").unwrap();
    serverbond_core::install::verify_hash(&executable, &package.sha256).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new(&executable)
            .arg("--version")
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains(&package.version));
    }
    let before = std::fs::metadata(&executable).unwrap().modified().unwrap();
    manager.prepare_launch_tools().unwrap();
    assert_eq!(
        std::fs::metadata(executable).unwrap().modified().unwrap(),
        before
    );
    assert!(!manager.snapshot().unwrap().any_running);
}
