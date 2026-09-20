#![cfg(windows)]

use anyhow::{Context, Result};
use f4box_core::{model::php_versions, Manager};
use std::{fs, time::Duration};

// Real downloads and processes are opt-in; ordinary unit tests stay offline.
#[test]
#[ignore = "downloads all PHP builds, Caddy and Composer; starts loopback services"]
fn downloaded_php_builds_switch_live_and_rollback_on_start_failure() -> Result<()> {
    let home = tempfile::Builder::new()
        .prefix("F4Box Türkçe O'Brien test ")
        .tempdir()?;
    let manager = Manager::new(home.path().into())?;
    let mut settings = manager.snapshot()?.settings;
    let listeners = (0..3)
        .map(|_| std::net::TcpListener::bind("127.0.0.1:0"))
        .collect::<std::io::Result<Vec<_>>>()?;
    settings.web_port = listeners[0].local_addr()?.port();
    settings.mysql_port = listeners[1].local_addr()?.port();
    settings.php_port = listeners[2].local_addr()?.port();
    drop(listeners);
    manager.save_settings(settings)?;
    if let Some(cache) = std::env::var_os("F4BOX_TEST_CACHE") {
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
    manager.install("caddy")?;
    manager.install("composer")?;
    manager.select_php("7.4.33")?;
    let project_path = manager.home.join("www/matrix");
    fs::create_dir_all(project_path.join("public"))?;
    fs::write(project_path.join("public/index.php"), "<?php header('Content-Type: application/json'); echo json_encode(['php'=>PHP_VERSION, 'pdo'=>extension_loaded('pdo_mysql'), 'zip'=>extension_loaded('zip')]);")?;
    let project = manager.add_project("matrix".into(), project_path)?;
    let port = manager.snapshot()?.settings.web_port;
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()?;
    let request = || -> Result<serde_json::Value> {
        Ok(client
            .get(format!("http://127.0.0.1:{port}/check"))
            .header("Host", format!("matrix.localhost:{port}"))
            .send()?
            .error_for_status()?
            .json()?)
    };
    let mut anchor_id = String::new();
    let mut anchor_pid = None;
    for (index, package) in php_versions().iter().enumerate() {
        println!(
            "Checking PHP {} download and live switch...",
            package.version
        );
        manager.select_php(&package.version)?;
        manager.select_project_php(&project.id, &package.version)?;
        if index == 0 {
            assert_eq!(
                fs::read_dir(manager.home.join("bin/php"))?.count(),
                1,
                "only the selected build is installed"
            );
            manager.start("caddy")?;
            let anchor_path = manager.home.join("www/anchor");
            fs::create_dir_all(anchor_path.join("public"))?;
            fs::write(
                anchor_path.join("public/index.php"),
                "<?php echo PHP_VERSION;",
            )?;
            let anchor = manager.add_project("anchor".into(), anchor_path)?;
            manager.select_project_php(&anchor.id, "8.4.25")?;
            anchor_id = anchor.id;
            anchor_pid = manager
                .snapshot()?
                .projects
                .iter()
                .find(|p| p.project.id == anchor_id)
                .unwrap()
                .pid;
        }
        assert_eq!(
            manager
                .snapshot()?
                .projects
                .iter()
                .find(|p| p.project.id == anchor_id)
                .unwrap()
                .pid,
            anchor_pid,
            "unrelated PHP worker must not restart"
        );
        assert_eq!(
            client
                .get(format!("http://127.0.0.1:{port}/"))
                .header("Host", format!("anchor.localhost:{port}"))
                .send()?
                .error_for_status()?
                .text()?,
            "8.4.25"
        );
        let response = request()?;
        assert_eq!(response["php"], package.version);
        assert_eq!(response["pdo"], true);
        assert_eq!(response["zip"], true);
        let output = manager
            .php_command()?
            .arg(manager.executable("composer")?)
            .args(["--version", "--no-ansi"])
            .env("PHP_INI_SCAN_DIR", "")
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("Composer version"));
        check_terminal(&manager, &project.id, &package.version)?;
        println!(
            "PASS: PHP {}, FastCGI, PDO, ZIP and Composer",
            package.version
        );
    }
    // Temporarily hide a binary in this disposable test installation to force a restart failure.
    let cgi = manager.home.join("bin/php/7.4.33/php-cgi.exe");
    let hidden = cgi.with_extension("test-disabled");
    fs::rename(&cgi, &hidden)?;
    // CLI can list modules but cannot listen on a FastCGI port (-b).
    fs::copy(cgi.with_file_name("php.exe"), &cgi)?;
    let failed = manager.select_project_php(&project.id, "7.4.33");
    fs::remove_file(&cgi)?;
    fs::rename(&hidden, &cgi)?;
    assert!(failed.is_err());
    assert!(format!("{:#}", failed.unwrap_err()).contains("geri alındı"));
    assert_eq!(request()?["php"], "8.5.10");
    // An unexpected owned process exit is visible and can be recovered explicitly.
    let pid = manager
        .snapshot()?
        .projects
        .iter()
        .find(|p| p.project.id == project.id)
        .unwrap()
        .pid
        .unwrap();
    unsafe {
        use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::*};
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        assert!(!handle.is_null());
        assert_ne!(TerminateProcess(handle, 17), 0);
        CloseHandle(handle);
    }
    for _ in 0..50 {
        if manager
            .snapshot()?
            .projects
            .iter()
            .find(|p| p.project.id == project.id)
            .unwrap()
            .issue
            .is_some()
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(manager
        .snapshot()?
        .projects
        .into_iter()
        .find(|p| p.project.id == project.id)
        .unwrap()
        .issue
        .as_ref()
        .unwrap()
        .contains("beklenmedik"));
    manager.start("php")?;
    assert!(manager
        .snapshot()?
        .projects
        .iter()
        .find(|p| p.project.id == project.id)
        .unwrap()
        .issue
        .is_none());
    assert_eq!(request()?["php"], "8.5.10");
    assert!(
        manager.repair("php").is_err(),
        "cannot replace a running installation"
    );
    manager.stop("all")?;
    fs::remove_file(&cgi)?;
    assert!(!manager.snapshot()?.php_versions[0].installed);
    manager.repair_php("7.4.33")?;
    manager.select_project_php(&project.id, "7.4.33")?;
    manager.start("caddy")?;
    assert_eq!(request()?["php"], "7.4.33");
    manager.select_php("7.4.33")?;
    assert_eq!(request()?["php"], "7.4.33");
    manager.stop("all")?;
    let path = manager.home.clone();
    drop(manager);
    let reopened = Manager::new(path)?;
    assert_eq!(reopened.snapshot()?.packages[0].package.version, "7.4.33");
    reopened
        .start("caddy")
        .context("persisted selection restart")?;
    assert_eq!(request()?["php"], "7.4.33");
    reopened.stop("all")?;
    println!("PASS: rollback, crash reporting/recovery, repair from verified cache, Unicode paths and persisted selection");
    Ok(())
}

