use crate::{
    model::{tool_package, validate_mysql_password},
    portable_path,
    preferences::PostgresSettings,
    process::{command, ManagedChild},
    secrets, storage, Manager,
};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

pub const ID: &str = crate::domain::ComponentId::Postgres.as_str();

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgresState {
    pub version: String,
    pub installed: bool,
    pub repairable: bool,
    pub running: bool,
    pub pid: Option<u32>,
    pub port: u16,
    pub auto_start: bool,
    pub password_saved: bool,
    pub issue: Option<String>,
}

impl Manager {
    pub(crate) fn postgres_state_with(
        &self,
        processes: &HashMap<String, crate::process::ManagedChild>,
        settings: &PostgresSettings,
    ) -> PostgresState {
        let package = tool_package(ID).expect("embedded postgres package");
        let health = self.tool_health(ID);
        PostgresState {
            version: package.version.clone(),
            installed: health.installed,
            repairable: health.repairable,
            running: processes.contains_key(ID),
            pid: processes.get(ID).map(|child| child.child.id()),
            port: settings.port,
            auto_start: settings.auto_start,
            password_saved: self.postgres_password_path().is_file(),
            issue: health.issue,
        }
    }

    pub fn install_postgres(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.install_tool(ID)?;
        self.log("PostgreSQL kuruldu. Hizmetler → PostgreSQL ekranından başlatın.");
        Ok(())
    }

    pub fn repair_postgres(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.stop_service(ID)?;
        self.repair_tool(ID)
    }

    pub fn start_postgres(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.start_postgres_inner()
    }

