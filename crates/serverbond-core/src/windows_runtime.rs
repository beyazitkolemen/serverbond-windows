//! Verified Microsoft prerequisite installation. No system PATH changes and no
//! automatic reboot; a failed/declined launch attempt needs an explicit retry.
use crate::{model::Package, process::ManagedChild, repository::DataDir, Manager};
use anyhow::{bail, Context, Result};
use std::{path::Path, time::Duration};

const VC_HASH: &str = "cc0ff0eb1dc3f5188ae6300faef32bf5beeba4bdd6e8e445a9184072096b713b";

fn runtime_package() -> Package {
    Package {
        id: "vc-runtime".into(),
        name: "Microsoft Visual C++ x64 Redistributable".into(),
        version: "14.44.35211.0".into(),
        description: "Windows x64 çalışma zamanı".into(),
        url: "https://download.visualstudio.microsoft.com/download/pr/bd1c8d9d-ba95-4eee-bc6e-df1fcc876373/CC0FF0EB1DC3F5188AE6300FAEF32BF5BEEBA4BDD6E8E445A9184072096B713B/VC_redist.x64.exe".into(),
        sha256: VC_HASH.into(),
        archive: false,
        prefix: String::new(),
        executable: "vc_redist.x64.exe".into(),
        license: "Microsoft Software License Terms".into(),
        source: "https://learn.microsoft.com/cpp/windows/latest-supported-vc-redist".into(),
    }
}

fn installer_script(path: &Path) -> String {
    let path = crate::terminal::literal(&path.to_string_lossy());
    format!(
        "$ErrorActionPreference='Stop';\n\
         $installer={path};\n\
         if ((Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash -ne '{VC_HASH}') {{ throw 'Visual C++ SHA-256 doğrulaması başarısız.' }}\n\
         $signature=Get-AuthenticodeSignature -LiteralPath $installer;\n\
         if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '(^|,\\s*)CN=Microsoft Corporation(,|$)') {{ throw 'Geçerli Microsoft imzası bulunamadı.' }}\n\
         $process=Start-Process -FilePath $installer -ArgumentList '/install','/quiet','/norestart' -Verb RunAs -WindowStyle Hidden -Wait -PassThru;\n\
         Write-Output ('SERVERBOND_VC_EXIT=' + $process.ExitCode);\n"
    )
}

fn installer_exit(stdout: &[u8]) -> Result<i32> {
    String::from_utf8_lossy(stdout)
        .lines()
        .find_map(|line| line.trim().strip_prefix("SERVERBOND_VC_EXIT="))
        .and_then(|value| value.parse().ok())
        .context("Visual C++ kurulum sonucu alınamadı.")
}

fn check_installer_result(code: i32, ready: bool) -> Result<bool> {
    match code {
        0 | 1638 if ready => Ok(false),
        3010 if ready => Ok(true),
        3010 => bail!("Visual C++ kurulumu tamamlandı; Windows yeniden başlatılmalı. Çalışma zamanı henüz kullanılamıyor; yeniden başlatmanın ardından tekrar deneyin."),
        0 | 1638 => bail!("Visual C++ yükleyicisi tamamlandı ancak gerekli x64 DLL dosyaları yüklenemiyor. Gereksinimler ekranından tekrar deneyin."),
        _ => bail!("Visual C++ kurulumu başarısız (kod {code}). Gereksinimler ekranından tekrar deneyin."),
    }
}

fn may_attempt(ready: bool, previous_attempt: bool, explicit_retry: bool) -> Result<bool> {
    if ready {
        return Ok(false);
    }
    if previous_attempt && !explicit_retry {
        bail!("Önceki Visual C++ kurulumu tamamlanamadı veya yönetici izni verilmedi. Gereksinimler ekranındaki Otomatik kur düğmesiyle tekrar deneyin.");
    }
    Ok(true)
}

fn package_needs_repair(directory: &Path, package: &Package) -> bool {
    directory.exists()
        && (crate::install::validate_installation(directory, package).is_err()
            || crate::install::verify_hash(&directory.join(&package.executable), &package.sha256)
                .is_err())
}

impl Manager {
    /// Explicit user retry may show the Windows UAC dialog again.
    pub fn install_windows_runtime(&self) -> Result<()> {
        let _gate = self.gate()?;
        self.install_windows_runtime_inner(true)
    }

    /// First launch provisions missing prerequisites once. Persisting the attempt
    /// before download/UAC also prevents repeated prompts after a crash.
    pub fn prepare_windows_runtime(&self) -> Result<()> {
        let _gate = self.gate()?;
        self.ensure_windows_runtime_inner()
    }

    /// Caller holds the operation gate. Used by component installation as well.
    pub(crate) fn ensure_windows_runtime_inner(&self) -> Result<()> {
        self.install_windows_runtime_inner(false)
    }

