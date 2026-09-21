//! Small file primitives shared by every module: bounded reads, atomic
//! writes through a sibling temp file, and free-space checks before large
//! downloads or backups.

use anyhow::{bail, Context, Result};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

pub(crate) fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("Dosya okunamadı: {}", path.display()))?;
    if file.metadata()?.len() > limit {
        bail!("{} dosyası {} bayt sınırını aşıyor.", path.display(), limit);
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("Dosya okuma sırasında boyut sınırını aştı.");
    }
    Ok(bytes)
}

// Stage on the same volume, flush, then replace. Failed writes preserve the old file.
pub(crate) fn atomic_write(path: &Path, bytes: impl AsRef<[u8]>) -> Result<()> {
    let parent = path.parent().context("Dosyanın üst klasörü bulunamadı.")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes.as_ref())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).with_context(|| {
        format!(
            "Dosya kaydedilemedi; önceki dosya korundu: {}",
            path.display()
        )
    })?;
    Ok(())
}

pub(crate) fn require_space(path: &Path, bytes: u64) -> Result<()> {
    let available = fs2::available_space(path).context("Boş disk alanı denetlenemedi.")?;
    if available < bytes.saturating_add(64 * 1024 * 1024) {
        bail!("Yeterli disk alanı yok. İşlem ve 64 MB güvenlik payı için {} MB gerekli, {} MB kullanılabilir.", bytes / 1024 / 1024 + 64, available / 1024 / 1024);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_oversized_files_without_loading_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.json");
        File::create(&path)
            .unwrap()
            .set_len(64 * 1024 * 1024)
            .unwrap();
        assert!(read_limited(&path, 1024).is_err());
    }
    #[test]
    fn failed_replace_preserves_existing_content_and_cleans_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("keep"), b"original").unwrap();
        assert!(atomic_write(&path, b"replacement").is_err());
        assert_eq!(std::fs::read(path.join("keep")).unwrap(), b"original");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[test]
    fn refuses_impossible_disk_requirement() {
        let dir = tempfile::tempdir().unwrap();
        assert!(require_space(dir.path(), u64::MAX).is_err());
    }
}
