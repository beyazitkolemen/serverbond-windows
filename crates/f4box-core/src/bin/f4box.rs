use anyhow::{bail, Context, Result};
use f4box_core::{model::Settings, Manager};
use std::{path::PathBuf, time::Duration};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "help" {
        println!("F4Box CLI\n  status\n  install [all|php|mysql|caddy|composer|phpmyadmin]\n  php [version] (listele veya indir ve kullan)\n  serve\n  add <name> <folder>\n  create <name> <parent>\n  discover\n  import [klasör...]\n  project release <ad>\n  env <ad>\n  queue <name> start|stop|restart|failed|retry|flush|log [worker|iş]\n  schedule <name> start|stop|restart|list|log\n  logs <name> [php|schedule|worker:<id>]\n  db <name> create|backup|restore <sql>\n  db password <parola>\n  https trust|untrust\n  tunnel install|start|stop|token <jeton>|apply <jeton>|forget\n  mail install|start|stop|open\n  postgres install|start|stop|repair|password [parola]\n  node install|repair\n  github token <jeton>|forget|import <depo> [ad] [dal]|status\n  permissions ensure|grant [defender]\n  smoke\n\nVeri dizini: %LOCALAPPDATA%/F4Box (F4BOX_HOME ile değiştirilebilir).\nServisler bu işlem kapandığında durur.");
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
        "discover" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&manager.discover_projects()?)?
            )
        }
        "import" => {
            let paths = if args.len() > 1 {
                args[1..].iter().map(PathBuf::from).collect()
            } else {
                manager
                    .discover_projects()?
                    .into_iter()
                    .map(|project| project.path)
                    .collect()
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&manager.import_projects(paths)?)?
            );
        }
        "db" => {
            if args.get(1).map(String::as_str) == Some("password") {
                manager
                    .change_mysql_password(args.get(2).context("Yeni MySQL parolası gerekli.")?)?;
            } else {
                let name = args.get(1).context("Proje adı gerekli.")?;
                match args.get(2).map(String::as_str).unwrap_or("") {
                    "create" => manager.create_database(name)?,
                    "backup" => println!("{}", manager.backup_database(name)?),
                    "restore" => manager.restore_database(
                        name,
                        PathBuf::from(args.get(3).context("SQL dosyası gerekli.")?),
                    )?,
                    _ => bail!(
                        "Kullanım: db <ad> create|backup|restore <sql>  veya  db password <parola>"
                    ),
                }
            }
        }
        "https" => match args.get(1).map(String::as_str).unwrap_or("") {
            "trust" => manager.trust_https()?,
            "untrust" => manager.untrust_https()?,
            _ => bail!("Kullanım: https trust|untrust"),
        },
        "env" => {
            let name = args.get(1).context("Proje adı gerekli.")?;
            let snapshot = manager.snapshot()?;
            let project = snapshot
                .projects
                .iter()
                .find(|p| p.project.name == *name)
                .context("Proje bulunamadı.")?;
            print!("{}", manager.read_project_env(&project.project.id)?.content);
        }
        "project" => {
            let action = args.get(1).map(String::as_str).unwrap_or("");
            if action != "release" {
                bail!("Kullanım: project release <ad>");
            }
            let name = args.get(2).context("Proje adı gerekli.")?;
            let snapshot = manager.snapshot()?;
            let project = snapshot
                .projects
                .iter()
                .find(|p| p.project.name == *name)
                .context("Proje bulunamadı.")?;
            let record = manager.deploy_project(&project.project.id)?;
            println!("{}", record.output);
            if !record.success {
                bail!("Sürüm başarısız.");
            }
        }
        "queue" | "schedule" => {
            let name = args.get(1).context("Proje adı gerekli.")?;
            let action = args.get(2).map(String::as_str).unwrap_or("status");
            let snapshot = manager.snapshot()?;
            let project = snapshot
                .projects
                .iter()
                .find(|p| p.project.name == *name)
                .context("Proje bulunamadı.")?;
            match (args[0].as_str(), action) {
                ("queue", "start" | "stop" | "restart" | "log") => {
                    let worker = args.get(3).map(String::as_str).unwrap_or("default");
                    let id = project
                        .project
                        .workers
                        .iter()
                        .find(|w| w.name == worker || w.id == worker)
                        .context("Kuyruk işçisi bulunamadı. Önce arayüzden ekleyin.")?
                        .id
                        .clone();
                    match action {
                        "start" => manager.start_project_worker(&project.project.id, &id)?,
                        "stop" => manager.stop_project_worker(&project.project.id, &id)?,
                        "restart" => manager.restart_project_worker(&project.project.id, &id)?,
                        _ => println!(
                            "{}",
                            manager.read_project_worker_log(&project.project.id, &id)?
                        ),
                    }
                }
                ("queue", "failed") => {
                    println!("{}", manager.list_failed_jobs(&project.project.id)?)
                }
                ("queue", "retry") => println!(
                    "{}",
                    manager.retry_failed_jobs(&project.project.id, args.get(3).map(String::as_str))?
                ),
                ("queue", "flush") => {
                    println!("{}", manager.flush_failed_jobs(&project.project.id)?)
                }
                ("schedule", "start") => manager.start_project_schedule(&project.project.id)?,
                ("schedule", "stop") => manager.stop_project_schedule(&project.project.id)?,
                ("schedule", "restart") => manager.restart_project_schedule(&project.project.id)?,
                ("schedule", "list") => {
                    println!("{}", manager.list_project_schedule(&project.project.id)?)
                }
                ("schedule", "log") => {
                    println!("{}", manager.read_project_schedule_log(&project.project.id)?)
                }
                _ => bail!(
                    "Kullanım: queue <ad> start|stop|restart|failed|retry|flush|log [işçi|iş]  veya  schedule <ad> start|stop|restart|list|log"
                ),
            }
        }
        "logs" => {
            let name = args.get(1).context("Proje adı gerekli.")?;
            let snapshot = manager.snapshot()?;
            let project = snapshot
                .projects
                .iter()
                .find(|p| p.project.name == *name)
                .context("Proje bulunamadı.")?;
            let raw = args.get(2).map(String::as_str).unwrap_or("php");
            let source = if raw == "php" || raw == "schedule" || raw.starts_with("worker:") {
                raw.to_string()
            } else {
                let id = project
                    .project
                    .workers
                    .iter()
                    .find(|w| w.name == raw || w.id == raw)
                    .context("Kuyruk işçisi bulunamadı.")?
                    .id
                    .clone();
                format!("worker:{id}")
            };
            println!(
                "{}",
                manager.read_project_log(&project.project.id, &source)?
            );
        }
        "tunnel" => {
            match args.get(1).map(String::as_str).unwrap_or("status") {
                "install" => manager.install_tunnel()?,
                "start" => manager.start_tunnel()?,
                "stop" => manager.stop_tunnel()?,
                "token" => manager.save_tunnel_token(args.get(2).context("Jeton gerekli.")?)?,
                "apply" => manager.apply_tunnel(args.get(2).context("Jeton gerekli.")?)?,
                "forget" => manager.clear_tunnel_token()?,
                "status" => println!(
                    "{}",
                    serde_json::to_string_pretty(&manager.snapshot()?.tunnel)?
                ),
                _ => {
                    bail!("Kullanım: tunnel install|start|stop|token <jeton>|apply <jeton>|forget|status")
                }
            }
        }
        "mail" => match args.get(1).map(String::as_str).unwrap_or("status") {
            "install" => manager.install_mail()?,
            "start" => manager.start_mail()?,
            "stop" => manager.stop_mail()?,
            "open" => manager.open_mail()?,
            "status" => println!(
                "{}",
                serde_json::to_string_pretty(&manager.snapshot()?.mail)?
            ),
            _ => bail!("Kullanım: mail install|start|stop|open|status"),
        },
        "postgres" => match args.get(1).map(String::as_str).unwrap_or("status") {
            "install" => manager.install_postgres()?,
            "repair" => manager.repair_postgres()?,
            "start" => manager.start_postgres()?,
            "stop" => manager.stop_postgres()?,
            "password" => {
                if let Some(password) = args.get(2) {
                    manager.change_postgres_password(password)?;
                } else {
                    println!("{}", manager.postgres_credentials()?);
                }
            }
            "status" => println!(
                "{}",
                serde_json::to_string_pretty(&manager.snapshot()?.postgres)?
            ),
            _ => bail!("Kullanım: postgres install|start|stop|repair|password [parola]|status"),
        },
        "github" => match args.get(1).map(String::as_str).unwrap_or("status") {
            "token" => {
                let token = args.get(2).context("GitHub jetonu gerekli.")?;
                manager.save_github_token(token)?;
            }
            "forget" => manager.clear_github_token()?,
            "import" => {
                let repository = args.get(2).context("GitHub deposu gerekli.")?;
                let name = args.get(3).cloned().unwrap_or_default();
                let branch = args.get(4).cloned().unwrap_or_default();
                manager.import_github_project(repository, name, branch)?;
            }
            "status" => println!(
                "{}",
                serde_json::to_string_pretty(&manager.snapshot()?.github)?
            ),
            _ => bail!("Kullanım: github token <jeton>|forget|import <depo> [ad] [dal]|status"),
        },
        "node" => match args.get(1).map(String::as_str).unwrap_or("status") {
            "install" => manager.install_node()?,
            "repair" => manager.repair_node()?,
            "status" => println!(
                "{}",
                serde_json::to_string_pretty(&manager.snapshot()?.node)?
            ),
            _ => bail!("Kullanım: node install|repair|status"),
        },
        "permissions" => match args.get(1).map(String::as_str).unwrap_or("status") {
            "ensure" => println!(
                "{}",
                serde_json::to_string_pretty(&manager.ensure_permissions()?)?
            ),
            "grant" => println!(
                "{}",
                serde_json::to_string_pretty(
                    &manager.grant_permissions(args.get(2).is_some_and(|a| a == "defender"))?
                )?
            ),
            "status" => println!(
                "{}",
                serde_json::to_string_pretty(&manager.permission_state())?
            ),
            _ => bail!("Kullanım: permissions ensure|grant [defender]|status"),
        },
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
