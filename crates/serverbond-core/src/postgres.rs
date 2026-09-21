//! Optional PostgreSQL 17: `initdb` on first start, `pg_ctl` lifecycle,
//! DPAPI-stored superuser password with safe rotation, and `psql` helpers.

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
        processes: &HashMap<String, u32>,
        settings: &PostgresSettings,
    ) -> PostgresState {
        let package = tool_package(ID).expect("embedded postgres package");
        let health = self.tool_health(ID);
        PostgresState {
            version: package.version.clone(),
            installed: health.installed,
            repairable: health.repairable,
            running: processes.contains_key(ID),
            pid: processes.get(ID).copied(),
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
        // Prove the new secret can be encrypted and stored before the server
        // learns it; a DPAPI failure after ALTER would lock the user out.
        let staged = self.postgres_password_path().with_extension("dpapi.next");
        secrets::save(&staged, password)?;
        if secrets::read(&staged).ok().as_deref() != Some(password) {
            let _ = std::fs::remove_file(&staged);
            bail!("Yeni parola şifrelenip geri okunamadı; parola değiştirilmedi.");
        }
        if self
            .psql_query(
                &current,
                &format!("ALTER USER postgres PASSWORD '{password}';"),
            )
            .is_err()
        {
            let _ = std::fs::remove_file(&staged);
            // PostgreSQL diagnostics may quote the ALTER statement, including the password.
            bail!("PostgreSQL parola değişikliği doğrulanamadı. Sunucu durumunu kontrol edin.");
        }
        std::fs::rename(&staged, self.postgres_password_path()).with_context(|| {
            format!(
                "PostgreSQL parolası sunucuda değişti ancak kayıt yenilenemedi. Yeni parola {} dosyasında.",
                staged.display()
            )
        })?;
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
            "-X",
            "-f",
            "-",
        ])
        .env("PGPASSWORD", password)
        .env("PGCLIENTENCODING", "UTF8");
        let mut input = tempfile::tempfile()?;
        input.write_all(sql.as_bytes())?;
        std::io::Seek::rewind(&mut input)?;
        let output = ManagedChild::output_with_stdin(cmd, Duration::from_secs(20), input)?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            bail!("{}", if stderr.is_empty() { stdout } else { stderr });
        }
        Ok(stdout)
    }
}

#[cfg(all(test, windows))]
mod credential_tests {
    use super::*;

    #[test]
    #[ignore = "Installs PostgreSQL and starts a disposable instance"]
    fn postgres_rotation_keeps_credentials_out_of_arguments_and_preserves_access() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut settings = manager.snapshot().unwrap().settings;
        settings.postgres.port = listener.local_addr().unwrap().port();
        drop(listener);
        manager.save_settings(settings).unwrap();
        if let Some(cache) = std::env::var_os("SERVERBOND_TEST_CACHE") {
            for entry in std::fs::read_dir(cache).unwrap().flatten() {
                if entry.file_type().unwrap().is_file() {
                    std::fs::copy(
                        entry.path(),
                        home.path().join("cache").join(entry.file_name()),
                    )
                    .unwrap();
                }
            }
        }
        manager.install_postgres().unwrap();
        manager.start_postgres().unwrap();
        let old = manager.postgres_credentials().unwrap();
        assert_eq!(
            manager.psql_query(&old, "SELECT 'İstanbul';").unwrap(),
            "İstanbul"
        );
        let next = format!("Test-{}", uuid::Uuid::new_v4().simple());
        let staged = manager
            .postgres_password_path()
            .with_extension("dpapi.next");
        std::fs::create_dir(&staged).unwrap();
        assert!(manager.change_postgres_password(&next).is_err());
        assert!(manager.postgres_credentials().unwrap() == old);
        assert_eq!(manager.psql_query(&old, "SELECT 1;").unwrap(), "1");
        std::fs::remove_dir(&staged).unwrap();
        manager.change_postgres_password(&next).unwrap();
        assert!(manager.postgres_credentials().unwrap() == next);
        assert!(manager.psql_query(&old, "SELECT 1;").is_err());
        assert_eq!(
            manager.psql_query(&next, "SELECT 'İstanbul';").unwrap(),
            "İstanbul"
        );
        assert!(!staged.exists());
        assert!(!String::from_utf8_lossy(
            &std::fs::read(manager.postgres_password_path()).unwrap()
        )
        .contains(&next));
        assert!(
            !std::fs::read_to_string(home.path().join("logs/serverbond.log"))
                .unwrap()
                .contains(&next)
        );
        manager.stop_postgres().unwrap();
    }
}
