//! Mailpit: install/repair, start/stop on the configured SMTP and web ports,
//! optional `sendmail_path` relay for PHP `mail()`, and the state shown on
//! the E-posta card.

use crate::{model::tool_package, preferences::MailSettings, process::command, Manager};
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;

pub const ID: &str = crate::domain::ComponentId::Mailpit.as_str();

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailState {
    pub version: String,
    pub installed: bool,
    pub repairable: bool,
    pub running: bool,
    pub pid: Option<u32>,
    pub smtp_port: u16,
    pub web_port: u16,
    pub auto_start: bool,
    pub relay_php_mail: bool,
    pub issue: Option<String>,
}

impl Manager {
    pub(crate) fn mail_state_with(
        &self,
        processes: &HashMap<String, crate::process::ManagedChild>,
        settings: &MailSettings,
    ) -> MailState {
        let package = tool_package(ID).expect("embedded mailpit package");
        let health = self.tool_health(ID);
        MailState {
            version: package.version.clone(),
            installed: health.installed,
            repairable: health.repairable,
            running: processes.contains_key(ID),
            pid: processes.get(ID).map(|child| child.child.id()),
            smtp_port: settings.smtp_port,
            web_port: settings.web_port,
            auto_start: settings.auto_start,
            relay_php_mail: settings.relay_php_mail,
            issue: health.issue,
        }
    }

    pub fn install_mail(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.install_tool(ID)
    }

    pub fn repair_mail(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.stop_service(ID)?;
        self.repair_tool(ID)
    }

    pub fn start_mail(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.start_mail_inner()
    }

    pub(crate) fn start_mail_inner(&self) -> Result<()> {
        let settings = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .mail
            .clone();
        let executable = self
            .tool_executable(ID)
            .context("Mailpit kurulu değil. Hizmetler → E-posta ekranından kurun.")?;
        let database = self.home.join("data/mailpit/mailpit.db");
        std::fs::create_dir_all(database.parent().unwrap())
            .context("Mailpit veri klasörü oluşturulamadı.")?;
        let mut cmd = command(executable);
        cmd.args([
            "--listen".into(),
            format!("127.0.0.1:{}", settings.web_port),
            "--smtp".into(),
            format!("127.0.0.1:{}", settings.smtp_port),
            "--database".into(),
            database.to_string_lossy().into_owned(),
            "--max".into(),
            settings.max_messages.to_string(),
        ]);
        self.spawn_service(ID, cmd, settings.web_port)?;
        self.log(format!(
            "Mailpit çalışıyor. SMTP 127.0.0.1:{} · arayüz http://127.0.0.1:{}",
            settings.smtp_port, settings.web_port
        ));
        Ok(())
    }

    pub fn stop_mail(&self) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        self.stop_service(ID)
    }

    pub fn open_mail(&self) -> Result<()> {
        let port = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .mail
            .web_port;
        if !self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(ID)
        {
            anyhow::bail!("Mailpit çalışmıyor. Hizmetler → E-posta ekranından başlatın.");
        }
        command("rundll32.exe")
            .args([
                "url.dll,FileProtocolHandler",
                &format!("http://127.0.0.1:{port}"),
            ])
            .spawn()?;
        Ok(())
    }

    /// Environment start never fails because of the mail catcher; the error is
    /// reported on the mail card instead.
    pub(crate) fn start_mail_autostart(&self) {
        let auto_start = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .mail
            .auto_start;
        if !auto_start {
            return;
        }
        if let Err(error) = self.start_mail_inner() {
            let message = format!("Mailpit başlatılamadı: {error:#}");
            self.log(&message);
            self.service_errors
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(ID.into(), message);
        }
    }
}
