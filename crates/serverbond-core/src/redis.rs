//! Optional Redis 8: install/repair and a loopback-only server with
//! persistence in `data/redis` and a no-eviction memory policy so queued
//! jobs are never dropped silently.

use crate::{model::tool_package, preferences::RedisSettings, process::command, Manager};
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;

pub const ID: &str = crate::domain::ComponentId::Redis.as_str();

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisState {
    pub version: String,
    pub installed: bool,
    pub repairable: bool,
    pub running: bool,
    pub pid: Option<u32>,
    pub port: u16,
    pub auto_start: bool,
    pub issue: Option<String>,
}

impl Manager {
    pub(crate) fn redis_state_with(
        &self,
        processes: &HashMap<String, crate::process::ManagedChild>,
        settings: &RedisSettings,
    ) -> RedisState {
        let package = tool_package(ID).expect("embedded redis package");
        let health = self.tool_health(ID);
        RedisState {
            version: package.version.clone(),
            installed: health.installed,
            repairable: health.repairable,
            running: processes.contains_key(ID),
            pid: processes.get(ID).map(|child| child.child.id()),
            port: settings.port,
            auto_start: settings.auto_start,
            issue: health.issue,
        }
    }

    pub fn install_redis(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.install_tool(ID)?;
        self.log("Redis kuruldu. Hizmetler → Redis ekranından başlatın.");
        Ok(())
    }

    pub fn repair_redis(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.stop_service(ID)?;
        self.repair_tool(ID)
    }

    pub fn start_redis(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.start_redis_inner()
    }

    pub fn stop_redis(&self) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        self.stop_service(ID)
    }

    pub(crate) fn start_redis_inner(&self) -> Result<()> {
        let settings = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .redis
            .clone();
        if self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(ID)
            .is_some_and(|p| p.alive())
        {
            return Ok(());
        }
        // spawn_service checks the port itself once we know no owned instance holds it.
        let executable = self
            .tool_executable(ID)
            .context("Redis kurulu değil. Hizmetler → Redis ekranından kurun.")?;
        let datadir = self.home.join("data/redis");
        std::fs::create_dir_all(&datadir).context("Redis veri klasörü oluşturulamadı.")?;
        let workdir = executable
            .parent()
            .context("Redis kurulum klasörü bulunamadı.")?
            .to_path_buf();
        let mut cmd = command(executable);
        cmd.current_dir(&workdir);
        cmd.args([
            "--bind".into(),
            "127.0.0.1".into(),
            "--port".into(),
            settings.port.to_string(),
            "--dir".into(),
            crate::portable_path(&datadir),
            "--dbfilename".into(),
            "dump.rdb".into(),
            "--protected-mode".into(),
            "yes".into(),
            "--daemonize".into(),
            "no".into(),
            "--logfile".into(),
            String::new(),
            "--maxmemory".into(),
            "256mb".into(),
            // Laravel queues and sessions live here: refuse writes when full
            // instead of silently evicting jobs the way allkeys-lru would.
            "--maxmemory-policy".into(),
            "noeviction".into(),
        ]);
        self.spawn_service(ID, cmd, settings.port)?;
        self.log(format!(
            "Redis çalışıyor. 127.0.0.1:{} · parola yok",
            settings.port
        ));
        Ok(())
    }

    /// Environment start never fails because of Redis; the error is reported
    /// on the Redis card instead.
    pub(crate) fn start_redis_autostart(&self) {
        let auto_start = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .redis
            .auto_start;
        if !auto_start {
            return;
        }
        if let Err(error) = self.start_redis_inner() {
            let message = format!("Redis başlatılamadı: {error:#}");
            self.log(&message);
            self.service_errors
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(ID.into(), message);
        }
    }
}
