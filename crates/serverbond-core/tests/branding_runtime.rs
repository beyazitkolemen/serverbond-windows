#![cfg(windows)]

use anyhow::Result;
use serverbond_core::Manager;
use std::{
    fs, net::TcpListener, os::windows::process::CommandExt, process::Command, time::Duration,
};

#[test]
#[ignore = "installs PHP, Composer and Caddy and runs a disposable web project"]
fn serverbond_environment_reaches_cli_web_and_project_terminal() -> Result<()> {
    let home = tempfile::Builder::new()
        .prefix("ServerBond Türkçe ")
        .tempdir()?;
    let manager = Manager::new(home.path().into())?;
    let mut settings = manager.snapshot()?.settings;
    let listeners = (0..4)
        .map(|_| TcpListener::bind("127.0.0.1:0"))
        .collect::<std::io::Result<Vec<_>>>()?;
    settings.web_port = listeners[0].local_addr()?.port();
    settings.mysql_port = listeners[1].local_addr()?.port();
    settings.php_port = listeners[2].local_addr()?.port();
    settings.web.https_port = listeners[3].local_addr()?.port();
    // This checks names and extension loading, independently of Windows'
    // shared OPcache mapping used by other PHP processes on the machine.
    settings.php.opcache = false;
    let web_port = settings.web_port;
    drop(listeners);
    manager.save_settings(settings)?;
    if let Some(cache) = std::env::var_os("SERVERBOND_TEST_CACHE") {
        for entry in fs::read_dir(cache)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::copy(
                    entry.path(),
                    home.path().join("cache").join(entry.file_name()),
                )?;
            }
        }
    }
    for package in ["php", "composer", "caddy"] {
        manager.install(package)?;
    }
    let cli = manager
        .php_command()?
        .args(["-r", "echo getenv('SERVERBOND_PHP_EXT');"])
        .output()?;
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let extensions = manager.package_dir("php")?.join("ext");
    assert_eq!(String::from_utf8(cli.stdout)?, extensions.to_string_lossy());
    assert!(fs::read_to_string(manager.php_ini_path()?)?.contains("${SERVERBOND_PHP_EXT}"));

    let path = home.path().join("projects/branding");
    fs::create_dir_all(path.join("public"))?;
    fs::write(path.join(".env"), "APP_KEY=preserve-branding-test")?;
    fs::write(path.join("public/index.php"), "<?php header('Content-Type: application/json'); echo json_encode(['env_set'=>getenv('SERVERBOND_PHP_EXT')!==false, 'ext_exists'=>is_dir(getenv('SERVERBOND_PHP_EXT')), 'pdo'=>extension_loaded('pdo_mysql')]);")?;
    let project = manager.add_project("branding".into(), path.clone())?;
    let mut script = manager.project_terminal_script(&project.id)?;
    script.push_str(r#"
php -r "echo json_encode(['env_set'=>getenv('SERVERBOND_PHP_EXT')!==false, 'ext_exists'=>is_dir(getenv('SERVERBOND_PHP_EXT')), 'pdo'=>extension_loaded('pdo_mysql')]), PHP_EOL;"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
composer --version --no-ansi
exit $LASTEXITCODE
"#);
    let powershell = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let terminal = Command::new(powershell)
        .creation_flags(0x08000000)
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()?;
    assert!(
        terminal.status.success(),
        "status: {}; stdout: {}; stderr: {}",
        terminal.status,
        String::from_utf8_lossy(&terminal.stdout),
        String::from_utf8_lossy(&terminal.stderr)
    );
    let terminal_output = String::from_utf8_lossy(&terminal.stdout);
    let terminal_php: serde_json::Value = serde_json::from_str(
        terminal_output
            .lines()
            .find(|line| line.starts_with('{'))
            .expect("PHP JSON output"),
    )?;
    assert_eq!(terminal_php["env_set"], true);
    assert_eq!(terminal_php["ext_exists"], true);
    assert_eq!(terminal_php["pdo"], true);
    assert!(terminal_output.contains("Composer version"));
    manager.start("php")?;
    manager.start("caddy")?;
    let response: serde_json::Value = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()?
        .get(format!("http://127.0.0.1:{web_port}/"))
        .header("Host", "branding.localhost")
        .send()?
        .error_for_status()?
        .json()?;
    assert_eq!(response["env_set"], true);
    assert_eq!(response["ext_exists"], true);
    assert_eq!(response["pdo"], true);
    manager.stop("all")?;
    assert_eq!(
        fs::read_to_string(path.join(".env"))?,
        "APP_KEY=preserve-branding-test"
    );
    Ok(())
}
