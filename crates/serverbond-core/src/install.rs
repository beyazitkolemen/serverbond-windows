//! Package download, SHA-256 verification, archive extraction and health
//! checks for every catalogued component, including the repair path that
//! replaces program files while keeping data folders.

use crate::domain::ComponentId;
use crate::model::Package;
use crate::repository::DataDir;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallPhase {
    Preparing,
    Downloading,
    Verifying,
    Extracting,
    Installing,
    Permissions,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub phase: InstallPhase,
    pub completed: u64,
    pub total: Option<u64>,
}

pub(crate) struct ProgressGuard<'a>(&'a std::sync::Mutex<Option<InstallProgress>>);

impl Drop for ProgressGuard<'_> {
    fn drop(&mut self) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

impl crate::Manager {
    #[cfg(windows)]
    pub(crate) fn permission_progress(&self) -> ProgressGuard<'_> {
        *self
            .install_progress
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(InstallProgress {
            package_id: "permissions".into(),
            name: "Windows".into(),
            version: String::new(),
            phase: InstallPhase::Permissions,
            completed: 0,
            total: None,
        });
        ProgressGuard(&self.install_progress)
    }

    /// All entry points share live state, including API requests and launch tools.
    /// The caller holds the operation gate; errors and unwinds clear the state.
    pub(crate) fn install_package(&self, package: &Package, repair: bool) -> Result<()> {
        let _progress = ProgressGuard(&self.install_progress);
        install_with_progress(
            &self.home,
            package,
            repair,
            |line| self.log(line),
            |phase, completed, total| {
                *self
                    .install_progress
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(InstallProgress {
                    package_id: package.id.clone(),
                    name: package.name.clone(),
                    version: package.version.clone(),
                    phase,
                    completed,
                    total,
                });
            },
        )
    }
}

// Report real byte/file counts, throttled independently of the text log.
struct Reporter<F: FnMut(InstallPhase, u64, Option<u64>)> {
    callback: F,
    phase: InstallPhase,
    last: Instant,
}

impl<F: FnMut(InstallPhase, u64, Option<u64>)> Reporter<F> {
    fn report(&mut self, phase: InstallPhase, completed: u64, total: Option<u64>) {
        if phase != self.phase
            || completed == 0
            || total == Some(completed)
            || self.last.elapsed() >= Duration::from_millis(150)
        {
            (self.callback)(phase, completed, total);
            self.phase = phase;
            self.last = Instant::now();
        }
    }
}

pub(crate) const DOWNLOAD_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const DOWNLOAD_TOTAL_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub(crate) fn download_user_agent() -> String {
    format!("{}/{}", crate::product::NAME, env!("CARGO_PKG_VERSION"))
}

pub(crate) fn download_client() -> Result<reqwest::blocking::Client> {
    // reqwest's blocking client has no per-read timeout. A 60s total timeout
    // aborted MySQL (~200 MB) on ordinary connections; bound the whole transfer
    // instead and keep a short connect timeout.
    reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(DOWNLOAD_CONNECT_TIMEOUT)
        .timeout(DOWNLOAD_TOTAL_TIMEOUT)
        .user_agent(download_user_agent())
        .build()
        .context("İndirme istemcisi oluşturulamadı.")
}

pub fn verify_hash(path: &Path, expected: &str) -> Result<()> {
    verify_hash_progress(path, expected, |_, _| {})
}

fn verify_hash_progress(
    path: &Path,
    expected: &str,
    mut progress: impl FnMut(u64, u64),
) -> Result<()> {
    let mut file = File::open(path)?;
    let total = file.metadata()?.len();
    let mut completed = 0;
    progress(0, total);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        completed += n as u64;
        progress(completed, total);
    }
    if format!("{:x}", hasher.finalize()) != expected.to_ascii_lowercase() {
        bail!("SHA-256 doğrulaması başarısız. Paket kurulmadı; yeniden indirin.");
    }
    Ok(())
}

pub fn extract_zip(path: &Path, destination: &Path, prefix: &str) -> Result<()> {
    extract_zip_progress(path, destination, prefix, |_, _| {})
}

