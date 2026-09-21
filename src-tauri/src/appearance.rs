use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Emitter, Manager as _};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

pub struct Appearance {
    path: PathBuf,
    lock: Mutex<()>,
}

impl Appearance {
    pub fn new(home: &Path) -> Self {
        Self {
            path: home.join("config/appearance.json"),
            lock: Mutex::new(()),
        }
    }
    fn read(&self) -> Result<Option<Theme>> {
        if !self.path.try_exists()? {
            return Ok(None);
        }
        let mut bytes = Vec::new();
        std::fs::File::open(&self.path)?
            .take(4097)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 4096 {
            bail!("Görünüm dosyası 4 KB sınırını aşıyor.");
        }
        Ok(Some(serde_json::from_slice(&bytes)?))
    }
    pub fn get(&self) -> Result<Option<Theme>> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        self.read()
    }
    pub fn save(&self, theme: Theme, initialize_only: bool) -> Result<Theme> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        if initialize_only {
            if let Some(current) = self.read()? {
                return Ok(current);
            }
        }
        let parent = self.path.parent().expect("appearance path has parent");
        std::fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(&serde_json::to_vec(&theme)?)?;
        file.as_file().sync_all()?;
        file.persist(&self.path)?;
        Ok(theme)
    }
}

pub fn save(app: &AppHandle, theme: Theme, initialize_only: bool) -> Result<Theme> {
    let theme = app.state::<Appearance>().save(theme, initialize_only)?;
    app.emit("desktop:theme", theme)?;
    Ok(theme)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_does_not_overwrite_an_api_preference() {
        let home = tempfile::tempdir().unwrap();
        let appearance = Appearance::new(home.path());
        assert_eq!(appearance.get().unwrap(), None);
        appearance.save(Theme::Dark, false).unwrap();
        assert_eq!(appearance.save(Theme::Light, true).unwrap(), Theme::Dark);
        assert_eq!(
            Appearance::new(home.path()).get().unwrap(),
            Some(Theme::Dark)
        );
    }
    #[test]
    fn invalid_or_oversized_appearance_is_reported_and_preserved() {
        let home = tempfile::tempdir().unwrap();
        let appearance = Appearance::new(home.path());
        appearance.save(Theme::Light, false).unwrap();
        std::fs::write(&appearance.path, "broken").unwrap();
        assert!(appearance.save(Theme::System, true).is_err());
        assert_eq!(std::fs::read_to_string(&appearance.path).unwrap(), "broken");
        std::fs::write(&appearance.path, vec![b' '; 4097]).unwrap();
        assert!(appearance.get().is_err());
    }
}
