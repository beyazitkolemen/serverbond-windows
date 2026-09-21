//! Core service lifecycle: PHP FastCGI, MySQL and Caddy start/stop/restart,
//! port checks, generated config files, readiness probes and the MySQL
//! database helpers (create, backup, restore, root password).

use crate::{
    model::caddy_config,
    portable_path,
    process::{command, ManagedChild},
    secrets, Manager,
};
use anyhow::{bail, Context, Result};
use std::{
    fs,
    io::Write,
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

fn log_contains(path: &std::path::Path, offset: u64, marker: &str) -> Result<bool> {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = fs::File::open(path) else {
        return Ok(false);
    };
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::new();
    file.take(256 * 1024).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).contains(marker))
}

pub fn port_free(port: u16) -> Result<()> {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).with_context(|| format!("{port} portu başka bir uygulama tarafından kullanılıyor. Ayarlar ekranından farklı port seçin."))?;
    Ok(())
}

impl Manager {
    pub fn php_command(&self) -> Result<Command> {
        let mut cmd = command(self.executable("php")?);
        cmd.arg("-c")
            .arg(self.php_ini_path()?)
            .arg("-d")
            .arg(format!(
                "extension_dir=\"{}\"",
                portable_path(&self.package_dir("php")?.join("ext"))
            ))
            .env("PHPRC", self.php_ini_path()?)
            .env("SERVERBOND_PHP_EXT", self.package_dir("php")?.join("ext"))
            .env("PHP_INI_SCAN_DIR", "");
        Ok(cmd)
    }

    pub fn php_ini_path(&self) -> Result<std::path::PathBuf> {
        Ok(self
            .home
            .join("config/php")
            .join(self.package("php")?.version)
            .join("php.ini"))
    }

    pub(crate) fn write_php_config(&self) -> Result<()> {
        self.write_php_config_for(&self.package("php")?.version)?;
        Ok(())
    }

    pub(crate) fn write_php_config_for(&self, version: &str) -> Result<std::path::PathBuf> {
        crate::model::php_package(version)?;
        let extensions = self.home.join("bin/php").join(version).join("ext");
        let (profile, mail) = {
            let config = self.config.lock().unwrap_or_else(|e| e.into_inner());
            (
                config.settings.php_for(version),
                config.settings.mail.clone(),
            )
        };
        self.validate_php_profile(version, &profile)?;
        let config = profile.render(&extensions, Some(&mail))?;
        let path = self.home.join("config/php").join(version).join("php.ini");
        fs::create_dir_all(path.parent().unwrap())?;
        crate::storage::atomic_write(&path, config)?;
        Ok(path)
    }

    pub fn select_php(&self, version: &str) -> Result<()> {
        let _guard = self.gate()?;
        self.select_php_inner(version)
    }

    pub fn repair_php(&self, version: &str) -> Result<()> {
        let _guard = self.gate()?;
        let package = crate::model::php_package(version)?;
        if self.snapshot()?.any_running {
            bail!("PHP onarımı için önce çalışan ortamı durdurun.");
        }
        crate::install::repair(&self.home, &package, |line| self.log(line))?;
        self.select_php_inner(version)
    }

    pub(crate) fn prepare_php(&self, version: &str) -> Result<()> {
        self.check_install_requirements()?;
        let package = crate::model::php_package(version)?;
        // Download and verify before interrupting the running environment.
        crate::install::install(&self.home, &package, |line| self.log(line))?;
        let ini = self.write_php_config_for(version)?;
        let executable = self.home.join("bin/php").join(version).join("php.exe");
        let profile = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .php_for(version);
        let mut cgi_check = command(executable.with_file_name("php-cgi.exe"));
        cgi_check
            .arg("-c")
            .arg(&ini)
            .arg("-d")
            .arg(format!(
                "extension_dir=\"{}\"",
                portable_path(&executable.parent().unwrap().join("ext"))
            ))
            .arg("-m")
            .env(
                "SERVERBOND_PHP_EXT",
                executable.parent().unwrap().join("ext"),
            )
            .env("PHP_INI_SCAN_DIR", "");
        let cgi_output = ManagedChild::output(cgi_check, Duration::from_secs(20))?;
        if !cgi_output.status.success() || !cgi_output.stderr.is_empty() {
            bail!(
                "PHP FastCGI doğrulanamadı: {} {}",
                cgi_output.status,
                String::from_utf8_lossy(&cgi_output.stderr)
            );
        }
        let modules = String::from_utf8_lossy(&cgi_output.stdout);
        for extension in &profile.extensions {
            if !modules
                .lines()
                .any(|line| line.trim() == extension.as_str())
            {
                bail!("PHP FastCGI uzantısı yüklenemedi: {extension}");
            }
        }
        Ok(())
    }

