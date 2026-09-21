//! Product identity constants (name, data directory, environment variables,
//! phpMyAdmin host) including the legacy F4Box values that old installs
//! still use.

use std::path::PathBuf;

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
    let next = base.join(DATA_DIR);
    let legacy = base.join(LEGACY_DATA_DIR);
    if next.exists() || !legacy.exists() {
        next
    } else {
        legacy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phpmyadmin_accepts_legacy_host() {
        assert!(is_phpmyadmin_host(PMA_HOST));
        assert!(is_phpmyadmin_host(LEGACY_PMA_HOST));
        assert!(!is_phpmyadmin_host("phpmyadmin.localhost"));
    }
}
