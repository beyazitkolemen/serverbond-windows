use crate::{model::tool_package, process::command, process::ManagedChild, secrets, Manager};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf, time::Duration};

pub const ID: &str = "cloudflared";
/// cloudflared prints this once a tunnel connection is registered with Cloudflare.
const READY_MARKER: &str = "Registered tunnel connection";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelState {
    pub version: String,
    pub installed: bool,
    pub running: bool,
    pub pid: Option<u32>,
    pub token_saved: bool,
    pub auto_start: bool,
    pub issue: Option<String>,
}

/// Connector tokens are base64url text. Reject anything that could reach a shell
/// or a command line in an unexpected shape before it is stored.
pub fn validate_token(token: &str) -> Result<()> {
    let token = token.trim();
    if !(40..=4096).contains(&token.len()) {
        bail!("Tünel jetonu 40–4096 karakter olmalı. Cloudflare Zero Trust panelinde oluşturulan jetonu olduğu gibi yapıştırın.");
    }
    if !token
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"=-_.".contains(&b))
    {
        bail!("Tünel jetonu yalnızca harf, rakam ve = - _ . karakterlerini içerebilir.");
    }
    Ok(())
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
        let directory = self.home.join("bin").join(ID).join(&package.version);
        let installed = crate::install::validate_installation(&directory, &package).is_ok();
        TunnelState {
            version: package.version,
            installed,
            running: processes.contains_key(ID),
            pid: processes.get(ID).map(|child| child.child.id()),
            token_saved: self.tunnel_token_path().is_file(),
            auto_start,
            issue: self
                .service_errors
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(ID)
                .cloned(),
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

    pub fn save_tunnel_token(&self, token: &str) -> Result<()> {
        let _guard = self.gate()?;
        validate_token(token)?;
        secrets::save(&self.tunnel_token_path(), token.trim())?;
        self.log("Cloudflare tünel jetonu Windows hesabınıza bağlı olarak şifrelendi.");
        Ok(())
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
        let token = secrets::read(&self.tunnel_token_path()).context(
            "Tünel jetonu okunamadı. Ayarlar → Cloudflare tüneli ekranından jetonu kaydedin.",
        )?;
        validate_token(&token)?;
        let executable = self
            .tool_executable(ID)
            .context("Cloudflared kurulu değil. Ayarlar → Cloudflare tüneli ekranından kurun.")?;
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
        validate_token(&"a".repeat(40)).unwrap();
        validate_token(&format!("  {}  ", "eyJhIjoiYiJ9".repeat(4))).unwrap();
        for invalid in [
            "short",
            &"a".repeat(4097),
            &format!("{} & calc.exe", "a".repeat(40)),
            &format!("{}\r\n{}", "a".repeat(40), "b".repeat(40)),
        ] {
            assert!(validate_token(invalid).is_err(), "{invalid}");
        }
    }
}