    fn select_php_inner(&self, version: &str) -> Result<()> {
        self.prepare_php(version)?;
        let original = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if original.php_version == version {
            self.log(format!("PHP {version} kullanıma hazır."));
            return Ok(());
        }
        self.snapshot()?;
        let php_running = self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key("php");
        self.stop_service("php")?;
        let restart = || -> Result<()> {
            if php_running {
                self.start_default_php()?;
            }
            Ok(())
        };
        let mut selected = original.clone();
        selected.php_version = version.into();
        let result = self.save_config(&selected).and_then(|_| restart());
        if let Err(error) = result {
            self.stop_service("php")?;
            self.save_config(&original)
                .context("Önceki PHP seçimi geri yüklenemedi.")?;
            restart().with_context(|| format!("PHP {version} geçişi başarısız ({error:#}); önceki sürüm yeniden başlatılamadı"))?;
            return Err(error.context("PHP geçişi başarısız; önceki sürüme dönüldü."));
        }
        self.log(format!(
            "Varsayılan PHP {version} seçildi. Yeni projeler bu sürümü kullanır."
        ));
        Ok(())
    }

    pub(crate) fn spawn_service(&self, id: &str, cmd: Command, port: u16) -> Result<()> {
        if self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(id)
            .is_some_and(|p| p.alive())
        {
            return Ok(());
        }
        port_free(port)?;
        let mut child =
            ManagedChild::spawn(cmd, &self.home.join("logs").join(format!("{id}.log")))?;
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(status) = child.child.try_wait()? {
                bail!("{id} başlatılamadı ({status}). Günlükler ekranındaki {id} kaydını kontrol edin.");
            }
            if TcpStream::connect_timeout(
                &SocketAddrV4::new(Ipv4Addr::LOCALHOST, port).into(),
                Duration::from_millis(150),
            )
            .is_ok()
            {
                break;
            }
            if Instant::now() > deadline {
                bail!("{id} başlatma zaman aşımı. Günlükleri kontrol edin.");
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        self.processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.into(), child);
        self.service_errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        self.log(format!("{id} çalışıyor · 127.0.0.1:{port}"));
        Ok(())
    }

    /// Starts a child that never opens a listening port: readiness is the line the
    /// program prints into its own log.
    pub(crate) fn spawn_watched(
        &self,
        id: &str,
        cmd: Command,
        marker: &str,
        timeout: Duration,
    ) -> Result<()> {
        if self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(id)
            .is_some_and(|p| p.alive())
        {
            return Ok(());
        }
        let log = self.home.join("logs").join(format!("{id}.log"));
        let offset = log.metadata().map(|m| m.len()).unwrap_or(0);
        let mut child = ManagedChild::spawn(cmd, &log)?;
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = child.child.try_wait()? {
                bail!("{id} başlatılamadı ({status}). Günlükler ekranındaki {id} kaydını kontrol edin.");
            }
            if log_contains(&log, offset, marker)? {
                break;
            }
            if Instant::now() > deadline {
                bail!("{id} hazır olduğunu bildirmedi. Günlükler ekranındaki {id} kaydını kontrol edin.");
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        self.processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.into(), child);
        self.service_errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        Ok(())
    }

    pub fn restart(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.stop_inner()?;
        self.start_inner("all")
    }

    pub fn start(&self, id: &str) -> Result<()> {
        let _guard = self.gate()?;
        self.start_inner(id)
    }

    fn start_inner(&self, id: &str) -> Result<()> {
        // Remove and report exited children before taking the rollback baseline.
        self.snapshot()?;
        match id {
            "all" => {
                for package in ["php", "mysql", "caddy"] {
                    self.executable(package)?;
                }
                let before: Vec<String> = self
                    .processes
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .keys()
                    .cloned()
                    .collect();
                let outcome = self
                    .start_mysql()
                    .and_then(|_| self.start_php())
                    .and_then(|_| self.start_caddy());
                if outcome.is_err() {
                    self.rollback_new_services(&before);
                } else {
                    self.start_mail_autostart();
                    self.start_postgres_autostart();
                    self.start_redis_autostart();
                    self.start_tunnel_autostart();
                }
                outcome
            }
            "mysql" => self.start_mysql(),
            crate::tunnel::ID => self.start_tunnel_inner(),
            crate::mail::ID => self.start_mail_inner(),
            crate::postgres::ID => self.start_postgres_inner(),
            crate::redis::ID => self.start_redis_inner(),
            "php" => self.start_php(),
            "caddy" => {
                let before = self
                    .processes
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>();
                let outcome = self.start_php().and_then(|_| self.start_caddy());
                if outcome.is_err() {
                    self.rollback_new_services(&before);
                }
                outcome
            }
            _ => bail!("Bu bileşen servis olarak çalıştırılmaz."),
        }
    }