    pub fn stop_postgres(&self) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        self.stop_service(ID)
    }

    pub fn postgres_credentials(&self) -> Result<String> {
        if !self.postgres_ready_path().is_file() {
            bail!("PostgreSQL parolası henüz hazır değil. Önce PostgreSQL'i başlatın.");
        }
        secrets::read(&self.postgres_password_path())
    }

    pub fn change_postgres_password(&self, password: &str) -> Result<()> {
        let _guard = self.gate()?;
        validate_mysql_password(password)?;
        if !self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(ID)
        {
            bail!("Parolayı değiştirmek için önce PostgreSQL'i başlatın.");
        }
        let current = secrets::read(&self.postgres_password_path())?;
        self.psql_query(
            &current,
            &format!("ALTER USER postgres PASSWORD '{password}';"),
        )?;
        secrets::save(&self.postgres_password_path(), password)?;
        self.log("PostgreSQL parolası değiştirildi. Proje .env dosyası yazılmaz.");
        Ok(())
    }

    pub(crate) fn shutdown_postgres(&self) -> Result<()> {
        let bin = self.postgres_bin()?;
        let mut cmd = command(bin.join("pg_ctl.exe"));
        self.postgres_env(&mut cmd, &bin);
        cmd.args([
            "stop",
            "-D",
            &portable_path(&self.postgres_data_dir()),
            "-m",
            "fast",
            "-w",
            "-t",
            "20",
        ]);
        let mut child = ManagedChild::spawn(cmd, &self.home.join("logs/postgres.log"))?;
        child.wait_timeout(Duration::from_secs(30))
    }

    pub(crate) fn start_postgres_inner(&self) -> Result<()> {
        if self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(ID)
            .is_some_and(|p| p.alive())
        {
            return Ok(());
        }
        let settings = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .postgres
            .clone();
        crate::services::port_free(settings.port)?;
        let bin = self.postgres_bin()?;
        let datadir = self.postgres_data_dir();
        let passfile = self.postgres_password_path();
        if !datadir.exists() {
            crate::storage::require_space(&self.home.join("data"), 128 * 1024 * 1024)?;
            if !passfile.exists() {
                secrets::save(&passfile, &uuid::Uuid::new_v4().simple().to_string())?;
            }
            let password = secrets::read(&passfile)?;
            self.log("PostgreSQL veri dizini ilk kez hazırlanıyor…");
            let stage = tempfile::tempdir_in(self.home.join("data"))?;
            let mut pw = tempfile::NamedTempFile::new_in(self.home.join("config"))?;
            writeln!(pw, "{password}")?;
            pw.flush()?;
            let mut cmd = command(bin.join("initdb.exe"));
            self.postgres_env(&mut cmd, &bin);
            cmd.args([
                "-D",
                &portable_path(stage.path()),
                "--auth-local=scram-sha-256",
                "--auth-host=scram-sha-256",
                "--username=postgres",
                "--encoding=UTF8",
                "--no-locale",
                "--no-instructions",
            ])
            .arg(format!("--pwfile={}", portable_path(pw.path())));
            let mut child = ManagedChild::spawn(cmd, &self.home.join("logs/postgres.log"))?;
            child.wait_timeout(Duration::from_secs(180)).context(
                "PostgreSQL veri dizini oluşturulamadı. postgres günlüğünü kontrol edin.",
            )?;
            fs::rename(stage.path(), &datadir)?;
        } else if !passfile.exists() {
            bail!("PostgreSQL veri klasörü var ama parola dosyası yok. Veriler korunuyor.");
        }
        let mut cmd = command(bin.join("postgres.exe"));
        self.postgres_env(&mut cmd, &bin);
        cmd.args([
            "-D",
            &portable_path(&datadir),
            "-p",
            &settings.port.to_string(),
            "-h",
            "127.0.0.1",
        ]);
        self.spawn_service(ID, cmd, settings.port)?;
        let password = secrets::read(&passfile)?;
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if self.psql_query(&password, "SELECT 1;").is_ok() {
                break;
            }
            if Instant::now() > deadline {
                let _ = self.stop_service(ID);
                bail!("PostgreSQL bağlantısı doğrulanamadı. Günlükleri kontrol edin.");
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        storage::atomic_write(&self.postgres_ready_path(), b"ok")?;
        self.log(format!(
            "PostgreSQL çalışıyor. 127.0.0.1:{} · kullanıcı postgres",
            settings.port
        ));
        Ok(())
    }

    pub(crate) fn start_postgres_autostart(&self) {
        let auto_start = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .postgres
            .auto_start;
        if !auto_start {
            return;
        }
        if let Err(error) = self.start_postgres_inner() {
            let message = format!("PostgreSQL başlatılamadı: {error:#}");
            self.log(&message);
            self.service_errors
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(ID.into(), message);
        }
    }

    fn postgres_bin(&self) -> Result<PathBuf> {
        let executable = self
            .tool_executable(ID)
            .context("PostgreSQL kurulu değil. Hizmetler → PostgreSQL ekranından kurun.")?;
        Ok(executable
            .parent()
            .expect("postgres.exe lives in bin")
            .to_path_buf())
    }

    fn postgres_data_dir(&self) -> PathBuf {
        self.home.join("data/postgresql-17")
    }

    fn postgres_password_path(&self) -> PathBuf {
        self.home.join("config/postgres-password.dpapi")
    }

    fn postgres_ready_path(&self) -> PathBuf {
        self.home.join("config/postgres-ready")
    }

    fn postgres_env(&self, cmd: &mut std::process::Command, bin: &std::path::Path) {
        let mut paths = vec![bin.to_path_buf()];
        if let Some(lib) = bin.parent().map(|root| root.join("lib")) {
            paths.push(lib);
        }
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        if let Ok(joined) = std::env::join_paths(paths) {
            cmd.env("PATH", joined);
        }
    }

    fn psql_query(&self, password: &str, sql: &str) -> Result<String> {
        let settings = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .postgres
            .clone();
        let bin = self.postgres_bin()?;
        let mut cmd = command(bin.join("psql.exe"));
        self.postgres_env(&mut cmd, &bin);
        cmd.args([
            "-h",
            "127.0.0.1",
            "-p",
            &settings.port.to_string(),
            "-U",
            "postgres",
            "-d",
            "postgres",
            "-v",
            "ON_ERROR_STOP=1",
            "-tA",
            "-c",
            sql,
        ])
        .env("PGPASSWORD", password)
        .env("PGCLIENTENCODING", "UTF8");
        let output = ManagedChild::output(cmd, Duration::from_secs(20))?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            bail!("{}", if stderr.is_empty() { stdout } else { stderr });
        }
        Ok(stdout)
    }
}
