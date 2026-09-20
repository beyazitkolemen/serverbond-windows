use crate::{startup, State};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{AppHandle, Emitter, Manager as _};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub close_to_tray: bool,
    pub start_minimized: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            start_minimized: false,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopStatus {
    pub preferences: Preferences,
    pub autostart: Option<bool>,
    pub issue: Option<String>,
    pub tray_available: bool,
    pub quitting: bool,
}

pub fn start_hidden(autostart: bool, status: &DesktopStatus, recovery: bool) -> bool {
    autostart
        && status.preferences.start_minimized
        && status.tray_available
        && status.issue.is_none()
        && !recovery
}

pub struct Desktop {
    path: PathBuf,
    preferences: Mutex<Preferences>,
    issue: Mutex<Option<String>>,
    pub tray_available: AtomicBool,
    pub quitting: AtomicBool,
    pub exit_ready: AtomicBool,
    pub pending_page: Mutex<Option<String>>,
}

impl Desktop {
    pub fn new(home: &std::path::Path) -> Self {
        let path = home.join("config/desktop.json");
        let result = read_preferences(&path);
        let issue = result.as_ref().err().map(|e| format!("Masaüstü ayarları okunamadı: {e:#}. Varsayılanlar kullanılıyor; kaydettiğinizde eski dosya yedeklenir."));
        Self {
            path,
            preferences: Mutex::new(result.unwrap_or_default()),
            issue: Mutex::new(issue),
            tray_available: AtomicBool::new(false),
            quitting: AtomicBool::new(false),
            exit_ready: AtomicBool::new(false),
            pending_page: Mutex::new(None),
        }
    }
    pub fn preferences(&self) -> Preferences {
        self.preferences
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn status(&self) -> DesktopStatus {
        let registration = startup::Registration::read();
        let issue = registration
            .as_ref()
            .err()
            .map(|e| format!("{e:#}"))
            .or_else(|| self.issue.lock().unwrap_or_else(|e| e.into_inner()).clone());
        DesktopStatus {
            preferences: self.preferences(),
            autostart: registration.ok().map(|r| r.enabled()),
            issue,
            tray_available: self.tray_available.load(Ordering::Acquire),
            quitting: self.quitting.load(Ordering::Acquire),
        }
    }
    pub fn save(
        &self,
        preferences: Preferences,
        autostart: bool,
        home: &std::path::Path,
    ) -> Result<()> {
        let mut current = self
            .preferences
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Masaüstü ayarları güncelleniyor. Tekrar deneyin."))?;
        if self.quitting.load(Ordering::Acquire) {
            bail!("ServerBond kapanıyor.");
        }
        let parent = self.path.parent().context("Ayar klasörü bulunamadı.")?;
        let mut staged = tempfile::NamedTempFile::new_in(parent)?;
        staged.write_all(&serde_json::to_vec_pretty(&preferences)?)?;
        staged.as_file().sync_all()?;
        if self.path.try_exists()? {
            let mut previous = tempfile::NamedTempFile::new_in(parent)?;
            let source = std::fs::File::open(&self.path)?;
            if source.metadata()?.len() > 64 * 1024 {
                bail!("Masaüstü ayar dosyası 64 KB sınırını aşıyor. Dosya korunuyor; veri klasöründeki config/desktop.json dosyasını düzeltin.");
            }
            let copied = std::io::copy(&mut source.take(64 * 1024 + 1), &mut previous)?;
            if copied > 64 * 1024 {
                bail!("Masaüstü ayar dosyası okuma sırasında boyut sınırını aştı.");
            }
            previous.as_file().sync_all()?;
            previous.persist(parent.join("desktop.previous.json"))?;
        }
        let registration = startup::Registration::read()?;
        let command = startup::command(&std::env::current_exe()?, home);
        let registration_changed = registration.needs_change(autostart, &command);
        if registration_changed {
            registration.apply(autostart, &command)?;
        }
        if let Err(error) = staged.persist(&self.path) {
            if registration_changed {
                registration.restore().context("Ayarlar kaydedilemedi; Windows başlangıç kaydı geri alınamadı. Başlangıç Uygulamaları'nı kontrol edin.")?;
            }
            return Err(error)
                .context("Masaüstü ayarları kaydedilemedi; önceki başlangıç ayarı korundu.");
        }
        *current = preferences;
        *self.issue.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }
}

fn read_preferences(path: &std::path::Path) -> Result<Preferences> {
    if !path.try_exists()? {
        return Ok(Preferences::default());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(64 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 {
        bail!("Masaüstü ayar dosyası 64 KB sınırını aşıyor.");
    }
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn show(app: &AppHandle, page: Option<&str>) {
    if let Some(page) = page {
        if let Some(desktop) = app.try_state::<Desktop>() {
            *desktop
                .pending_page
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(page.into());
        }
        let _ = app.emit("desktop:navigate", page);
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn report(app: &AppHandle, error: impl std::fmt::Display) {
    let message = error.to_string();
    app.state::<State>().log(&message);
    let _ = app.emit("desktop:error", &message);
    show(app, Some("logs"));
}

pub fn request_exit(app: &AppHandle) {
    let desktop = app.state::<Desktop>();
    if desktop.quitting.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let manager = app.state::<State>();
        match manager.contain(|| manager.shutdown()) {
            Ok(()) => {
                app.state::<Desktop>()
                    .exit_ready
                    .store(true, Ordering::Release);
                app.exit(0);
            }
            Err(error) => {
                app.state::<Desktop>()
                    .quitting
                    .store(false, Ordering::Release);
                report(&app, format!("Çıkış tamamlanamadı: {error:#}"));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_launch_requires_startup_intent_tray_and_healthy_configuration() {
        let mut status = DesktopStatus {
            preferences: Preferences {
                close_to_tray: true,
                start_minimized: true,
            },
            autostart: Some(true),
            issue: None,
            tray_available: true,
            quitting: false,
        };
        assert!(start_hidden(true, &status, false));
        assert!(!start_hidden(false, &status, false));
        assert!(!start_hidden(true, &status, true));
        status.tray_available = false;
        assert!(!start_hidden(true, &status, false));
        status.tray_available = true;
        status.issue = Some("configuration failure".into());
        assert!(!start_hidden(true, &status, false));
    }
    #[test]
    fn corrupt_preferences_do_not_prevent_startup_or_overwrite_original() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join("config")).unwrap();
        let path = home.path().join("config/desktop.json");
        std::fs::write(&path, "broken").unwrap();
        let desktop = Desktop::new(home.path());
        assert!(desktop.issue.lock().unwrap().is_some());
        assert_eq!(desktop.preferences(), Preferences::default());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "broken");
    }
    #[test]
    fn settings_reader_rejects_large_files_and_unknown_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("desktop.json");
        std::fs::write(&path, r#"{"unexpected":true}"#).unwrap();
        assert!(read_preferences(&path).is_err());
        std::fs::File::create(&path)
            .unwrap()
            .set_len(65537)
            .unwrap();
        assert!(read_preferences(&path).is_err());
    }
}