fn extract_zip_progress(
    path: &Path,
    destination: &Path,
    prefix: &str,
    mut progress: impl FnMut(u64, u64),
) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(path)?)?;
    let files = archive.len() as u64;
    progress(0, files);
    if archive.len() > 100_000 {
        bail!("Arşiv dosya sayısı sınırını aşıyor.");
    }
    let mut required = 0u64;
    for i in 0..archive.len() {
        required = required.saturating_add(archive.by_index(i)?.size());
        if required > 4 * 1024 * 1024 * 1024 {
            bail!("Arşiv boyutu sınırı aşıldı.");
        }
    }
    fs::create_dir_all(destination)?;
    crate::storage::require_space(destination, required)?;
    let mut expanded = 0u64;
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let entry_path = entry
            .enclosed_name()
            .context("Arşivde güvenli olmayan yol bulundu.")?;
        if entry.name().contains(':')
            || entry.name().contains('\\')
            || entry_path
                .components()
                .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            bail!("Arşivde geçersiz yol veya bağlantı bulundu.");
        }
        expanded = expanded.saturating_add(entry.size());
        if expanded > 4 * 1024 * 1024 * 1024 {
            bail!("Arşiv boyutu sınırı aşıldı.");
        }
        let relative = if prefix.is_empty() {
            entry_path.as_path()
        } else {
            entry_path
                .strip_prefix(prefix)
                .context("Arşiv klasör yapısı beklenenden farklı.")?
        };
        if relative.as_os_str().is_empty() {
            progress(i as u64 + 1, files);
            continue;
        }
        let out = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(out)?;
        } else {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            let size = entry.size();
            let copied = std::io::copy(&mut entry.take(size + 1), &mut File::create(out)?)?;
            if copied != size {
                bail!("Arşiv dosyasının gerçek boyutu kayıtla eşleşmiyor.");
            }
        }
        progress(i as u64 + 1, files);
    }
    Ok(())
}

pub fn install(home: &Path, package: &Package, log: impl Fn(String)) -> Result<()> {
    install_with_progress(home, package, false, log, |_, _, _| {})
}

pub fn repair(home: &Path, package: &Package, log: impl Fn(String)) -> Result<()> {
    install_with_progress(home, package, true, log, |_, _, _| {})
}