fn check_terminal(manager: &Manager, id: &str, version: &str) -> Result<()> {
    use base64::Engine;
    let project = manager
        .snapshot()?
        .projects
        .into_iter()
        .find(|p| p.project.id == id)
        .unwrap()
        .project;
    fs::write(
        project.path.join("composer.json"),
        r#"{"scripts":{"runtime":"@php -r \"echo PHP_VERSION;\""}}"#,
    )?;
    fs::write(project.path.join("terminal-check.php"), "<?php echo json_encode([PHP_VERSION, getcwd(), extension_loaded('pdo_mysql'), extension_loaded('zip')]).PHP_EOL;")?;
    let script = manager.project_terminal_script(id)? + "\nphp './terminal-check.php'\nif ($LASTEXITCODE -ne 0) { exit 2 }\ncomposer run-script runtime --no-ansi\nexit $LASTEXITCODE\n";
    let encoded = base64::engine::general_purpose::STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let mut cmd = std::process::Command::new("powershell.exe");
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    let output = cmd
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encoded,
        ])
        .output()?;
    assert!(
        output.status.success(),
        "terminal: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.matches(version).count() >= 2,
        "PHP and Composer child must use {version}: {stdout}"
    );
    let json_line = stdout
        .lines()
        .find(|line| line.starts_with('['))
        .with_context(|| format!("Missing terminal JSON: {stdout}"))?;
    let end = json_line.find(']').context("Missing JSON array end")? + 1;
    let data: serde_json::Value = serde_json::from_str(&json_line[..end])?;
    assert_eq!(data[0], version);
    assert_eq!(
        std::path::PathBuf::from(data[1].as_str().unwrap()),
        project.path
    );
    assert_eq!(data[2], true);
    assert_eq!(data[3], true);
    Ok(())
}
