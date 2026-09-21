use crate::{model::Settings, Manager};
use anyhow::{ensure, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Save {
    settings: Settings,
    expected_revision: String,
    confirm: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Validate {
    settings: Settings,
}

pub(super) fn show(manager: &Manager) -> Result<Value> {
    let (settings, revision) = manager.settings_with_revision()?;
    Ok(json!({"settings":settings,"revision":revision,"versions":versions()}))
}

pub(super) fn save(manager: &Manager, input: Save) -> Result<Value> {
    ensure!(input.confirm, "Ayar kaydı için açık onay gerekli.");
    let (settings, revision) =
        manager.save_settings_checked(input.settings, &input.expected_revision)?;
    Ok(json!({"settings":settings,"revision":revision,"versions":versions()}))
}

fn versions() -> Vec<String> {
    crate::model::php_versions()
        .into_iter()
        .map(|package| package.version)
        .collect()
}

pub(super) fn validate(input: Validate) -> Result<Value> {
    input.settings.validate()?;
    Ok(json!({"valid":true}))
}

pub(super) fn defaults(manager: &Manager) -> Value {
    json!({"settings":manager.default_settings()})
}

pub(super) fn previous(manager: &Manager) -> Result<Value> {
    Ok(json!({"settings":manager.previous_settings()?}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_unicode_profiles_validate_without_lifting_other_operation_limits() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let mut settings = Settings::default();
        settings.php.extra_ini = format!(";{}", "ş".repeat(7000));
        for version in crate::model::php_versions().into_iter().take(3) {
            settings
                .php_versions
                .insert(version.version, settings.php.clone());
        }
        let parameters = json!({"settings":settings});
        assert!(serde_json::to_vec(&parameters).unwrap().len() > 32768);
        let before = manager.export_settings().unwrap();
        let result = super::super::operations::Operation::parse("settings.validate", &parameters)
            .unwrap()
            .execute(&manager)
            .unwrap();
        assert_eq!(result, json!({"valid":true}));
        assert_eq!(before, manager.export_settings().unwrap());
        let mut oversized = parameters;
        oversized["settings"]["php"]["extraIni"] = "a".repeat(256 * 1024).into();
        let error = super::super::operations::Operation::parse("settings.validate", &oversized)
            .err()
            .unwrap();
        assert!(error.to_string().contains("çok büyük"));
        let error = super::super::operations::Operation::parse(
            "projects.add",
            &json!({"name":"test","path":"a".repeat(32768)}),
        )
        .err()
        .unwrap();
        assert!(error.to_string().contains("çok büyük"));
    }

    #[test]
    fn settings_transport_requires_confirmation_and_does_not_mutate_on_reads() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let before = std::fs::read(home.path().join("config.json")).unwrap();
        let current = show(&manager).unwrap();
        assert_eq!(current["revision"].as_str().unwrap().len(), 64);
        assert!(defaults(&manager)["settings"].is_object());
        assert!(previous(&manager).is_err());
        let settings: Settings = serde_json::from_value(current["settings"].clone()).unwrap();
        assert_eq!(
            validate(Validate {
                settings: settings.clone()
            })
            .unwrap(),
            json!({"valid":true})
        );
        assert!(save(
            &manager,
            Save {
                settings: settings.clone(),
                expected_revision: current["revision"].as_str().unwrap().into(),
                confirm: false
            }
        )
        .is_err());
        let error = save(
            &manager,
            Save {
                settings,
                expected_revision: "0".repeat(64),
                confirm: true,
            },
        )
        .unwrap_err();
        assert!(error.is::<crate::preferences::SettingsConflict>());
        assert_eq!(
            before,
            std::fs::read(home.path().join("config.json")).unwrap()
        );
        assert!(
            serde_json::from_value::<Validate>(json!({"settings":{"password":"unexpected"}}))
                .is_err()
        );
    }
}
