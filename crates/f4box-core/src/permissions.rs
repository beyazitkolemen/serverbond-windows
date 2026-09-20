use crate::{
    process::{command, ManagedChild},
    storage, Manager,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

/// Everything the elevated helper is allowed to change, in the order the script
/// applies it. The list is fixed in code so an elevated run can never be widened
/// from configuration or from the interface.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PermissionState {
    pub granted: bool,
    pub applied_at: Option<String>,
    pub applied: Vec<String>,
    pub failed: Vec<String>,
    pub programs: Vec<String>,
    pub defender_exclusion: bool,
    /// Managed programs installed after the last elevated run.
    pub pending: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScriptResult {
    #[serde(default)]
    applied: Vec<String>,
    #[serde(default)]
    failed: Vec<String>,
}

/// PowerShell single-quoted literals only need the quote itself doubled, which
/// keeps user-chosen folder names out of the command grammar.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// netsh and icacls store the path as given; both expect Windows separators.
fn native(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', "\\")
}

fn rule_name(program: &str) -> String {
    format!("F4Box: {}", program.rsplit('\\').next().unwrap_or(program))
}

pub(crate) fn script(
    home: &Path,
    programs: &[PathBuf],
    account: Option<&str>,
    defender: bool,
    result: &Path,
) -> String {
    let mut text = String::from(
        "# F4Box tarafından üretilir. Yükseltilmiş yetkiyle bir kez çalışır.\n$ErrorActionPreference = 'Stop'\n$applied = New-Object System.Collections.ArrayList\n$failed = New-Object System.Collections.ArrayList\n",
    );
    for program in programs {
        let path = native(program);
        let name = rule_name(&path);
        // netsh works on every Windows edition and localization; rule names are ours.
        text.push_str(&format!(
            "try {{\n  $null = netsh advfirewall firewall delete rule name={name} 2>&1\n  $out = netsh advfirewall firewall add rule name={name} dir=in action=allow enable=yes profile=private,domain program={path} 2>&1\n  if ($LASTEXITCODE -ne 0) {{ throw ($out | Out-String) }}\n  $out = netsh advfirewall firewall add rule name={name} dir=out action=allow enable=yes profile=private,domain program={path} 2>&1\n  if ($LASTEXITCODE -ne 0) {{ throw ($out | Out-String) }}\n  $null = $applied.Add('Güvenlik duvarı kuralı: ' + {path})\n}} catch {{\n  $null = $failed.Add('Güvenlik duvarı kuralı: ' + {path} + ' · ' + $_.Exception.Message)\n}}\n",
            name = quote(&name),
            path = quote(&path),
        ));
    }
    if let Some(account) = account {
        let home_path = native(home);
        text.push_str(&format!(
            "try {{\n  $out = icacls {home} /grant {grant} /T /C 2>&1\n  if ($LASTEXITCODE -ne 0) {{ throw ($out | Out-String) }}\n  $null = $applied.Add('Klasör yetkisi: ' + {home})\n}} catch {{\n  $null = $failed.Add('Klasör yetkisi: ' + $_.Exception.Message)\n}}\n",
            home = quote(&home_path),
            grant = quote(&format!("{account}:(OI)(CI)F")),
        ));
    }
    if defender {
        text.push_str(&format!(
            "try {{\n  Add-MpPreference -ExclusionPath {home} -ErrorAction Stop\n  $null = $applied.Add('Defender klasör istisnası: ' + {home})\n}} catch {{\n  $null = $failed.Add('Defender klasör istisnası: ' + $_.Exception.Message)\n}}\n",
            home = quote(&native(home)),
        ));
    }
    text.push_str(&format!(
        "[pscustomobject]@{{ applied = @($applied); failed = @($failed) }} | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath {result} -Encoding UTF8\n",
        result = quote(&native(result)),
    ));
    text
}

impl Manager {
    fn permission_marker(&self) -> PathBuf {
        self.home.join("config/permissions.json")
    }

    /// Managed executables that listen on a socket or talk to the network.
    pub(crate) fn managed_programs(&self) -> Vec<PathBuf> {
        let mut programs = Vec::new();
        let config = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let mut versions: Vec<String> = config
            .projects
            .iter()
            .map(|project| project.php_version.clone())
            .chain(Some(config.php_version))
            .collect();
        versions.sort();
        versions.dedup();
        for version in versions {
            let directory = self.home.join("bin/php").join(&version);
            for name in ["php.exe", "php-cgi.exe"] {
                let path = directory.join(name);
                if path.is_file() {
                    programs.push(path);
                }
            }
        }
        for id in ["mysql", "caddy"] {
            if let Ok(path) = self.executable(id) {
                programs.push(path);
            }
        }
        for tool in [crate::tunnel::ID, crate::mail::ID, crate::node::ID] {
            if let Ok(path) = self.tool_executable(tool) {
                programs.push(path);
            }
        }
        programs
    }

    pub fn permission_state(&self) -> PermissionState {
        let mut state: PermissionState =
            storage::read_limited(&self.permission_marker(), 256 * 1024)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or_default();
        state.pending = self
            .managed_programs()
            .iter()
            .map(|path| native(path))
            .filter(|path| !state.programs.contains(path))
            .collect();
        state
    }

    /// Runs one elevated helper: a single Windows consent prompt covers the
    /// firewall rules, the data folder permission and the optional antivirus
    /// exclusion. Nothing else is ever elevated.
    pub fn grant_permissions(&self, defender: bool) -> Result<PermissionState> {
        let _guard = self.gate()?;
        let programs = self.managed_programs();
        if programs.is_empty() {
            bail!("Önce bileşenleri kurun; yetki verilecek program bulunamadı.");
        }
        let account = std::env::var("USERNAME").ok().and_then(|user| {
            let user = user.trim().to_string();
            if user.is_empty() || user.contains('"') || user.contains('\'') {
                return None;
            }
            Some(match std::env::var("USERDOMAIN") {
                Ok(domain) if !domain.trim().is_empty() && !domain.contains('\\') => {
                    format!("{}\\{user}", domain.trim())
                }
                _ => user,
            })
        });
        let directory = self.home.join("config");
        let result = directory.join(format!("permissions-{}.json", uuid::Uuid::new_v4()));
        let script_path = directory.join(format!("permissions-{}.ps1", uuid::Uuid::new_v4()));
        // UTF-8 with BOM: PowerShell 5.1 reads Turkish text in scripts correctly only with it.
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(
            script(&self.home, &programs, account.as_deref(), defender, &result).as_bytes(),
        );
        storage::atomic_write(&script_path, bytes)?;
        let cleanup = || {
            let _ = std::fs::remove_file(&script_path);
            let _ = std::fs::remove_file(&result);
        };
        self.log("Windows yetki penceresi açılıyor. Onay verilmezse hiçbir değişiklik yapılmaz.");
        let mut cmd = command(crate::terminal::powershell_path());
        cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command"])
            .arg(format!(
                "$process = Start-Process -FilePath {shell} -ArgumentList '-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',{script} -Verb RunAs -WindowStyle Hidden -Wait -PassThru; exit $process.ExitCode",
                shell = quote(&native(&crate::terminal::powershell_path())),
                script = quote(&native(&script_path)),
            ));
        let output = ManagedChild::output(cmd, Duration::from_secs(300));
        let output = match output {
            Ok(output) => output,
            Err(error) => {
                cleanup();
                return Err(error);
            }
        };
        if !output.status.success() {
            cleanup();
            let detail = String::from_utf8_lossy(&output.stderr);
            if detail.contains("canceled") || detail.contains("iptal") {
                bail!("Yetki isteği onaylanmadı. Hiçbir ayar değiştirilmedi.");
            }
            bail!(
                "Yetki verilemedi: {} {}",
                output.status,
                detail.chars().take(600).collect::<String>()
            );
        }
        let parsed: ScriptResult = serde_json::from_slice(
            &storage::read_limited(&result, 256 * 1024)
                .context("Yetki sonucu okunamadı. Hiçbir ayarın uygulandığı varsayılmadı.")?,
        )
        .context("Yetki sonucu anlaşılamadı.")?;
        let state = PermissionState {
            granted: !parsed.applied.is_empty(),
            applied_at: Some(chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()),
            programs: programs
                .iter()
                .map(|path| native(path))
                .filter(|path| {
                    parsed
                        .applied
                        .iter()
                        .any(|line| line.contains(path.as_str()))
                })
                .collect(),
            defender_exclusion: defender
                && parsed
                    .applied
                    .iter()
                    .any(|line| line.starts_with("Defender")),
            applied: parsed.applied,
            failed: parsed.failed,
            pending: Vec::new(),
        };
        storage::atomic_write(
            &self.permission_marker(),
            serde_json::to_vec_pretty(&state)?,
        )?;
        cleanup();
        for line in &state.applied {
            self.log(format!("Yetki verildi · {line}"));
        }
        for line in &state.failed {
            self.log(format!("Yetki verilemedi · {line}"));
        }
        Ok(self.permission_state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_quotes_paths_and_covers_every_requested_change() {
        let home = PathBuf::from(r"C:\Users\O'Brien\F4Box");
        let programs = vec![home.join(r"bin\php\8.4.25\php-cgi.exe")];
        let text = script(
            &home,
            &programs,
            Some(r"HOST\dev"),
            true,
            &home.join("config/result.json"),
        );
        assert!(text.contains("'C:\\Users\\O''Brien\\F4Box\\bin\\php\\8.4.25\\php-cgi.exe'"));
        assert!(text.contains("name='F4Box: php-cgi.exe' dir=in action=allow"));
        assert!(text.contains("dir=out action=allow"));
        assert!(text.contains("icacls 'C:\\Users\\O''Brien\\F4Box' /grant 'HOST\\dev:(OI)(CI)F'"));
        assert!(text.contains("Add-MpPreference -ExclusionPath 'C:\\Users\\O''Brien\\F4Box'"));
        assert!(text.contains("ConvertTo-Json"));
        assert_eq!(
            text.matches("netsh advfirewall firewall add rule").count(),
            2
        );
    }

    #[test]
    fn script_leaves_out_changes_that_were_not_requested() {
        let home = PathBuf::from(r"C:\F4Box");
        let text = script(&home, &[], None, false, &home.join("result.json"));
        assert!(!text.contains("netsh"));
        assert!(!text.contains("icacls"));
        assert!(!text.contains("Add-MpPreference"));
        assert!(text.contains("ConvertTo-Json"));
    }
}
