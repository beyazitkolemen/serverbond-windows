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
