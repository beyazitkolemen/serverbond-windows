#![cfg(windows)]
mod support;

use anyhow::{ensure, Context, Result};
use serverbond_core::{model::tool_package, Manager};
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    os::windows::process::CommandExt,
    process::Command,
    time::Duration,
};

fn configure_ports(manager: &Manager) -> Result<()> {
    let ports = (0..9)
        .map(|_| TcpListener::bind("127.0.0.1:0"))
        .collect::<std::io::Result<Vec<_>>>()?;
    let mut settings = manager.snapshot()?.settings;
    settings.web_port = ports[0].local_addr()?.port();
    settings.php_port = ports[1].local_addr()?.port();
    settings.mysql_port = ports[2].local_addr()?.port();
    settings.web.https_port = ports[3].local_addr()?.port();
    settings.mail.smtp_port = ports[4].local_addr()?.port();
    settings.mail.web_port = ports[5].local_addr()?.port();
    settings.redis.port = ports[6].local_addr()?.port();
    settings.postgres.port = ports[7].local_addr()?.port();
    settings.api.port = ports[8].local_addr()?.port();
    drop(ports);
    manager.save_settings(settings)
}

fn redis(manager: &Manager, args: &[&str]) -> Result<String> {
    let package = tool_package("redis")?;
    let directory = manager.home.join("bin/redis").join(package.version);
    let output = Command::new(directory.join("redis-cli.exe"))
        .current_dir(&directory)
        .args([
            "-h",
            "127.0.0.1",
            "-p",
            &manager.snapshot()?.settings.redis.port.to_string(),
            "--raw",
        ])
        .args(args)
        .creation_flags(0x08000000)
        .output()?;
    ensure!(
        output.status.success(),
        "Redis CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

#[test]
#[ignore = "downloads real Redis and checks durable data across stop, repair and reopen"]
fn redis_shutdown_preserves_recent_queue_data() -> Result<()> {
    let home = tempfile::Builder::new()
        .prefix("ServerBond Redis Türkçe ")
        .tempdir()?;
    let manager = Manager::new(home.path().into())?;
    configure_ports(&manager)?;
    support::restore_package_cache(home.path())?;
    manager.install_redis()?;
    support::save_package_cache(home.path())?;
    manager.start_redis()?;
    ensure!(
        redis(&manager, &["PING"])? == "PONG",
        "Redis did not answer"
    );
    ensure!(
        redis(&manager, &["SET", "queue:sentinel", "preserved"])? == "OK",
        "SET failed"
    );
    // Simulate a failed RDB write using only this disposable data directory.
    let blocked_dump = home.path().join("data/redis/dump.rdb");
    std::fs::create_dir(&blocked_dump)?;
    ensure!(
        manager.stop_redis().is_err(),
        "Stop hid the failed Redis save"
    );
    ensure!(
        manager.snapshot()?.redis.running,
        "Failed save killed Redis"
    );
    ensure!(
        redis(&manager, &["GET", "queue:sentinel"])? == "preserved",
        "Failed save lost live Redis data"
    );
    std::fs::remove_dir(blocked_dump)?;
    // No explicit SAVE: normal application stop must flush recent queue/session data.
    manager.stop_redis()?;
    manager.start_redis()?;
    ensure!(
        redis(&manager, &["GET", "queue:sentinel"])? == "preserved",
        "Recent Redis data lost during normal stop"
    );
    manager.repair_redis()?;
    manager.start_redis()?;
    ensure!(
        redis(&manager, &["GET", "queue:sentinel"])? == "preserved",
        "Redis repair lost data"
    );
    manager.stop_redis()?;
    drop(manager);
    let manager = Manager::new(home.path().into())?;
    manager.start_redis()?;
    ensure!(
        redis(&manager, &["GET", "queue:sentinel"])? == "preserved",
        "Reopened Redis lost data"
    );
    manager.stop_redis()?;
    Ok(())
}

fn smtp_reply(reader: &mut BufReader<TcpStream>, expected: &str) -> Result<()> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    ensure!(
        line.starts_with(expected),
        "Unexpected SMTP response: {line}"
    );
    Ok(())
}

fn mail_messages(port: u16) -> Result<serde_json::Value> {
    reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()?
        .get(format!("http://127.0.0.1:{port}/api/v1/messages"))
        .send()?
        .error_for_status()?
        .json()
        .context("Mailpit message listing failed")
}

#[test]
#[ignore = "downloads real Mailpit, sends local SMTP and verifies message persistence"]
fn mailpit_smtp_messages_survive_repair_and_reopen() -> Result<()> {
    let home = tempfile::Builder::new()
        .prefix("ServerBond Mail Türkçe ")
        .tempdir()?;
    let manager = Manager::new(home.path().into())?;
    configure_ports(&manager)?;
    support::restore_package_cache(home.path())?;
    manager.install_mail()?;
    support::save_package_cache(home.path())?;
    manager.start_mail()?;
    let settings = manager.snapshot()?.settings.mail;
    let mut socket = TcpStream::connect(("127.0.0.1", settings.smtp_port))?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    socket.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(socket.try_clone()?);
    smtp_reply(&mut reader, "220")?;
    for (command, response) in [
        ("HELO localhost\r\n", "250"),
        ("MAIL FROM:<qa@example.invalid>\r\n", "250"),
        ("RCPT TO:<test@example.invalid>\r\n", "250"),
        ("DATA\r\n", "354"),
        ("From: qa@example.invalid\r\nTo: test@example.invalid\r\nSubject: ServerBond persistence\r\n\r\nCaptured locally only.\r\n.\r\n", "250"),
        ("QUIT\r\n", "221"),
    ] {
        socket.write_all(command.as_bytes())?;
        smtp_reply(&mut reader, response)?;
    }
    let messages = mail_messages(settings.web_port)?;
    ensure!(
        messages["total"] == 1,
        "SMTP message not captured: {messages}"
    );
    let id = messages["messages"][0]["ID"].clone();
    ensure!(
        messages["messages"][0]["Subject"] == "ServerBond persistence",
        "Wrong SMTP subject"
    );
    manager.repair_mail()?;
    manager.start_mail()?;
    ensure!(
        mail_messages(settings.web_port)?["messages"][0]["ID"] == id,
        "Mailpit repair lost message"
    );
    manager.stop_mail()?;
    drop(manager);
    let manager = Manager::new(home.path().into())?;
    manager.start_mail()?;
    ensure!(
        mail_messages(settings.web_port)?["messages"][0]["ID"] == id,
        "Reopened Mailpit lost message"
    );
    manager.stop_mail()?;
    Ok(())
}
