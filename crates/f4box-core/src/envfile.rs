use crate::{storage, Manager};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const ENV_LIMIT: u64 = 256 * 1024;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnv {
    pub exists: bool,
    pub content: String,
    pub example: Option<String>,
}

pub fn validate_env_contents(raw: &str) -> Result<()> {
    if raw.len() as u64 > ENV_LIMIT {
        bail!(".env dosyası 256 KB sınırını aşıyor.");
    }
    if raw.contains('\0') {
        bail!(".env dosyası ikili veri içeremez.");
    }
    Ok(())
}

fn read_text(path: &Path) -> Result<String> {
    let bytes = storage::read_limited(path, ENV_LIMIT).with_context(|| {
        format!(
            ".env okunamadı veya 256 KB sınırını aşıyor: {}",
            path.display()
        )
    })?;
    String::from_utf8(bytes).context(".env UTF-8 olmalı.")
}

fn enclosed_env(root: &Path, path: &Path) -> Result<PathBuf> {
    let root = dunce::canonicalize(root).context("Proje klasörü bulunamadı.")?;
    if !path.exists() {
        return Ok(root.join(".env"));
    }
    let canon = dunce::canonicalize(path).context(".env yolu çözülemedi.")?;
    if !canon.starts_with(&root) || canon.file_name() != Some(".env".as_ref()) {
        bail!(".env yolu proje kökünün dışında.");
    }
    if !canon.is_file() {
        bail!(".env bir dosya olmalı.");
    }
    Ok(canon)
}

impl Manager {
    pub fn read_project_env(&self, id: &str) -> Result<ProjectEnv> {
        let project = self.project(id)?;
        let path = enclosed_env(&project.path, &project.path.join(".env"))?;
        let exists = path.is_file();
        let content = if exists {
            read_text(&path)?
        } else {
            String::new()
        };
        let example_path = project.path.join(".env.example");
        let example = example_path
            .is_file()
            .then(|| read_text(&example_path).ok())
            .flatten();
        Ok(ProjectEnv {
            exists,
            content,
            example,
        })
    }

    pub fn save_project_env(&self, id: &str, content: String) -> Result<()> {
        let _guard = self.gate()?;
        validate_env_contents(&content)?;
        let project = self.project(id)?;
        let path = enclosed_env(&project.path, &project.path.join(".env"))?;
        crate::storage::require_space(&project.path, content.len() as u64)?;
        storage::atomic_write(&path, content.as_bytes())?;
        self.log(format!(
            "Proje .env kaydedildi: {}. İçerik günlüğe yazılmaz.",
            project.name
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::validate_env_contents;

    #[test]
    fn rejects_nul_and_oversize_env() {
        assert!(validate_env_contents("APP_KEY=ok\n").is_ok());
        assert!(validate_env_contents("APP_KEY=bad\0value").is_err());
        assert!(validate_env_contents(&"x".repeat(256 * 1024 + 1)).is_err());
    }
}
