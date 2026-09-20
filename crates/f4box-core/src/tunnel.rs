use crate::{model::tool_package, process::command, process::ManagedChild, secrets, Manager};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf, time::Duration};

pub const ID: &str = crate::domain::ComponentId::Cloudflared.as_str();
/// cloudflared prints this once a tunnel connection is registered with Cloudflare.
const READY_MARKER: &str = "Registered tunnel connection";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelState {
    pub version: String,
    pub installed: bool,
    pub repairable: bool,
    pub running: bool,
    pub pid: Option<u32>,
    pub token_saved: bool,
    pub auto_start: bool,
    pub issue: Option<String>,
}

/// Pulls the connector token out of a dashboard paste. People copy the whole
/// `cloudflared service install …` line; only the token is stored.
pub fn normalize_token(raw: &str) -> Result<String> {
    let trimmed = raw.trim().trim_matches(['"', '\'']);
    let lower = trimmed.to_ascii_lowercase();
    let candidate = if lower.contains("cloudflared")
        || lower.contains("service install")
        || lower.contains("--token")
    {
        trimmed
            .split_whitespace()
            .last()
            .unwrap_or(trimmed)
            .trim_matches(['"', '\''])
    } else {
        trimmed
    };
    let token: String = candidate.chars().filter(|c| !c.is_whitespace()).collect();
    if !(40..=8192).contains(&token.len()) {
        bail!("Tünel jetonu 40–8192 karakter olmalı. Cloudflare Zero Trust → Tunnels → Install and run a connector adımındaki jetonu olduğu gibi yapıştırın.");
    }
    if !token
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"=-_+./".contains(&b))
    {
        bail!("Tünel jetonu yalnızca harf, rakam ve = - _ + . / karakterlerini içerebilir. Komut satırının tamamını yapıştırsanız da olur; ServerBond jetonu ayıklar.");
    }
    Ok(token)
}

impl Manager {
    pub(crate) fn tunnel_token_path(&self) -> PathBuf {
        self.home.join("config/cloudflared-token.dpapi")
    }

    pub(crate) fn tunnel_state_with(
        &self,
        processes: &HashMap<String, ManagedChild>,
        auto_start: bool,
    ) -> TunnelState {
        let package = tool_package(ID).expect("embedded cloudflared package");
        let health = self.tool_health(ID);
        TunnelState {
            version: package.version,
            installed: health.installed,
            repairable: health.repairable,
            running: processes.contains_key(ID),
            pid: processes.get(ID).map(|child| child.child.id()),
            token_saved: self.tunnel_token_path().is_file(),
            auto_start,
            issue: health.issue,
        }
    }

    pub fn install_tunnel(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.install_tool(ID)
    }

    pub fn repair_tunnel(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.stop_service(ID)?;
        self.repair_tool(ID)
    }

    fn write_tunnel_token(&self, token: &str) -> Result<String> {
        let token = normalize_token(token)?;
        secrets::save(&self.tunnel_token_path(), &token)?;
        self.log("Cloudflare tünel jetonu Windows hesabınıza bağlı olarak şifrelendi.");
        Ok(token)
    }

    pub fn save_tunnel_token(&self, token: &str) -> Result<()> {
        let _guard = self.gate()?;
        self.write_tunnel_token(token)?;
        Ok(())
    }

    /// Saves the token from the app, installs cloudflared if needed, then starts
    /// the connector. One action covers the whole setup path.
    pub fn apply_tunnel(&self, token: &str) -> Result<()> {
        let _guard = self.gate()?;
        self.write_tunnel_token(token)?;
        self.ensure_cloudflared()?;
        self.start_tunnel_inner()
    }

    fn ensure_cloudflared(&self) -> Result<()> {
        if self.tool_executable(ID).is_ok() {
            return Ok(());
        }
        self.install_tool(ID)
    }

    pub fn clear_tunnel_token(&self) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        self.stop_service(ID)?;
        let path = self.tunnel_token_path();
        if path.exists() {
            std::fs::remove_file(&path).context("Jeton dosyası silinemedi.")?;
        }
        self.log("Cloudflare tünel jetonu silindi; tünel durduruldu.");
        Ok(())
    }

    pub fn save_tunnel_auto_start(&self, auto_start: bool) -> Result<()> {
        let _guard = self.gate()?;
        let mut config = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if config.settings.tunnel.auto_start == auto_start {
            return Ok(());
        }
        config.settings.tunnel.auto_start = auto_start;
        self.save_config(&config)?;
        self.log(if auto_start {
            "Cloudflare tüneli ortamla birlikte başlatılacak."
        } else {
            "Cloudflare tüneli yalnızca elle başlatılacak."
        });
        Ok(())
    }

    pub fn start_tunnel(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.start_tunnel_inner()
    }

    pub(crate) fn start_tunnel_inner(&self) -> Result<()> {
        let token = secrets::read(&self.tunnel_token_path())
            .context("Tünel jetonu okunamadı. Hizmetler → Tünel ekranından jetonu kaydedin.")?;
        let token = normalize_token(&token)?;
        self.ensure_cloudflared()
            .context("Cloudflared kurulamadı. Hizmetler → Tünel ekranından yeniden deneyin.")?;
        let executable = self
            .tool_executable(ID)
            .context("Cloudflared kurulu değil. Hizmetler → Tünel ekranından kurun.")?;
        let mut cmd = command(executable);
        // The token stays in the environment block; command lines are visible to
        // every process on the machine.
        cmd.args(["tunnel", "--no-autoupdate", "run"])
            .env("TUNNEL_TOKEN", token)
            .env("TUNNEL_METRICS", "127.0.0.1:0");
        self.spawn_watched(ID, cmd, READY_MARKER, Duration::from_secs(90))?;
        self.log("Cloudflare tüneli çalışıyor. Genel adres Cloudflare panelindeki tünel yapılandırmasına bağlıdır.");
        Ok(())
    }

    pub fn stop_tunnel(&self) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        self.stop_service(ID)
    }

    /// Environment start never fails because of the tunnel; the error is reported
    /// on the tunnel card instead.
    pub(crate) fn start_tunnel_autostart(&self) {
        let (auto_start, token) = {
            let config = self.config.lock().unwrap_or_else(|e| e.into_inner());
            (
                config.settings.tunnel.auto_start,
                self.tunnel_token_path().is_file(),
            )
        };
        if !auto_start || !token {
            return;
        }
        if let Err(error) = self.start_tunnel_inner() {
            let message = format!("Cloudflare tüneli başlatılamadı: {error:#}");
            self.log(&message);
            self.service_errors
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(ID.into(), message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_checked_before_they_are_stored() {
        assert_eq!(normalize_token(&"a".repeat(40)).unwrap(), "a".repeat(40));
        let jwt = format!("  {}  ", "eyJhIjoiYiJ9".repeat(4));
        assert_eq!(normalize_token(&jwt).unwrap(), "eyJhIjoiYiJ9".repeat(4));
        let command = format!(
            "cloudflared.exe service install {}",
            "eyJhIjoiYi+/._".repeat(6)
        );
        assert_eq!(
            normalize_token(&command).unwrap(),
            "eyJhIjoiYi+/._".repeat(6)
        );
        let wrapped = format!("\"{}\r\n{}\"", "eyJ".repeat(20), "a".repeat(20));
        assert!(normalize_token(&wrapped).unwrap().len() >= 40);
        for invalid in [
            "short",
            &"a".repeat(8193),
            &format!("{} & calc.exe", "a".repeat(40)),
        ] {
            assert!(normalize_token(invalid).is_err(), "{invalid}");
        }
    }
}