    fn install_windows_runtime_inner(&self, explicit_retry: bool) -> Result<()> {
        let marker = self.home.join("config/vc-runtime-attempted.json");
        if !may_attempt(
            crate::requirements::vc_runtime(),
            marker.exists(),
            explicit_retry,
        )? {
            return Ok(());
        }
        self.check_runtime_platform()?;
        crate::storage::atomic_write(&marker, b"{\"attempted\":true}")?;
        self.log("Eksik Visual C++ x64 çalışma zamanı indiriliyor ve doğrulanıyor.");
        let package = runtime_package();
        let directory = DataDir::new(&self.home).package(&package.id, &package.version);
        // Rebuild only an incomplete package directory, preserving its repair
        // backup through the existing verified installer.
        let repair = package_needs_repair(&directory, &package);
        self.install_package(&package, repair)?;
        let installer = directory.join(&package.executable);
        crate::install::verify_hash(&installer, VC_HASH)?;
        #[cfg(windows)]
        let _progress = self.permission_progress();
        let mut command = crate::process::command(crate::terminal::powershell_path());
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &installer_script(&installer),
        ]);
        let output = ManagedChild::output(command, Duration::from_secs(15 * 60))
            .context("Visual C++ kurulumu tamamlanamadı; yönetici izni gerekebilir. Kurulum hâlâ çalışıyorsa bitmesini bekleyin.")?;
        if !output.status.success() {
            bail!(
                "Visual C++ kurulumu tamamlanamadı; yönetici izni reddedilmiş olabilir. {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let reboot = check_installer_result(
            installer_exit(&output.stdout)?,
            crate::requirements::vc_runtime(),
        )?;
        std::fs::remove_file(&marker).context("Visual C++ kurulum deneme kaydı temizlenemedi.")?;
        self.log(if reboot {
            "Visual C++ x64 çalışma zamanı kullanılabilir. Microsoft yükleyicisi Windows'u uygun zamanda yeniden başlatmanızı öneriyor; otomatik yeniden başlatma yapılmadı."
        } else {
            "Visual C++ x64 çalışma zamanı kuruldu ve DLL dosyaları doğrulandı."
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_runtime_never_prompts_even_for_explicit_retry() {
        assert!(!may_attempt(true, false, false).unwrap());
        assert!(!may_attempt(true, true, true).unwrap());
    }

    #[test]
    fn failed_automatic_attempt_requires_explicit_retry() {
        assert!(may_attempt(false, true, false).is_err());
        assert!(may_attempt(false, true, true).unwrap());
        assert!(may_attempt(false, false, false).unwrap());
    }

    #[test]
    fn installer_success_requires_actual_loadable_runtime() {
        for code in [0, 1638, 3010] {
            assert!(check_installer_result(code, false).is_err());
            assert_eq!(check_installer_result(code, true).unwrap(), code == 3010);
        }
        assert!(check_installer_result(1603, true).is_err());
        assert!(check_installer_result(1641, true).is_err());
    }

    #[test]
    fn script_checks_hash_and_microsoft_signature_before_elevation() {
        let script = installer_script(Path::new("D:/Test O'Connor/vc.exe"));
        assert!(script.contains("O''Connor"));
        assert!(script.find("Get-FileHash").unwrap() < script.find("Start-Process").unwrap());
        assert!(
            script.find("Get-AuthenticodeSignature").unwrap()
                < script.find("Start-Process").unwrap()
        );
        assert!(script.contains("CN=Microsoft Corporation"));
        assert!(script.contains("'/install','/quiet','/norestart'"));
        assert!(script.contains("-Verb RunAs -WindowStyle Hidden -Wait -PassThru"));
    }

    #[test]
    fn parses_installer_result_without_truncating_windows_exit_code() {
        assert_eq!(
            installer_exit(b"SERVERBOND_VC_EXIT=3010\r\n").unwrap(),
            3010
        );
        assert!(installer_exit(b"success").is_err());
    }

    #[test]
    fn modified_cached_installer_is_repaired_even_with_valid_receipt() {
        let temporary = tempfile::tempdir().unwrap();
        let package = runtime_package();
        std::fs::write(
            temporary.path().join("installed.json"),
            serde_json::to_vec(&package).unwrap(),
        )
        .unwrap();
        std::fs::write(
            temporary.path().join(&package.executable),
            b"modified installer",
        )
        .unwrap();
        assert!(crate::install::validate_installation(temporary.path(), &package).is_ok());
        assert!(package_needs_repair(temporary.path(), &package));
    }

    #[cfg(windows)]
    #[test]
    fn powershell_rejects_bad_hash_or_publisher_without_launching_installer() {
        // Shadow every OS operation. This exercises PowerShell control flow
        // without downloading, requesting UAC or changing the host runtime.
        for (hash, status, subject, accepted) in [
            ("bad-hash", "Valid", "CN=Microsoft Corporation", false),
            (VC_HASH, "NotSigned", "CN=Microsoft Corporation", false),
            (VC_HASH, "Valid", "CN=Untrusted Publisher", false),
            (
                VC_HASH,
                "Valid",
                "CN=Microsoft Corporation, O=Microsoft Corporation",
                true,
            ),
        ] {
            let mocks = format!(
                "function Get-FileHash {{ [pscustomobject]@{{Hash='{hash}'}} }};\n\
                 function Get-AuthenticodeSignature {{ [pscustomobject]@{{Status='{status}';SignerCertificate=[pscustomobject]@{{Subject='{subject}'}}}} }};\n\
                 function Start-Process {{ Write-Host 'MOCK_INSTALLER_STARTED'; [pscustomobject]@{{ExitCode=1638}} }};\n"
            );
            let mut command = crate::process::command(crate::terminal::powershell_path());
            command.args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("{mocks}{}", installer_script(Path::new("D:/unused.exe"))),
            ]);
            let output = ManagedChild::output(command, Duration::from_secs(30)).unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert_eq!(output.status.success(), accepted, "{stdout}");
            assert_eq!(stdout.contains("MOCK_INSTALLER_STARTED"), accepted);
            if accepted {
                assert_eq!(installer_exit(&output.stdout).unwrap(), 1638);
            }
        }
    }
}
