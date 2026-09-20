use anyhow::{bail, Context, Result};
use f4box_core::{model::Settings, Manager};
use std::{path::PathBuf, time::Duration};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "help" {
        println!("F4Box CLI\n  status\n  install [all|php|mysql|caddy|composer|phpmyadmin]\n  php [version] (listele veya indir ve kullan)\n  serve\n  add <name> <folder>\n  create <name> <parent>\n  smoke\n\nVeri dizini: %LOCALAPPDATA%/F4Box (F4BOX_HOME ile değiştirilebilir).\nServisler bu işlem kapandığında durur.");
        return Ok(());
    }
    let manager = Manager::new(Manager::default_home())?;
    match args[0].as_str() {
        "status" => println!("{}", serde_json::to_string_pretty(&manager.snapshot()?)?),
        "install" => manager.install(args.get(1).map(String::as_str).unwrap_or("all"))?,
        "php" => {
            if let Some(version) = args.get(1) {
                manager.select_php(version)?;
                println!("PHP {version} seçildi.");
            } else {
                for p in manager.snapshot()?.php_versions {
                    println!(
                        "{}{}",
                        p.package.version,
                        if p.installed { " (kurulu)" } else { "" }
                    );
                }
            }
        }
        "serve" => {
            manager.start("all")?;
            println!("Ortam çalışıyor. Durdurmak için Enter'a basın.");
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            manager.stop("all")?;
        }
        "add" | "create" => {
            let name = args.get(1).context("Proje adı gerekli.")?.clone();
            let path = PathBuf::from(args.get(2).context("Klasör yolu gerekli.")?);
            let project = if args[0] == "add" {
                manager.add_project(name, path)?
            } else {
                manager.create_project(name, path)?
            };
            println!("{}", serde_json::to_string_pretty(&project)?);
        }
        "smoke" => smoke(&manager)?,
        _ => bail!("Bilinmeyen komut. f4box help"),
    }
    Ok(())
}

fn smoke(manager: &Manager) -> Result<()> {
    manager.install("all")?;
    let original = manager.snapshot()?.settings;
    let free_port = || -> Result<u16> {
        Ok(std::net::TcpListener::bind("127.0.0.1:0")?
            .local_addr()?
            .port())
    };
    let settings = loop {
        let settings = Settings {
            web_port: free_port()?,
            php_port: free_port()?,
            mysql_port: free_port()?,
            ..Settings::default()
        };
        if settings.validate().is_ok() {
            break settings;
        }
    };
    manager.save_settings(settings.clone())?;
    let dir = tempfile::tempdir_in(manager.home.join("www"))?;
    std::fs::create_dir(dir.path().join("public"))?;
    std::fs::write(dir.path().join("public/index.php"), "<?php header('Content-Type: application/json'); echo json_encode(['app'=>'F4Box', 'php'=>PHP_VERSION, 'pdo'=>extension_loaded('pdo_mysql'), 'uri'=>$_SERVER['REQUEST_URI']]);")?;
    let name = format!("smoke-{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    let project = manager.add_project(name.clone(), dir.path().into())?;
    let result = (|| -> Result<()> {
        manager.start("all")?;
        let version = manager.mysql_query("SELECT VERSION()")?;
        manager.create_database(&name)?;
        let database = name.replace('-', "_");
        manager.mysql_query(&format!("CREATE TABLE `{database}`.checks (id INT PRIMARY KEY, value VARCHAR(30)); INSERT INTO `{database}`.checks VALUES (1, 'f4box-ok')"))?;
        let value =
            manager.mysql_query(&format!("SELECT value FROM `{database}`.checks WHERE id=1"))?;
        if value != "f4box-ok" {
            bail!("MySQL veri doğrulaması başarısız.");
        }
        let backup = manager.backup_database(&name)?;
        let sql = std::fs::read_to_string(&backup)?;
        if !sql.contains("f4box-ok") {
            bail!("Veritabanı yedeğinde veri bulunamadı.");
        }
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(15))
            .resolve(&project.host, ([127, 0, 0, 1], settings.web_port).into())
            .build()?;
        let response: serde_json::Value = client
            .get(format!(
                "http://{}:{}/nested/route?check=1",
                project.host, settings.web_port
            ))
            .send()?
            .error_for_status()?
            .json()?;
        if response["app"] != "F4Box"
            || response["pdo"] != true
            || response["uri"] != "/nested/route?check=1"
        {
            bail!("PHP FastCGI ve yönlendirme testi başarısız: {response}");
        }
        let hidden = client
            .get(format!(
                "http://{}:{}/.env",
                project.host, settings.web_port
            ))
            .send()?;
        if hidden.status() != 404 {
            bail!("Gizli dosya koruması başarısız.");
        }
        manager.mysql_query(&format!("DROP DATABASE `{database}`"))?;
        std::fs::remove_file(backup)?;
        println!("PASS: MySQL {version}, SQL write/read, backup, PHP {} + PDO MySQL, Caddy FastCGI routing, hidden-file protection", response["php"]);
        manager.stop("all")?;
        manager.start("all")?;
        if manager.mysql_query("SELECT 1")? != "1" {
            bail!("Yeniden başlatma başarısız.");
        }
        println!("PASS: graceful stop and restart with persisted database and credentials");
        Ok(())
    })();
    let stop = manager.stop("all");
    let remove = manager.remove_project(&project.id);
    let restore = manager.save_settings(original);
    result?;
    stop?;
    remove?;
    restore?;
    Ok(())
}
