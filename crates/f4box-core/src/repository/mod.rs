use crate::domain::ComponentId;
use std::path::{Path, PathBuf};

/// On-disk layout under the data directory. Persistence stays in `storage`.
pub struct DataDir<'a> {
    home: &'a Path,
}

impl<'a> DataDir<'a> {
    pub fn new(home: &'a Path) -> Self {
        Self { home }
    }

    pub fn home(&self) -> &'a Path {
        self.home
    }

    pub fn config(&self) -> PathBuf {
        self.home.join("config.json")
    }

    pub fn last_good(&self) -> PathBuf {
        self.home.join("config.last-good.json")
    }

    pub fn package(&self, id: &str, version: &str) -> PathBuf {
        self.home.join("bin").join(id).join(version)
    }

    pub fn component(&self, id: ComponentId, version: &str) -> PathBuf {
        self.package(id.as_str(), version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn layout_keeps_binaries_under_home() {
        let home = Path::new("home");
        let dir = DataDir::new(home);
        assert_eq!(dir.config(), home.join("config.json"));
        assert_eq!(
            dir.component(ComponentId::Mailpit, "1.31.2"),
            home.join("bin").join("mailpit").join("1.31.2")
        );
        assert_eq!(
            dir.package("mailpit", "1.31.2"),
            dir.component(ComponentId::Mailpit, "1.31.2")
        );
    }
}
