//! Authenticated HTTP bridge to desktop-only operations.
use crate::{desktop, State};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use serverbond_core::api::{DesktopApi, DesktopReply};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager as _};
use tauri_plugin_updater::UpdaterExt;

pub struct Host {
    app: AppHandle,
    update: Arc<Mutex<Value>>,
    updating: Arc<std::sync::atomic::AtomicBool>,
}

impl Host {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            update: Arc::new(Mutex::new(json!({"phase":"idle"}))),
            updating: Default::default(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    preferences: desktop::Preferences,
    autostart: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Navigate {
    page: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AppearanceInput {
    theme: crate::appearance::Theme,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Install {
    version: String,
    confirm: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateLink {
    url: String,
}

fn approved_install(body: Value) -> Result<Install> {
    let input: Install = serde_json::from_value(body)?;
    if !input.confirm || input.version.trim().is_empty() {
        bail!("Güncelleme için version ve confirm:true gerekli.");
    }
    Ok(input)
}

fn page_valid(page: &str) -> bool {
    matches!(
        page,
        "overview" | "packages" | "projects" | "logs" | "services" | "api" | "settings" | "updates"
    )
}

impl DesktopApi for Host {
    fn call(&self, operation: &str, body: Value) -> Result<DesktopReply> {
        let app = self.app.clone();
        let manager = app.state::<State>().inner().clone();
        let result = match operation {
            "appearance-get" => {
                json!({"theme":app.state::<crate::appearance::Appearance>().get()?})
            }
            "appearance-save" => {
                let input: AppearanceInput = serde_json::from_value(body)?;
                json!({"theme":crate::appearance::save(&app, input.theme, false)?})
            }
            "status" => serde_json::to_value(app.state::<desktop::Desktop>().status())?,
            "save" => {
                let input: Save = serde_json::from_value(body)?;
                app.state::<desktop::Desktop>().save(
                    input.preferences,
                    input.autostart,
                    &manager.home,
                )?;
                serde_json::to_value(app.state::<desktop::Desktop>().status())?
            }
            "show" => {
                desktop::show(&app, None);
                json!({"shown":true})
            }
            "hide" | "menu" => {
                crate::desktop_action(app.clone(), operation.into()).map_err(anyhow::Error::msg)?;
                json!({"action":operation})
            }
            "navigate" => {
                let input: Navigate = serde_json::from_value(body)?;
                if !page_valid(&input.page) {
                    bail!("Bilinmeyen sayfa.");
                }
                desktop::show(&app, Some(&input.page));
                json!({"page":input.page})
            }
            "exit" | "restart" => {
                if manager.is_busy() {
                    bail!("Başka bir işlem devam ediyor. Tamamlanmasını bekleyin.");
                }
                let restart = operation == "restart";
                return Ok(DesktopReply {
                    data: json!({"accepted":true,"action":operation}),
                    after_response: Some(Box::new(move || {
                        if restart {
                            manager.shutdown()?;
                            app.state::<desktop::Desktop>()
                                .exit_ready
                                .store(true, std::sync::atomic::Ordering::Release);
                            app.restart();
                        } else {
                            desktop::request_exit(&app);
                        }
                        Ok(())
                    })),
                });
            }
            "update-status" => self
                .update
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            "update-check" => serde_json::to_value(crate::updates::check(&app)?)?,
            "update-open" => {
                let input: UpdateLink = serde_json::from_value(body)?;
                serverbond_core::updates::open_release_link(&input.url)?;
                json!({"opened":true})
            }
            "update-install" => {
                let input = approved_install(body)?;
                if manager.is_busy() || manager.snapshot()?.any_running {
                    bail!("Güncellemeden önce sunucuyu durdurun ve çalışan işlemin bitmesini bekleyin.");
                }
                use std::sync::atomic::Ordering;
                if self.updating.swap(true, Ordering::AcqRel) {
                    bail!("Bir güncelleme zaten devam ediyor.");
                }
                let state = self.update.clone();
                *state.lock().unwrap_or_else(|e| e.into_inner()) =
                    json!({"phase":"downloading","version":input.version,"downloaded":0});
                let download = (|| -> Result<_> {
                    let update = tauri::async_runtime::block_on(
                        app.updater_builder()
                            .timeout(std::time::Duration::from_secs(300))
                            .build()?
                            .check(),
                    )?
                    .context("Yüklenecek imzalı güncelleme bulunamadı.")?;
                    if update.version != input.version {
                        bail!("Yayın sürümü değişti; güncellemeyi yeniden denetleyin ve yeni sürümü onaylayın.");
                    }
                    let mut downloaded = 0;
                    let bytes = tauri::async_runtime::block_on(update.download(|chunk, total| {
                        downloaded += chunk;
                        *state.lock().unwrap_or_else(|e| e.into_inner()) = json!({"phase":"downloading","version":input.version,"downloaded":downloaded,"total":total});
                    }, || {}))?;
                    Ok((update, bytes))
                })();
                let (update, bytes) = match download {
                    Ok(result) => result,
                    Err(error) => {
                        self.updating.store(false, Ordering::Release);
                        *state.lock().unwrap_or_else(|e| e.into_inner()) =
                            json!({"phase":"failed","error":format!("{error:#}")});
                        return Err(error);
                    }
                };
                *state.lock().unwrap_or_else(|e| e.into_inner()) =
                    json!({"phase":"ready","version":input.version});
                let updating = self.updating.clone();
                return Ok(DesktopReply {
                    data: json!({"accepted":true,"version":input.version,"signatureVerified":true}),
                    after_response: Some(Box::new(move || {
                        *state.lock().unwrap_or_else(|e| e.into_inner()) =
                            json!({"phase":"installing","version":update.version});
                        let result = manager
                            .shutdown()
                            .and_then(|()| update.install(bytes).map_err(anyhow::Error::from));
                        if let Err(error) = &result {
                            updating.store(false, Ordering::Release);
                            *state.lock().unwrap_or_else(|e| e.into_inner()) =
                                json!({"phase":"failed","error":format!("{error:#}")});
                            desktop::report(&app, format!("Güncelleme tamamlanamadı: {error:#}. ServerBond'ı yeniden başlatın."));
                        }
                        result
                    })),
                });
            }
            _ => bail!("Bilinmeyen masaüstü API işlemi."),
        };
        Ok(DesktopReply::immediate(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn install_requires_an_explicit_version_and_confirmation() {
        for value in [
            json!({}),
            json!({"confirm":false,"version":"1.2.0"}),
            json!({"confirm":true,"version":""}),
            json!({"confirm":true,"version":"1.2.0","url":"https://example.com"}),
        ] {
            assert!(approved_install(value).is_err());
        }
        assert!(approved_install(json!({"confirm":true,"version":"1.2.0"})).is_ok());
    }
    #[test]
    fn navigation_is_limited_to_known_pages() {
        assert!(page_valid("updates"));
        assert!(page_valid("projects"));
        assert!(!page_valid("https://example.com"));
        assert!(!page_valid(""));
    }
}
