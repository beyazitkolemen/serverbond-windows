//! Read and write a project's root `.env` (and read `.env.example`) with
//! path enclosure, size and encoding checks; nothing is written unless the
//! user saves.

use crate::{storage, Manager};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const ENV_LIMIT: u64 = 256 * 1024;

#[derive(Debug)]
pub(crate) struct EnvConflict;
impl std::fmt::Display for EnvConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(".env değişti. Güncel dosyayı alın; değişiklikleriniz kaydedilmedi.")
    }
}
impl std::error::Error for EnvConflict {}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnv {
    pub exists: bool,
    pub content: String,
    pub example: Option<String>,
}

pub(crate) fn env_revision(env: &ProjectEnv) -> String {
    use sha2::{Digest, Sha256};
    let mut digest = Sha256::new();
    digest.update([u8::from(env.exists)]);
    digest.update(env.content.as_bytes());
    format!("{:x}", digest.finalize())
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
    let content = String::from_utf8(bytes).context(".env UTF-8 olmalı.")?;
    validate_env_contents(&content)?;
    Ok(content)
}

fn enclosed_env(root: &Path, path: &Path) -> Result<PathBuf> {
    let root = dunce::canonicalize(root).context("Proje klasörü bulunamadı.")?;
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        let linked = metadata.file_type().is_symlink();
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            linked || metadata.file_attributes() & 0x400 != 0
        };
        if linked {
            bail!(".env bağlantı veya reparse noktası olamaz.");
        }
    }
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
        // Same enclosure rule as .env: a symlinked example must not read outside the project.
        let example_path = project.path.join(".env.example");
        let example = dunce::canonicalize(&example_path)
            .ok()
            .filter(|canon| {
                canon.is_file()
                    && dunce::canonicalize(&project.path).is_ok_and(|root| canon.starts_with(root))
            })
            .and_then(|canon| read_text(&canon).ok());
        Ok(ProjectEnv {
            exists,
            content,
            example,
        })
    }

    pub fn save_project_env(&self, id: &str, content: String) -> Result<()> {
        self.save_project_env_checked(id, content, None).map(|_| ())
    }

    pub(crate) fn save_project_env_checked(
        &self,
        id: &str,
        content: String,
        expected: Option<&str>,
    ) -> Result<ProjectEnv> {
        let _guard = self.gate()?;
        validate_env_contents(&content)?;
        if let Some(expected) = expected {
            anyhow::ensure!(
                env_revision(&self.read_project_env(id)?) == expected,
                EnvConflict
            );
        }
        let project = self.project(id)?;
        let path = enclosed_env(&project.path, &project.path.join(".env"))?;
        crate::storage::require_space(&project.path, content.len() as u64)?;
        storage::atomic_write(&path, content.as_bytes())?;
        self.log(format!(
            "Proje .env kaydedildi: {}. İçerik günlüğe yazılmaz.",
            project.name
        ));
        self.read_project_env(id)
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
