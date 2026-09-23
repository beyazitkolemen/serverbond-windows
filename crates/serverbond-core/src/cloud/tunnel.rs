use crate::Manager;
use anyhow::{ensure, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Token {
    token: String,
    confirm: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AutoStart {
    auto_start: bool,
    confirm: bool,
}

pub(super) fn show(manager: &Manager) -> Result<Value> {
    let state = manager.snapshot()?.tunnel;
    Ok(output(&state))
}

pub(super) fn output(state: &crate::tunnel::TunnelState) -> Value {
    json!({"version":state.version,"installed":state.installed,"repairable":state.repairable,"running":state.running,"tokenSaved":state.token_saved,"autoStart":state.auto_start,"hasIssue":state.issue.is_some()})
}

pub(super) fn token(manager: &Manager, input: Token, apply: bool) -> Result<Value> {
    ensure!(
        input.confirm,
        "Tünel jetonu değişikliği için açık onay gerekli."
    );
    if apply {
        manager.apply_tunnel(&input.token)?;
    } else {
        manager.save_tunnel_token(&input.token)?;
    }
    show(manager)
}

pub(super) fn clear(manager: &Manager, input: super::mysql::Confirm) -> Result<Value> {
    ensure!(
        input.confirm,
        "Tüneli durdurup jetonu silmek için açık onay gerekli."
    );
    manager.clear_tunnel_token()?;
    show(manager)
}

pub(super) fn auto_start(manager: &Manager, input: AutoStart) -> Result<Value> {
    ensure!(
        input.confirm,
        "Otomatik başlatma değişikliği için açık onay gerekli."
    );
    manager.save_tunnel_auto_start(input.auto_start)?;
    show(manager)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn tunnel_commands_preserve_confirmation_and_never_return_the_secret() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let secret = "TestOnly".repeat(12);
        for apply in [false, true] {
            assert!(token(
                &manager,
                Token {
                    token: secret.clone(),
                    confirm: false
                },
                apply
            )
            .is_err());
            assert!(token(
                &manager,
                Token {
                    token: "invalid".into(),
                    confirm: true
                },
                apply
            )
            .is_err());
        }
        assert_eq!(show(&manager).unwrap()["tokenSaved"], false);
        let result = token(
            &manager,
            Token {
                token: secret.clone(),
                confirm: true,
            },
            false,
        )
        .unwrap();
        assert_eq!(result["tokenSaved"], true);
        assert!(!result.to_string().contains(&secret));
        assert!(result.get("issue").is_none());
        let encrypted = std::fs::read(manager.tunnel_token_path()).unwrap();
        assert!(!String::from_utf8_lossy(&encrypted).contains(&secret));
        assert!(clear(&manager, super::super::mysql::Confirm { confirm: false }).is_err());
        assert!(auto_start(
            &manager,
            AutoStart {
                auto_start: true,
                confirm: false
            }
        )
        .is_err());
        assert_eq!(
            auto_start(
                &manager,
                AutoStart {
                    auto_start: true,
                    confirm: true
                }
            )
            .unwrap()["autoStart"],
            true
        );
        assert_eq!(
            clear(&manager, super::super::mysql::Confirm { confirm: true }).unwrap()["tokenSaved"],
            false
        );
        assert!(!manager.tunnel_token_path().exists());
    }
}