pub fn required_files(package: &Package) -> Vec<&str> {
    match package.id.parse::<ComponentId>() {
        Ok(ComponentId::PhpMyAdmin) => vec![
            "index.php",
            "vendor/autoload.php",
            "libraries/classes/DatabaseInterface.php",
            "templates/login/form.twig",
            "js/dist/common.js",
        ],
        Ok(ComponentId::Php) => vec!["php.exe", "php-cgi.exe", "php8.dll"]
            .into_iter()
            .filter(|name| *name != "php8.dll" || !package.version.starts_with("7."))
            .chain(package.version.starts_with("7.").then_some("php7.dll"))
            .collect(),
        Ok(ComponentId::Mysql) => vec![
            "bin/mysqld.exe",
            "bin/mysql.exe",
            "bin/mysqladmin.exe",
            "bin/mysqldump.exe",
        ],
        Ok(ComponentId::Postgres) => vec![
            "bin/postgres.exe",
            "bin/initdb.exe",
            "bin/pg_ctl.exe",
            "bin/psql.exe",
            "bin/pg_isready.exe",
        ],
        Ok(ComponentId::Redis) => vec!["redis-server.exe", "redis-cli.exe", "msys-2.0.dll"],
        _ => vec![&package.executable],
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallHealth {
    pub installed: bool,
    pub repairable: bool,
    pub issue: Option<String>,
}

pub fn health(home: &Path, package: &Package) -> InstallHealth {
    let directory = DataDir::new(home).package(&package.id, &package.version);
    match validate_installation(&directory, package) {
        Ok(()) => InstallHealth {
            installed: true,
            repairable: true,
            issue: None,
        },
        Err(error) => InstallHealth {
            installed: false,
            repairable: directory.exists(),
            issue: directory.exists().then(|| format!("{error:#}")),
        },
    }
}

pub fn validate_installation(directory: &Path, package: &Package) -> Result<()> {
    let receipt: Package = serde_json::from_slice(
        &crate::storage::read_limited(&directory.join("installed.json"), 256 * 1024)
            .context("Kurulum kaydı eksik; Onar düğmesini kullanın.")?,
    )
    .context("Kurulum kaydı okunamıyor; Onar düğmesini kullanın.")?;
    if receipt.id != package.id
        || receipt.version != package.version
        || receipt.sha256 != package.sha256
    {
        bail!("Kurulum kaydı seçilen paketle eşleşmiyor; Onar düğmesini kullanın.");
    }
    for name in required_files(package) {
        if !directory
            .join(name)
            .metadata()
            .is_ok_and(|m| m.is_file() && m.len() > 0)
        {
            bail!("{name} eksik veya boş; Onar düğmesini kullanın.");
        }
    }
    Ok(())
}

pub fn archive_fallback(package: &Package) -> Option<String> {
    if package.id != "php" {
        return None;
    }
    for base in [
        "https://downloads.php.net/~windows/releases/",
        "https://windows.php.net/downloads/releases/",
    ] {
        if let Some(file) = package
            .url
            .strip_prefix(base)
            .filter(|file| !file.contains('/') && file.ends_with(".zip"))
        {
            return Some(format!(
                "https://downloads.php.net/~windows/releases/archives/{file}"
            ));
        }
    }
    None
}

pub(crate) fn install_with_progress(
    home: &Path,
    package: &Package,
    repair: bool,
    log: impl Fn(String),
    progress: impl FnMut(InstallPhase, u64, Option<u64>),
) -> Result<()> {
    let mut reporter = Reporter {
        callback: progress,
        phase: InstallPhase::Preparing,
        last: Instant::now(),
    };
    reporter.report(InstallPhase::Preparing, 0, None);
    let destination = DataDir::new(home).package(&package.id, &package.version);
    if !repair && validate_installation(&destination, package).is_ok() {
        log(format!(
            "{} {} zaten kurulu.",
            package.name, package.version
        ));
        return Ok(());
    }
    if !repair && destination.exists() {
        bail!(
            "Eksik kurulum klasörü bulundu: {}. Sunucuyu durdurup Bileşenler ekranındaki Onar düğmesini kullanın.",
            destination.display()
        );
    }
    let cache = home.join("cache").join(format!(
        "{}-{}.{}",
        package.id,
        package.version,
        if package.archive {
            "zip"
        } else {
            Path::new(&package.executable)
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("bin")
        }
    ));
    if cache.exists()
        && verify_hash_progress(&cache, &package.sha256, |done, total| {
            reporter.report(InstallPhase::Verifying, done, Some(total));
        })
        .is_err()
    {
        fs::remove_file(&cache)?;
    }
    if !cache.exists() {
        log(format!("{} {} indiriliyor…", package.name, package.version));
        reporter.report(InstallPhase::Downloading, 0, None);
        let client = download_client()?;
        let mut response = client.get(&package.url).send()?;
        if matches!(response.status().as_u16(), 404 | 410) {
            if let Some(archive) = archive_fallback(package) {
                log(format!("{} resmî PHP arşivinde aranıyor…", package.version));
                response = client.get(archive).send()?;
            }
        }
        let mut response = response.error_for_status()?;
        let total = response.content_length().unwrap_or(0);
        reporter.report(InstallPhase::Downloading, 0, (total > 0).then_some(total));
        if total > 1024 * 1024 * 1024 {
            bail!("Paket boyutu sınırı aşıldı.");
        }
        crate::storage::require_space(
            &home.join("cache"),
            if total == 0 {
                1024 * 1024 * 1024
            } else {
                total
            },
        )?;
        let mut partial = tempfile::NamedTempFile::new_in(home.join("cache"))?;
        let mut buffer = [0u8; 128 * 1024];
        let (mut received, mut last_percent) = (0u64, 0u64);
        loop {
            let count = response.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            received += count as u64;
            if received > 1024 * 1024 * 1024 {
                bail!("Paket boyutu sınırı aşıldı.");
            }
            partial.write_all(&buffer[..count])?;
            reporter.report(
                InstallPhase::Downloading,
                received,
                (total > 0).then_some(total),
            );
            let percent = (received * 100).checked_div(total).unwrap_or(0);
            if percent >= last_percent + 10 {
                log(format!("{} indiriliyor: %{percent}", package.name));
                last_percent = percent;
            }
        }
        partial.as_file().sync_all()?;
        verify_hash_progress(partial.path(), &package.sha256, |done, total| {
            reporter.report(InstallPhase::Verifying, done, Some(total));
        })?;
        partial.persist(&cache)?;
    }
    verify_hash_progress(&cache, &package.sha256, |done, total| {
        reporter.report(InstallPhase::Verifying, done, Some(total));
    })?;
    log(format!(
        "{} SHA-256 doğrulandı. Dosyalar hazırlanıyor…",
        package.name
    ));
    fs::create_dir_all(destination.parent().unwrap())?;
    let stage = tempfile::tempdir_in(destination.parent().unwrap())?;
    if package.archive {
        extract_zip_progress(&cache, stage.path(), &package.prefix, |done, total| {
            reporter.report(InstallPhase::Extracting, done, Some(total));
        })?;
    } else {
        reporter.report(InstallPhase::Installing, 0, None);
        fs::copy(&cache, stage.path().join(&package.executable))?;
    }
    reporter.report(InstallPhase::Installing, 0, None);
    fs::write(
        stage.path().join("installed.json"),
        serde_json::to_vec_pretty(package)?,
    )?;
    validate_installation(stage.path(), package)?;
    let backup = destination.with_file_name(format!(
        "{}-before-repair-{}",
        package.version,
        uuid::Uuid::new_v4()
    ));
    let replaced = destination.exists();
    if replaced {
        fs::rename(&destination, &backup)
            .context("Paket klasörü kullanılıyor; çalışan sunucuyu durdurun.")?;
    }
    if let Err(error) = fs::rename(stage.path(), &destination) {
        if replaced {
            fs::rename(&backup, &destination)
                .with_context(|| format!("Eski paket {} konumunda korunuyor", backup.display()))?;
        }
        return Err(error.into());
    }
    if replaced {
        // Retain the previous directory: repairs never discard user-added files.
        log(format!(
            "Önceki program dosyaları korundu: {}",
            backup.display()
        ));
    }
    log(format!("{} {} kuruldu.", package.name, package.version));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_install_reports_verification_extraction_and_commit() {
        let home = tempfile::tempdir().unwrap();
        fs::create_dir(home.path().join("cache")).unwrap();
        let archive_path = home.path().join("cache/fixture-1.zip");
        let mut archive = zip::ZipWriter::new(File::create(&archive_path).unwrap());
        archive
            .start_file("app.exe", zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"test executable").unwrap();
        archive
            .start_file("assets/data.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"test asset").unwrap();
        archive.finish().unwrap();
        let mut package = crate::model::catalog().remove(0);
        package.id = "fixture".into();
        package.version = "1".into();
        package.executable = "app.exe".into();
        package.archive = true;
        package.prefix.clear();
        package.sha256 = format!("{:x}", Sha256::digest(fs::read(&archive_path).unwrap()));
        let mut events = Vec::new();
        install_with_progress(
            home.path(),
            &package,
            false,
            |_| {},
            |phase, done, total| events.push((phase, done, total)),
        )
        .unwrap();
        assert_eq!(events.first().unwrap().0, InstallPhase::Preparing);
        assert!(events
            .iter()
            .any(|(phase, done, total)| *phase == InstallPhase::Verifying
                && *done > 0
                && Some(*done) == *total));
        assert!(events.contains(&(InstallPhase::Extracting, 2, Some(2))));
        assert_eq!(events.last().unwrap().0, InstallPhase::Installing);
        assert!(!events
            .iter()
            .any(|(phase, _, _)| *phase == InstallPhase::Downloading));
        assert!(health(home.path(), &package).installed);
    }

    #[test]
    fn progress_guard_clears_state_after_failure_and_unwind() {
        let home = tempfile::tempdir().unwrap();
        let manager = crate::Manager::new(home.path().into()).unwrap();
        let package = crate::model::catalog().remove(0);
        fs::create_dir_all(DataDir::new(home.path()).package(&package.id, &package.version))
            .unwrap();
        assert!(manager.install_package(&package, false).is_err());
        assert!(manager.snapshot().unwrap().install_progress.is_none());
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = ProgressGuard(&manager.install_progress);
            *manager.install_progress.lock().unwrap() = Some(InstallProgress {
                package_id: package.id.clone(),
                name: package.name.clone(),
                version: package.version.clone(),
                phase: InstallPhase::Downloading,
                completed: 12,
                total: None,
            });
            assert_eq!(
                manager
                    .snapshot()
                    .unwrap()
                    .install_progress
                    .unwrap()
                    .completed,
                12
            );
            panic!("interrupted installation");
        }));
        assert!(manager.snapshot().unwrap().install_progress.is_none());
    }

    #[test]
    fn health_marks_missing_and_incomplete_installs() {
        let home = tempfile::tempdir().unwrap();
        let mut package = crate::model::catalog().remove(0);
        package.id = "fixture".into();
        package.version = "1".into();
        package.executable = "app.exe".into();
        let absent = health(home.path(), &package);
        assert!(!absent.installed);
        assert!(!absent.repairable);
        assert!(absent.issue.is_none());
        let dir = home.path().join("bin/fixture/1");
        std::fs::create_dir_all(&dir).unwrap();
        let broken = health(home.path(), &package);
        assert!(!broken.installed);
        assert!(broken.repairable);
        assert!(broken.issue.as_ref().unwrap().contains("Onar"));
    }

    #[test]
    fn download_client_uses_current_version_and_long_transfer_timeout() {
        assert_eq!(DOWNLOAD_CONNECT_TIMEOUT, Duration::from_secs(30));
        assert_eq!(DOWNLOAD_TOTAL_TIMEOUT, Duration::from_secs(30 * 60));
        assert_eq!(
            download_user_agent(),
            format!("{}/{}", crate::product::NAME, env!("CARGO_PKG_VERSION"))
        );
        assert!(download_user_agent().starts_with(&format!("{}/1.", crate::product::NAME)));
        download_client().unwrap();
    }
}
