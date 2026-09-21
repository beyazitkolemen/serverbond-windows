//! Product identity constants (name, data directory, environment variables,
//! phpMyAdmin host) including the legacy F4Box values that old installs
//! still use.

use std::path::{Path, PathBuf};

pub const NAME: &str = "ServerBond";
pub const DATA_DIR: &str = "ServerBond";
pub const LEGACY_DATA_DIR: &str = "F4Box";
pub const HOME_ENV: &str = "SERVERBOND_HOME";
pub const LEGACY_HOME_ENV: &str = "F4BOX_HOME";
pub const PMA_HOST: &str = "phpmyadmin.serverbond.localhost";
pub const LEGACY_PMA_HOST: &str = "phpmyadmin.f4box.localhost";
pub const LOG_ID: &str = "serverbond";
pub const LEGACY_LOG_ID: &str = "f4box";
pub const CLI: &str = "serverbond";
pub const PERMISSIONS_TASK: &str = "ServerBond Permissions";
pub const PMA_TMP_ENV: &str = "SERVERBOND_PMA_TMP";
pub const PMA_SESSIONS_ENV: &str = "SERVERBOND_PMA_SESSIONS";

pub fn is_phpmyadmin_host(host: &str) -> bool {
    host == PMA_HOST || host == LEGACY_PMA_HOST
}

pub fn is_app_log(id: &str) -> bool {
    id == LOG_ID || id == LEGACY_LOG_ID
}

pub fn default_home() -> PathBuf {
    if let Some(value) = std::env::var_os(HOME_ENV).or_else(|| std::env::var_os(LEGACY_HOME_ENV)) {
        return PathBuf::from(value);
    }
    let base = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into()));
    data_home(&base)
}

fn data_home(base: &Path) -> PathBuf {
    let next = base.join(DATA_DIR);
    let legacy = base.join(LEGACY_DATA_DIR);
    // NSIS can create this directory before the first launch. An executable
    // alone must not hide existing F4Box data after the product rename.
    let has_state = [
        "config.json",
        "config.last-good.json",
        "config",
        "data",
        "bin",
        "www",
        "projects",
    ]
    .iter()
    .any(|entry| next.join(entry).exists());
    if has_state || !legacy.exists() {
        next
    } else {
        legacy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_directory_does_not_hide_legacy_data() {
        let base = tempfile::tempdir().unwrap();
        let next = base.path().join(DATA_DIR);
        let legacy = base.path().join(LEGACY_DATA_DIR);
        assert_eq!(data_home(base.path()), next);
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("config.json"), "legacy configuration").unwrap();
        std::fs::create_dir_all(&next).unwrap();
        std::fs::write(next.join("f4box-desktop.exe"), "installed executable").unwrap();
        assert_eq!(data_home(base.path()), legacy);
        assert_eq!(
            std::fs::read_to_string(legacy.join("config.json")).unwrap(),
            "legacy configuration"
        );
    }

    #[test]
    fn existing_serverbond_data_keeps_its_home_even_without_config() {
        let base = tempfile::tempdir().unwrap();
        let next = base.path().join(DATA_DIR);
        std::fs::create_dir_all(base.path().join(LEGACY_DATA_DIR)).unwrap();
        std::fs::create_dir_all(next.join("data/mysql")).unwrap();
        std::fs::write(next.join("data/mysql/ibdata1"), "existing database").unwrap();
        assert_eq!(data_home(base.path()), next);
        std::fs::write(next.join("config.json"), "broken config").unwrap();
        assert_eq!(data_home(base.path()), next);
    }

    #[test]
    fn phpmyadmin_accepts_legacy_host() {
        assert!(is_phpmyadmin_host(PMA_HOST));
        assert!(is_phpmyadmin_host(LEGACY_PMA_HOST));
        assert!(!is_phpmyadmin_host("phpmyadmin.localhost"));
    }
}