    fn start_php(&self) -> Result<()> {
        let before = self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let result = self
            .start_default_php()
            .and_then(|_| self.start_project_workers());
        if result.is_err() {
            self.rollback_new_services(&before);
            return result;
        }
        self.start_autostart_jobs();
        Ok(())
    }

    fn start_default_php(&self) -> Result<()> {
        self.write_php_config()?;
        let settings = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .clone();
        let mut cmd = command(self.executable("php")?.with_file_name("php-cgi.exe"));
        cmd.arg("-c")
            .arg(self.php_ini_path()?)
            .arg("-d")
            .arg(format!(
                "extension_dir=\"{}\"",
                portable_path(&self.package_dir("php")?.join("ext"))
            ))
            .arg("-b")
            .arg(format!("127.0.0.1:{}", settings.php_port));
        cmd.env("PHP_FCGI_MAX_REQUESTS", "0")
            .env(
                crate::product::PMA_TMP_ENV,
                self.home.join("data/phpmyadmin/tmp"),
            )
            .env(
                crate::product::PMA_SESSIONS_ENV,
                self.home.join("data/phpmyadmin/sessions"),
            )
            .env("SERVERBOND_PHP_EXT", self.package_dir("php")?.join("ext"))
            .env("PHP_INI_SCAN_DIR", "");
        self.spawn_service("php", cmd, settings.php_port)
    }

    pub(crate) fn start_caddy(&self) -> Result<()> {
        self.start_project_workers()?;
        let config = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let path = self.home.join("config/Caddyfile");
        let mut routes = caddy_config(
            &config.settings,
            &config.projects,
            &self.home.join("welcome"),
            &self.project_ports.lock().unwrap_or_else(|e| e.into_inner()),
        )?;
        if config.settings.phpmyadmin.enabled && self.executable("phpmyadmin").is_ok() {
            self.write_phpmyadmin_config()?;
            routes.push_str(&crate::phpmyadmin::caddy_route(
                &config.settings,
                &self.package_dir("phpmyadmin")?,
            ));
        }
        crate::storage::atomic_write(&path, routes)?;
        let mut cmd = command(self.executable("caddy")?);
        cmd.env("XDG_CONFIG_HOME", self.home.join("config"))
            .env("XDG_DATA_HOME", self.home.join("data"))
            .env("APPDATA", self.home.join("config"));
        cmd.args(["run", "--config"])
            .arg(path)
            .args(["--adapter", "caddyfile"]);
        self.spawn_service("caddy", cmd, config.settings.web_port)?;
        if config.settings.web.https {
            let https_port = config.settings.web.https_port;
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline
                && TcpStream::connect_timeout(
                    &SocketAddrV4::new(Ipv4Addr::LOCALHOST, https_port).into(),
                    Duration::from_millis(150),
                )
                .is_err()
            {
                std::thread::sleep(Duration::from_millis(200));
            }
            if TcpStream::connect_timeout(
                &SocketAddrV4::new(Ipv4Addr::LOCALHOST, https_port).into(),
                Duration::from_millis(150),
            )
            .is_err()
            {
                self.log(format!(
                    "HTTPS {https_port} portu henüz yanıt vermiyor. Caddy günlüğünü kontrol edin."
                ));
            }
            self.try_trust_https();
        }
        Ok(())
    }

