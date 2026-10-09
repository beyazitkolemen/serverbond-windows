//! Prepare WebView2 before Tauri creates its first webview, including portable launches.
use anyhow::{bail, Context, Result};
use serverbond_core::{install, model::Package, Manager};
use std::{
    io::{Read, Seek, SeekFrom},
    os::windows::process::CommandExt,
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
use winreg::{enums::*, RegKey};

const BOOTSTRAPPER_SHA256: &str =
    "aa38a8cfce6179b87181609b1c730a29eaf26138fc833af5759e67576770f3a3";
const MINIMUM_VERSION: [u32; 4] = [109, 0, 1518, 0];
const RUNTIME_KEY: &str =
    r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

fn supported_version(value: &str) -> bool {
    let parts: Option<Vec<u32>> = value.split('.').map(|part| part.parse().ok()).collect();
    parts.is_some_and(|parts| parts.len() == 4 && parts.as_slice() >= MINIMUM_VERSION.as_slice())
}

fn runtime_ready() -> bool {
    [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER]
        .into_iter()
        .any(|hive| {
            RegKey::predef(hive)
                .open_subkey_with_flags(RUNTIME_KEY, KEY_READ | KEY_WOW64_32KEY)
                .and_then(|key| key.get_value::<String, _>("pv"))
                .is_ok_and(|version| supported_version(&version))
        })
}

fn package() -> Package {
    Package {
        id: "webview2-bootstrapper".into(),
        name: "Microsoft Edge WebView2 Runtime Bootstrapper".into(),
        version: "1.3.275.13".into(),
        description: "Windows masaüstü arayüzü çalışma zamanı".into(),
        url: "https://msedge.sf.dl.delivery.mp.microsoft.com/filestreamingservice/files/c289719c-c70c-464b-81ea-9efa692d5428/MicrosoftEdgeWebview2Setup.exe".into(),
        sha256: BOOTSTRAPPER_SHA256.into(),
        archive: false,
        prefix: String::new(),
        executable: "MicrosoftEdgeWebview2Setup.exe".into(),
        license: "Microsoft Software License Terms".into(),
        source: "https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution".into(),
    }
}

fn powershell_literal(value: &str) -> String {
    let mut literal = String::with_capacity(value.len() + 2);
    literal.push('\'');
    for character in value.chars() {
        match character {
            '\'' => literal.push_str("''"),
            '\u{2018}'..='\u{201F}' => {
                literal.push_str(&format!("'+[char]0x{:04X}+'", character as u32));
            }
            _ => literal.push(character),
        }
    }
    literal.push('\'');
    literal
}

fn installer_script(path: &Path) -> String {
    let escaped = powershell_literal(&path.to_string_lossy());
    format!(
        "$ErrorActionPreference='Stop';\n\
         $installer={escaped};\n\
         if ((Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash -ne '{BOOTSTRAPPER_SHA256}') {{ throw 'WebView2 SHA-256 doğrulaması başarısız.' }}\n\
         $signature=Get-AuthenticodeSignature -LiteralPath $installer;\n\
         if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '(^|,\\s*)CN=Microsoft Corporation(,|$)') {{ throw 'Geçerli Microsoft imzası bulunamadı.' }}\n\
         $process=Start-Process -FilePath $installer -ArgumentList '/silent','/install' -WindowStyle Hidden -PassThru;\n\
         if (!$process.WaitForExit(900000)) {{ throw 'WebView2 kurulumu zaman sınırını aştı. Kurulum sürüyorsa bitmesini bekleyin.' }}\n\
         Write-Output ('SERVERBOND_WEBVIEW_EXIT=' + $process.ExitCode);\n"
    )
}

fn installer_exit(stdout: &[u8]) -> Result<i32> {
    String::from_utf8_lossy(stdout)
        .lines()
        .find_map(|line| line.trim().strip_prefix("SERVERBOND_WEBVIEW_EXIT="))
        .and_then(|value| value.parse().ok())
        .context("WebView2 kurulum sonucu alınamadı.")
}

fn needs_repair(home: &Path, package: &Package) -> bool {
    let health = install::health(home, package);
    let installer = home
        .join("bin")
        .join(&package.id)
        .join(&package.version)
        .join(&package.executable);
    health.repairable
        && (!health.installed || install::verify_hash(&installer, &package.sha256).is_err())
}

// Bound the entire PowerShell operation, including signature verification and
// Start-Process. Avoid a kill-on-close Job Object: the Microsoft installer must
// be allowed to finish safely even when its reporting wrapper times out.
fn run_script(mut command: Command, timeout: Duration) -> Result<Output> {
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    command
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?);
    let mut child = command.spawn()?;
    let started = Instant::now();
    let limit = 1024 * 1024;
    let status = loop {
        let size = stdout
            .metadata()?
            .len()
            .saturating_add(stderr.metadata()?.len());
        if size > limit || started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            bail!("WebView2 hazırlama işlemi zaman veya çıktı sınırını aştı. Microsoft yükleyicisi hâlâ çalışıyor olabilir; bitmesini bekleyip tekrar deneyin.");
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let read = |file: &mut std::fs::File, remaining: u64| -> Result<Vec<u8>> {
        file.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        file.take(remaining + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > remaining {
            bail!("WebView2 hazırlama çıktısı 1 MB sınırını aştı.");
        }
        Ok(bytes)
    };
    let stdout = read(&mut stdout, limit)?;
    let stderr = read(&mut stderr, limit - stdout.len() as u64)?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// This runs before Tauri's builder so a missing runtime can be installed
/// without requiring an already-working webview. No automatic system reboot.
pub fn ensure(home: &Path) -> Result<()> {
    if runtime_ready() {
        return Ok(());
    }
    // Reuse the data-directory lock and initialization before any download.
    // A first portable launch has no cache directory yet; concurrent launches
    // must not extract or execute the same bootstrapper at the same time.
    // Keep this guard alive until provisioning finishes, then release it before
    // Tauri's setup opens its long-lived Manager.
    let bootstrap_manager = Manager::open_recovering(home.to_path_buf())?;
    let home = bootstrap_manager.home.as_path();
    // Another process may have completed installation before we got the lock.
    if runtime_ready() {
        return Ok(());
    }
    let package = package();
    if needs_repair(home, &package) {
        install::repair(home, &package, |_| {})?;
    } else {
        install::install(home, &package, |_| {})?;
    }
    let installer = home
        .join("bin")
        .join(&package.id)
        .join(&package.version)
        .join(&package.executable);
    install::verify_hash(&installer, BOOTSTRAPPER_SHA256)?;
    let system_root =
        std::env::var_os("SystemRoot").context("Windows sistem dizini bulunamadı.")?;
    let powershell = Path::new(&system_root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut command = Command::new(powershell);
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &installer_script(&installer),
        ])
        .creation_flags(0x08000000);
    let output = run_script(command, Duration::from_secs(16 * 60))
        .context("WebView2 yükleyicisi başlatılamadı.")?;
    if !output.status.success() {
        bail!(
            "WebView2 kurulumu tamamlanamadı. İnternet bağlantınızı kontrol edip tekrar açın veya ServerBond Setup paketini çalıştırın. {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let code = installer_exit(&output.stdout)?;
    if !matches!(code, 0 | 3010) || !runtime_ready() {
        bail!("WebView2 hazır değil (kurulum kodu {code}). Yeniden başlatma istenmişse Windows'u yeniden başlatın; ardından tekrar deneyin veya ServerBond Setup paketini çalıştırın.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_malformed_and_old_versions_require_installation() {
        for version in [
            "",
            "0.0.0.0",
            "109.0.1517.99",
            "not-a-version",
            "109.0.1518",
            "109.0.1518.0.1",
        ] {
            assert!(!supported_version(version), "{version}");
        }
        assert!(supported_version("109.0.1518.0"));
        assert!(supported_version("141.0.3537.71"));
    }

    #[test]
    fn verifies_microsoft_and_pinned_hash_before_launching() {
        let script = installer_script(Path::new("D:/O'Connor/webview setup.exe"));
        assert!(script.contains("O''Connor"));
        let launch = script.find("Start-Process").unwrap();
        assert!(script.find("Get-FileHash").unwrap() < launch);
        assert!(script.find("Get-AuthenticodeSignature").unwrap() < launch);
        assert!(script.contains("CN=Microsoft Corporation"));
        assert!(script.contains("'/silent','/install'"));
        assert!(!script.contains("RunAs"));
        assert!(script.contains("WaitForExit(900000)"));
    }

    #[test]
    fn typographic_quotes_in_paths_cannot_end_powershell_literal() {
        for character in '\u{2018}'..='\u{201F}' {
            let script = installer_script(Path::new(&format!(
                "D:/Example{character};Write-Output injected;{character}/webview.exe"
            )));
            assert!(!script.contains(character));
            assert!(script.contains(&format!("'+[char]0x{:04X}+'", character as u32)));
        }
    }

    #[test]
    fn parses_full_exit_status_and_rejects_missing_status() {
        assert_eq!(installer_exit(b"SERVERBOND_WEBVIEW_EXIT=0\r\n").unwrap(), 0);
        assert_eq!(
            installer_exit(b"SERVERBOND_WEBVIEW_EXIT=-2147219198\n").unwrap(),
            -2147219198
        );
        assert!(installer_exit(b"unrelated output").is_err());
    }

    #[test]
    fn fresh_portable_home_is_initialized_and_locked_before_provisioning() {
        let temporary = tempfile::tempdir().unwrap();
        let home = temporary.path().join("first-launch");
        assert!(!home.exists());
        let manager = Manager::open_recovering(home.clone()).unwrap();
        for directory in ["bin", "cache", "config", "logs"] {
            assert!(home.join(directory).is_dir());
        }
        assert!(Manager::open_recovering(home.clone()).is_err());
        drop(manager);
        assert!(Manager::open_recovering(home).is_ok());
    }

    #[test]
    fn corrupted_bootstrapper_with_valid_receipt_is_repaired() {
        let temporary = tempfile::tempdir().unwrap();
        let package = package();
        let directory = temporary
            .path()
            .join("bin")
            .join(&package.id)
            .join(&package.version);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("installed.json"),
            serde_json::to_vec(&package).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.join(&package.executable),
            b"modified bootstrapper",
        )
        .unwrap();
        assert!(install::health(temporary.path(), &package).installed);
        assert!(needs_repair(temporary.path(), &package));
    }

    #[test]
    fn outer_script_timeout_covers_work_before_installer_launch() {
        let shell = Path::new(&std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut command = Command::new(shell);
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ])
            .creation_flags(0x08000000);
        let start = Instant::now();
        let error = run_script(command, Duration::from_millis(100)).unwrap_err();
        assert!(error.to_string().contains("bitmesini bekleyip"));
        assert!(start.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn outer_script_captures_status_and_bounded_output() {
        let shell = Path::new(&std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut command = Command::new(&shell);
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Console]::Out.Write('result'); exit 7",
            ])
            .creation_flags(0x08000000);
        let output = run_script(command, Duration::from_secs(30)).unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"result");
        let mut command = Command::new(shell);
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Console]::Out.Write(('x' * 1048577))",
            ])
            .creation_flags(0x08000000);
        assert!(run_script(command, Duration::from_secs(30)).is_err());
    }
}
