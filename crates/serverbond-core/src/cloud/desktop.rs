use crate::Manager;
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Appearance {
    theme: Theme,
    confirm: bool,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Preferences {
    close_to_tray: bool,
    start_minimized: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Save {
    preferences: Preferences,
    autostart: bool,
    confirm: bool,
}
pub(super) fn save(manager: &Manager, input: Save) -> Result<Value> {
    ensure!(
        input.confirm,
        "Masaüstü tercihlerini değiştirmek için açık onay gerekli."
    );
    call(
        manager,
        "save",
        json!({"preferences":input.preferences,"autostart":input.autostart}),
    )?;
    show(manager)
}

fn call(manager: &Manager, operation: &str, input: Value) -> Result<Value> {
    let host = manager
        .desktop_api()
        .context("Masaüstü hostu bağlı değil.")?;
    let reply = host.call(operation, input)?;
    ensure!(
        reply.after_response.is_none(),
        "Bu işlem ertelenmiş masaüstü eylemi içeremez."
    );
    Ok(reply.data)
}
pub(super) fn show(manager: &Manager) -> Result<Value> {
    let state = call(manager, "status", json!({}))?;
    let appearance = call(manager, "appearance-get", json!({}))?;
    Ok(
        json!({"preferences":state["preferences"],"autostart":state["autostart"],"trayAvailable":state["trayAvailable"],"quitting":state["quitting"],"hasIssue":!state["issue"].is_null(),"theme":appearance["theme"]}),
    )
}
pub(super) fn appearance(manager: &Manager, input: Appearance) -> Result<Value> {
    ensure!(input.confirm, "Tema değişikliği için açık onay gerekli.");
    call(manager, "appearance-save", json!({"theme":input.theme}))?;
    show(manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{DesktopApi, DesktopReply};
    use std::sync::{Arc, Mutex};
    struct Host(Mutex<String>);
    struct PreferencesHost(Mutex<Value>);
    impl DesktopApi for PreferencesHost {
        fn call(&self, operation: &str, input: Value) -> Result<DesktopReply> {
            let mut state = self.0.lock().unwrap();
            match operation {
                "save" => {
                    state["preferences"] = input["preferences"].clone();
                    state["autostart"] = input["autostart"].clone();
                }
                "status" => {}
                "appearance-get" => return Ok(DesktopReply::immediate(json!({"theme":"system"}))),
                _ => anyhow::bail!("Unexpected host operation"),
            }
            Ok(DesktopReply::immediate(state.clone()))
        }
    }
    #[test]
    fn desktop_preferences_forward_only_confirmed_typed_fields() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let host = Arc::new(PreferencesHost(Mutex::new(
            json!({"preferences":{"closeToTray":true,"startMinimized":false},"autostart":false,"trayAvailable":true,"quitting":false,"issue":null}),
        )));
        manager.attach_desktop_api(host.clone());
        let input = |confirm| Save {
            preferences: Preferences {
                close_to_tray: false,
                start_minimized: true,
            },
            autostart: true,
            confirm,
        };
        assert!(save(&manager, input(false)).is_err());
        assert_eq!(host.0.lock().unwrap()["autostart"], false);
        let result = save(&manager, input(true)).unwrap();
        assert_eq!(result["autostart"], true);
        assert_eq!(result["preferences"]["closeToTray"], false);
        assert_eq!(result["preferences"]["startMinimized"], true);
        assert!(serde_json::from_value::<Save>(json!({"preferences":{"closeToTray":true,"startMinimized":false,"unknown":true},"autostart":true,"confirm":true})).is_err());
    }
    impl DesktopApi for Host {
        fn call(&self, operation: &str, input: Value) -> Result<DesktopReply> {
            let mut theme = self.0.lock().unwrap();
            Ok(DesktopReply::immediate(match operation {
                "status" => {
                    json!({"preferences":{"closeToTray":true,"startMinimized":false},"autostart":false,"trayAvailable":true,"quitting":false,"issue":null})
                }
                "appearance-get" => json!({"theme":*theme}),
                "appearance-save" => {
                    *theme = input["theme"].as_str().unwrap().into();
                    json!({"theme":*theme})
                }
                _ => anyhow::bail!("Unexpected host operation"),
            }))
        }
    }
    #[test]
    fn desktop_bridge_requires_host_and_confirmation() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        assert!(show(&manager).is_err());
        let host = Arc::new(Host(Mutex::new("system".into())));
        manager.attach_desktop_api(host.clone());
        assert!(appearance(
            &manager,
            Appearance {
                theme: Theme::Dark,
                confirm: false
            }
        )
        .is_err());
        assert_eq!(*host.0.lock().unwrap(), "system");
        assert_eq!(
            appearance(
                &manager,
                Appearance {
                    theme: Theme::Dark,
                    confirm: true
                }
            )
            .unwrap()["theme"],
            "dark"
        );
        assert!(show(&manager).unwrap().get("issue").is_none());
    }
}
