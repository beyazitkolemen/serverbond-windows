//! ServerBond identity and data-directory selection.

use crate::legacy;
use std::path::{Path, PathBuf};

pub const NAME: &str = "ServerBond";
pub const DATA_DIR: &str = "ServerBond";
pub const WINDOWS_HOME: &str = r"C:\ServerBond";
pub const HOME_ENV: &str = "SERVERBOND_HOME";
pub const PMA_HOST: &str = "phpmyadmin.serverbond.localhost";
pub const LOG_ID: &str = "serverbond";
pub const CLI: &str = "serverbond";
pub const PERMISSIONS_TASK: &str = "ServerBond Permissions";
pub const PMA_TMP_ENV: &str = "SERVERBOND_PMA_TMP";
pub const PMA_SESSIONS_ENV: &str = "SERVERBOND_PMA_SESSIONS";

pub fn is_phpmyadmin_host(host: &str) -> bool {
    host == PMA_HOST || host == legacy::PMA_HOST
}

pub fn is_app_log(id: &str) -> bool {
    id == LOG_ID || id == legacy::LOG_ID
}

pub fn default_home() -> PathBuf {
    if let Some(value) = std::env::var_os(HOME_ENV).or_else(|| std::env::var_os(legacy::HOME_ENV)) {
        return PathBuf::from(value);
    }
    let base = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into()));
    let preferred = if cfg!(windows) {
        PathBuf::from(WINDOWS_HOME)
    } else {
        base.join(DATA_DIR)
    };
    data_home(&preferred, &base)
}

fn has_state(home: &Path) -> bool {
    // An installer-created directory containing only executables must not
    // hide an existing database or a configuration that needs recovery.
    [
        "config.json",
        "config.last-good.json",
        "config",
        "data",
        "bin",
        "www",
        "projects",
        "backups",
    ]
    .iter()
    .any(|entry| home.join(entry).exists())
}

fn data_home(preferred: &Path, base: &Path) -> PathBuf {
    let previous = base.join(DATA_DIR);
    let legacy = base.join(legacy::DATA_DIR);
    if has_state(preferred) {
        preferred.to_path_buf()
    } else if has_state(&previous) {
        previous
    } else if legacy.exists() {
        legacy
    } else {
        preferred.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_directory_does_not_hide_legacy_data() {
        let base = tempfile::tempdir().unwrap();
        let next = base.path().join(DATA_DIR);
        let legacy = base.path().join(legacy::DATA_DIR);
        assert_eq!(data_home(&next, base.path()), next);
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("config.json"), "legacy configuration").unwrap();
        std::fs::create_dir_all(&next).unwrap();
        std::fs::write(next.join("serverbond-desktop.exe"), "installed executable").unwrap();
        assert_eq!(data_home(&next, base.path()), legacy);
        assert_eq!(
            std::fs::read_to_string(legacy.join("config.json")).unwrap(),
            "legacy configuration"
        );
    }

    #[test]
    fn existing_serverbond_data_keeps_its_home_even_without_config() {
        let base = tempfile::tempdir().unwrap();
        let next = base.path().join(DATA_DIR);
        std::fs::create_dir_all(base.path().join(legacy::DATA_DIR)).unwrap();
        std::fs::create_dir_all(next.join("data/mysql")).unwrap();
        std::fs::write(next.join("data/mysql/ibdata1"), "existing database").unwrap();
        assert_eq!(data_home(&next, base.path()), next);
        std::fs::write(next.join("config.json"), "broken config").unwrap();
        assert_eq!(data_home(&next, base.path()), next);
    }

    #[test]
    fn fresh_install_uses_root_but_executable_does_not_hide_appdata() {
        let dir = tempfile::tempdir().unwrap();
        let preferred = dir.path().join("drive/ServerBond");
        let base = dir.path().join("AppData/Local");
        assert_eq!(data_home(&preferred, &base), preferred);
        std::fs::create_dir_all(&preferred).unwrap();
        std::fs::write(preferred.join("serverbond-desktop.exe"), "exe").unwrap();
        let previous = base.join(DATA_DIR);
        std::fs::create_dir_all(previous.join("data/mysql")).unwrap();
        std::fs::write(previous.join("data/mysql/ibdata1"), "database").unwrap();
        assert_eq!(data_home(&preferred, &base), previous);
        std::fs::write(previous.join("config.json"), "broken").unwrap();
        assert_eq!(data_home(&preferred, &base), previous);
        assert_eq!(
            std::fs::read_to_string(previous.join("data/mysql/ibdata1")).unwrap(),
            "database"
        );
    }

    #[test]
    fn root_state_wins_and_backup_only_state_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let preferred = dir.path().join("drive/ServerBond");
        let base = dir.path().join("AppData/Local");
        std::fs::create_dir_all(base.join(DATA_DIR).join("config")).unwrap();
        std::fs::create_dir_all(preferred.join("backups")).unwrap();
        assert_eq!(data_home(&preferred, &base), preferred);
    }

    #[test]
    fn root_install_preserves_old_product_data() {
        let dir = tempfile::tempdir().unwrap();
        let preferred = dir.path().join("drive/ServerBond");
        let base = dir.path().join("AppData/Local");
        let legacy = base.join(legacy::DATA_DIR);
        std::fs::create_dir_all(legacy.join("www")).unwrap();
        assert_eq!(data_home(&preferred, &base), legacy);
    }

    #[test]
    fn phpmyadmin_accepts_legacy_host() {
        assert!(is_phpmyadmin_host(PMA_HOST));
        assert!(is_phpmyadmin_host(legacy::PMA_HOST));
        assert!(!is_phpmyadmin_host("phpmyadmin.localhost"));
    }
}