    fn start_mysql(&self) -> Result<()> {
        if self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut("mysql")
            .is_some_and(|p| p.alive())
        {
            return Ok(());
        }
        self.check_mysql_data()?;
        let executable = self.executable("mysql")?;
        let port = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .mysql_port;
        port_free(port)?;
        let datadir = self.home.join("data/mysql-8.4");
        let passfile = self.home.join("config/mysql-password.dpapi");
        let ready = self.home.join("config/mysql-ready");
        if !datadir.exists() {
            crate::storage::require_space(&self.home.join("data"), 512 * 1024 * 1024)?;
            if passfile.exists() {
                secrets::read(&passfile)?;
            } else {
                let password = uuid::Uuid::new_v4().simple().to_string();
                secrets::save(&passfile, &password)?;
            }
            self.log("MySQL veri dizini ilk kez hazırlanıyor…");
            let stage = tempfile::tempdir_in(self.home.join("data"))?;
            let mut cmd = command(&executable);
            cmd.arg("--no-defaults")
                .arg("--initialize-insecure")
                .arg(format!(
                    "--basedir={}",
                    portable_path(&self.package_dir("mysql")?)
                ))
                .arg(format!("--datadir={}", portable_path(stage.path())));
            let mut child = ManagedChild::spawn(cmd, &self.home.join("logs/mysql.log"))?;
            child.wait_timeout(Duration::from_secs(180))?;
            fs::rename(stage.path(), &datadir)?;
        }
        let password = secrets::read(&passfile)?;
        let config = self.home.join("config/my.ini");
        let preferences = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .mysql
            .clone();
        crate::storage::atomic_write(&config, format!("[mysqld]\nbasedir=\"{}\"\ndatadir=\"{}\"\nport={port}\nbind-address=127.0.0.1\nmysqlx=OFF\nlocal-infile=OFF\nsecure-file-priv=NULL\ncharacter-set-server=utf8mb4\ncollation-server={}\ninnodb-buffer-pool-size={}M\nmax-connections={}\nmax-allowed-packet={}M\nwait-timeout={}\nsql-mode=\"{}\"\nslow-query-log={}\nlong-query-time={}\n", portable_path(&self.package_dir("mysql")?), portable_path(&datadir), preferences.collation, preferences.buffer_pool_mb, preferences.max_connections, preferences.max_packet_mb, preferences.wait_seconds, preferences.sql_mode, u8::from(preferences.slow_query_log), preferences.long_query_seconds))?;
        let mut bootstrap = None;
        let mut cmd = command(executable);
        cmd.arg(format!("--defaults-file={}", portable_path(&config)))
            // Windows MySQL option-file paths use the system codepage. Native
            // arguments preserve Unicode paths, as in initialization above.
            .arg(format!(
                "--basedir={}",
                portable_path(&self.package_dir("mysql")?)
            ))
            .arg(format!("--datadir={}", portable_path(&datadir)))
            .arg("--console");
        if !ready.exists() {
            let mut file = tempfile::NamedTempFile::new_in(self.home.join("config"))?;
            writeln!(
                file,
                "ALTER USER 'root'@'localhost' IDENTIFIED BY '{password}';"
            )?;
            file.flush()?;
            cmd.arg(format!("--init-file={}", portable_path(file.path())));
            bootstrap = Some(file);
        }
        self.spawn_service("mysql", cmd, port)?;
        let result = (|| {
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                if self.mysql_query("SELECT 1").is_ok() {
                    break;
                }
                if Instant::now() > deadline {
                    bail!("MySQL bağlantısı doğrulanamadı. Günlükleri kontrol edin.");
                }
                std::thread::sleep(Duration::from_millis(250));
            }
            crate::storage::atomic_write(&ready, "8.4")?;
            Ok(())
        })();
        drop(bootstrap);
        if result.is_err() {
            self.processes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove("mysql");
        }
        result
    }

    pub fn stop(&self, id: &str) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        if id == "all" {
            self.stop_inner()
        } else {
            if id == "php" {
                self.stop_service("caddy")?;
                self.stop_project_jobs()?;
                self.stop_project_workers()?;
                self.project_ports
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clear();
            }
            self.stop_service(id)
        }
    }

    pub(crate) fn stop_inner(&self) -> Result<()> {
        let mut ids: Vec<String> = self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect();
        // Stop listeners first and the database last; attempt every owned child.
        ids.sort_by_key(|id| {
            if id == "caddy" {
                0
            } else if id == "mysql" || id == crate::postgres::ID {
                2
            } else {
                1
            }
        });
        let mut failures = Vec::new();
        for id in ids {
            if let Err(error) = self.stop_service(&id) {
                failures.push(format!("{id}: {error:#}"));
            }
        }
        self.project_ports
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        if !failures.is_empty() {
            bail!("{}", failures.join("; "));
        }
        Ok(())
    }

    pub(crate) fn check_mysql_data(&self) -> Result<()> {
        let data = self.home.join("data/mysql-8.4");
        let ready = self.home.join("config/mysql-ready");
        let password = self.home.join("config/mysql-password.dpapi");
        if ready.exists() && !data.is_dir() {
            bail!("MySQL veri klasörü kayıp. Boş veritabanı oluşturulmadı; veri klasörünü/yedeğinizi geri getirin.");
        }
        if data.exists() && (!data.is_dir() || !password.is_file()) {
            bail!(
                "MySQL veri klasörü veya parola dosyası tutarsız. Mevcut veriler değiştirilmedi."
            );
        }
        if password.exists() {
            secrets::read(&password)
                .context("MySQL parolası okunamadı; mevcut parola ve veriler korunuyor.")?;
        }
        Ok(())
    }

    pub(crate) fn stop_service(&self, id: &str) -> Result<()> {
        if ![
            "caddy",
            "php",
            "mysql",
            crate::tunnel::ID,
            crate::mail::ID,
            crate::postgres::ID,
            crate::redis::ID,
        ]
        .contains(&id)
            && !Self::is_project_service_id(id)
            && !Self::is_job_service_id(id)
        {
            bail!("Bilinmeyen servis.");
        }
        let child = self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        if let Some(mut child) = child {
            if id == "mysql" && child.alive() {
                let shutdown =
                    self.mysql_command("mysqladmin.exe")
                        .and_then(|(mut cmd, _credentials)| {
                            cmd.arg("shutdown");
                            let mut shutdown =
                                ManagedChild::spawn(cmd, &self.home.join("logs/mysql.log"))?;
                            shutdown.wait_timeout(Duration::from_secs(20))
                        });
                if shutdown.is_ok() {
                    let _ = child.wait_timeout(Duration::from_secs(30));
                } else {
                    self.log("MySQL normal kapatma başarısız; sahip olunan süreç sonlandırılıyor.");
                }
            }
            if id == crate::postgres::ID && child.alive() {
                if self.shutdown_postgres().is_ok() {
                    let _ = child.wait_timeout(Duration::from_secs(30));
                } else {
                    self.log(
                        "PostgreSQL normal kapatma başarısız; sahip olunan süreç sonlandırılıyor.",
                    );
                }
            }
            drop(child);
            self.log(format!("{id} durduruldu."));
        }
        Ok(())
    }

    fn mysql_command(&self, binary: &str) -> Result<(Command, tempfile::NamedTempFile)> {
        let port = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .mysql_port;
        // Startup probes need the secret before mysql-ready is written.
        let password = secrets::read(&self.home.join("config/mysql-password.dpapi"))?;
        let mut file = tempfile::NamedTempFile::new_in(self.home.join("config"))?;
        writeln!(file, "[client]\nuser=root\npassword={password}\nhost=127.0.0.1\nport={port}\nprotocol=TCP\ndefault-character-set=utf8mb4")?;
        file.flush()?;
        let mut cmd = command(self.package_dir("mysql")?.join("bin").join(binary));
        cmd.current_dir(&self.home);
        cmd.arg(format!(
            "--defaults-extra-file=config/{}",
            file.path().file_name().unwrap().to_string_lossy()
        ));
        if binary != "mysqldump.exe" {
            cmd.arg("--connect-timeout=5");
        }
        Ok((cmd, file))
    }

    pub fn change_mysql_password(&self, password: &str) -> Result<()> {
        let _guard = self.gate()?;
        crate::model::validate_mysql_password(password)?;
        if !self.mysql_is_running() {
            bail!("Parolayı değiştirmek için önce MySQL'i başlatın.");
        }
        if !self.home.join("config/mysql-ready").is_file() {
            bail!("MySQL henüz ilk kurulumu bitirmedi. Önce ortamı bir kez başlatın.");
        }
        self.mysql_query(&format!(
            "ALTER USER 'root'@'localhost' IDENTIFIED BY '{password}'"
        ))?;
        crate::secrets::save(&self.home.join("config/mysql-password.dpapi"), password)?;
        self.log("MySQL root parolası güncellendi. Proje .env dosyaları yazılmadı.");
        Ok(())
    }

    pub fn mysql_query(&self, sql: &str) -> Result<String> {
        let (mut cmd, _credentials) = self.mysql_command("mysql.exe")?;
        cmd.args(["--batch", "--skip-column-names", "--execute", sql]);
        let output = ManagedChild::output(cmd, Duration::from_secs(30))?;
        if !output.status.success() {
            bail!(
                "MySQL sorgusu başarısız: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().into())
    }

    fn mysql_is_running(&self) -> bool {
        self.processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut("mysql")
            .is_some_and(|child| child.alive())
    }

    pub(crate) fn create_database_inner(&self, name: &str) -> Result<()> {
        crate::model::validate_slug(name)?;
        if !self.mysql_is_running() {
            bail!("Önce MySQL'i başlatın.");
        }
        let db = crate::model::database_name(name);
        let collation = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .mysql
            .collation
            .clone();
        self.mysql_query(&format!(
            "CREATE DATABASE IF NOT EXISTS `{db}` CHARACTER SET utf8mb4 COLLATE {collation}"
        ))?;
        self.log(format!("Veritabanı hazır: {db}"));
        Ok(())
    }

    pub(crate) fn maybe_create_project_database(&self, name: &str) {
        if !self.mysql_is_running() {
            self.log(format!(
                "MySQL kapalı; {name} için veritabanını sonra oluşturabilirsiniz."
            ));
            return;
        }
        if let Err(error) = self.create_database_inner(name) {
            self.log(format!("{name} veritabanı oluşturulamadı: {error:#}"));
        }
    }

    pub fn create_database(&self, name: &str) -> Result<()> {
        let _guard = self.gate()?;
        self.create_database_inner(name)
    }

    pub fn backup_database(&self, name: &str) -> Result<String> {
        let _guard = self.gate()?;
        crate::model::validate_slug(name)?;
        if !self.mysql_is_running() {
            bail!("Yedek almadan önce MySQL'i başlatın.");
        }
        let database = crate::model::database_name(name);
        let custom = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .backups_dir
            .clone();
        let destination = if custom.is_empty() {
            self.home.join("backups")
        } else {
            std::path::PathBuf::from(custom)
        };
        let path = destination.join(format!(
            "{database}-{}-{}.sql",
            chrono::Local::now().format("%Y%m%d-%H%M%S"),
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        ));
        let (mut cmd, _credentials) = self.mysql_command("mysqldump.exe")?;
        let temporary = tempfile::NamedTempFile::new_in(self.home.join("backups"))?;
        cmd.args([
            "--single-transaction",
            "--routines",
            "--events",
            "--set-gtid-purged=OFF",
        ])
        .arg(format!(
            "--result-file=backups/{}",
            temporary.path().file_name().unwrap().to_string_lossy()
        ))
        .arg(&database);
        let mut child = ManagedChild::spawn(cmd, &self.home.join("logs/mysql.log"))?;
        child.wait_timeout(Duration::from_secs(300))?;
        // The MySQL client writes through an ASCII relative name, then Rust handles
        // the user-selected Unicode destination and cross-volume copy atomically.
        let mut final_file = tempfile::NamedTempFile::new_in(&destination)?;
        std::io::copy(&mut std::fs::File::open(temporary.path())?, &mut final_file)?;
        final_file.as_file().sync_all()?;
        final_file.persist_noclobber(&path)?;
        self.log(format!("{database} yedeği alındı."));
        Ok(portable_path(&path))
    }

    pub fn restore_database(&self, name: &str, path: PathBuf) -> Result<()> {
        let _guard = self.gate()?;
        crate::model::validate_slug(name)?;
        if !self.mysql_is_running() {
            bail!("Geri yüklemeden önce MySQL'i başlatın.");
        }
        let source = dunce::canonicalize(&path).context("SQL dosyası bulunamadı.")?;
        if !source.is_file() {
            bail!("SQL yedeği bir dosya olmalı.");
        }
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if !extension.eq_ignore_ascii_case("sql") {
            bail!("Yalnızca .sql dosyaları geri yüklenebilir.");
        }
        let size = source.metadata()?.len();
        if size == 0 {
            bail!("SQL dosyası boş.");
        }
        if size > 512 * 1024 * 1024 {
            bail!("SQL dosyası 512 MB sınırını aşıyor.");
        }
        self.create_database_inner(name)?;
        let database = crate::model::database_name(name);
        let mut temporary = tempfile::NamedTempFile::new_in(self.home.join("backups"))?;
        std::io::copy(&mut std::fs::File::open(&source)?, temporary.as_file_mut())?;
        temporary.as_file().sync_all()?;
        let (mut cmd, _credentials) = self.mysql_command("mysql.exe")?;
        cmd.arg("--one-database").arg(&database);
        let stdin = std::fs::File::open(temporary.path())?;
        let mut child =
            ManagedChild::spawn_with_stdin(cmd, &self.home.join("logs/mysql.log"), stdin)?;
        child.wait_timeout(Duration::from_secs(300))?;
        self.log(format!(
            "{database} veritabanı geri yüklendi: {}",
            portable_path(&source)
        ));
        Ok(())
    }
}
