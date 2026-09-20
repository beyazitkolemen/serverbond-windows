#[cfg(windows)]
use crate::process::{command, ManagedChild};
use crate::{storage, Manager};
#[cfg(windows)]
use anyhow::Context;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::time::{Duration, Instant};

pub const TASK: &str = crate::product::PERMISSIONS_TASK;

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
    /// Managed programs installed after the last successful apply.
    pub pending: Vec<String>,
    /// A highest-privilege scheduled task can refresh rules without a new UAC.
    pub helper: bool,
    /// The user dismissed the first-launch prompt. Later launches stay silent.
    pub declined: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PermissionAction {
    Skip,
    Prompt,
    Silent,
}

/// One Windows consent covers first launch. Later program installs reuse the
/// helper. An explicit button always retries; a declined first launch never
/// nags again on its own.
pub(crate) fn next_action(
    granted: bool,
    declined: bool,
    helper: bool,
    pending: bool,
    explicit: bool,
) -> PermissionAction {
    if explicit {
        return if helper && granted {
            PermissionAction::Silent
        } else {
            PermissionAction::Prompt
        };
    }
    if declined {
        return PermissionAction::Skip;
    }
    if helper && pending {
        return PermissionAction::Silent;
    }
    if !granted {
        return PermissionAction::Prompt;
    }
    PermissionAction::Skip
}

#[cfg(windows)]
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
#[cfg(any(test, windows))]
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// netsh and icacls store the path as given; both expect Windows separators.
fn native(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', "\\")
}

#[cfg(any(test, windows))]
fn rule_name(program: &str) -> String {
    format!(
        "{}: {}",
        crate::product::NAME,
        program.rsplit('\\').next().unwrap_or(program)
    )
}

#[cfg(any(test, windows))]
pub(crate) fn script(
    home: &Path,
    programs: &[PathBuf],
    account: Option<&str>,
    defender: bool,
    result: &Path,
    helper_script: Option<&Path>,
) -> String {
    let mut text = String::from(
        "# ServerBond tarafından üretilir. Yükseltilmiş yetkiyle çalışır.\n$ErrorActionPreference = 'Stop'\n$applied = New-Object System.Collections.ArrayList\n$failed = New-Object System.Collections.ArrayList\ntry {\n",
    );
    for program in programs {
        let path = native(program);
        let name = rule_name(&path);
        // netsh works on every Windows edition and localization; rule names are ours.
        text.push_str(&format!(
            "  try {{\n    $null = netsh advfirewall firewall delete rule name={name} 2>&1\n    $out = netsh advfirewall firewall add rule name={name} dir=in action=allow enable=yes profile=private,domain program={path} 2>&1\n    if ($LASTEXITCODE -ne 0) {{ throw ($out | Out-String) }}\n    $out = netsh advfirewall firewall add rule name={name} dir=out action=allow enable=yes profile=private,domain program={path} 2>&1\n    if ($LASTEXITCODE -ne 0) {{ throw ($out | Out-String) }}\n    $null = $applied.Add('Güvenlik duvarı kuralı: ' + {path})\n  }} catch {{\n    $null = $failed.Add('Güvenlik duvarı kuralı: ' + {path} + ' · ' + $_.Exception.Message)\n  }}\n",
            name = quote(&name),
            path = quote(&path),
        ));
    }
    if let Some(account) = account {
        let home_path = native(home);
        text.push_str(&format!(
            "  try {{\n    $out = icacls {home} /grant {grant} /T /C 2>&1\n    if ($LASTEXITCODE -ne 0) {{ throw ($out | Out-String) }}\n    $null = $applied.Add('Klasör yetkisi: ' + {home})\n  }} catch {{\n    $null = $failed.Add('Klasör yetkisi: ' + $_.Exception.Message)\n  }}\n",
            home = quote(&home_path),
            grant = quote(&format!("{account}:(OI)(CI)F")),
        ));
    }
    if defender {
        text.push_str(&format!(
            "  try {{\n    Add-MpPreference -ExclusionPath {home} -ErrorAction Stop\n    $null = $applied.Add('Defender klasör istisnası: ' + {home})\n  }} catch {{\n    $null = $failed.Add('Defender klasör istisnası: ' + $_.Exception.Message)\n  }}\n",
            home = quote(&native(home)),
        ));
    }
    if let Some(helper) = helper_script {
        let shell = native(&crate::terminal::powershell_path());
        let script = native(helper);
        text.push_str(&format!(
            "  try {{\n    $action = New-ScheduledTaskAction -Execute {shell} -Argument ('-NoProfile -NonInteractive -ExecutionPolicy Bypass -File ' + {script})\n    $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Highest\n    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew\n    Register-ScheduledTask -TaskName {task} -Action $action -Principal $principal -Settings $settings -Description 'ServerBond güvenlik duvarı ve klasör izinlerini yeniler.' -Force | Out-Null\n    $null = icacls {script} /inheritance:r /grant:r 'SYSTEM:(F)' /grant:r ($env:USERNAME + ':(F)') 2>&1\n    $null = $applied.Add('Zamanlanmış görev: ' + {task})\n  }} catch {{\n    $null = $failed.Add('Zamanlanmış görev: ' + $_.Exception.Message)\n  }}\n",
            shell = quote(&shell),
            script = quote(&script),
            task = quote(TASK),
        ));
    }
    text.push_str(&format!(
        "}} finally {{\n  [pscustomobject]@{{ applied = @($applied); failed = @($failed) }} | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath {result} -Encoding UTF8\n}}\n",
        result = quote(&native(result)),
    ));
    text
}

impl Manager {
    fn permission_marker(&self) -> PathBuf {
        self.home.join("config/permissions.json")
    }

    #[cfg(windows)]
    fn permission_script_path(&self) -> PathBuf {
        self.home.join("config/permissions-apply.ps1")
    }

    #[cfg(windows)]
    fn permission_result_path(&self) -> PathBuf {
        self.home.join("config/permissions-result.json")
    }

    fn stored_permissions(&self) -> PermissionState {
        storage::read_limited(&self.permission_marker(), 256 * 1024)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    #[cfg(windows)]
    fn save_permission_state(&self, state: &PermissionState) -> Result<()> {
        storage::atomic_write(&self.permission_marker(), serde_json::to_vec_pretty(state)?)?;
        Ok(())
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
        for tool in [
            crate::tunnel::ID,
            crate::mail::ID,
            crate::postgres::ID,
            crate::redis::ID,
            crate::node::ID,
        ] {
            if let Ok(path) = self.tool_executable(tool) {
                programs.push(path);
            }
        }
        programs
    }

    pub fn permission_state(&self) -> PermissionState {
        let mut state = self.stored_permissions();
        state.pending = self
            .managed_programs()
            .iter()
            .map(|path| native(path))
            .filter(|path| !state.programs.contains(path))
            .collect();
        state
    }

    #[cfg(windows)]
    fn windows_account() -> Option<String> {
        let user = std::env::var("USERNAME").ok()?;
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
    }

    #[cfg(windows)]
    fn write_permission_script(&self, defender: bool) -> Result<(PathBuf, PathBuf)> {
        let script_path = self.permission_script_path();
        let result = self.permission_result_path();
        let programs = self.managed_programs();
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(
            script(
                &self.home,
                &programs,
                Self::windows_account().as_deref(),
                defender,
                &result,
                Some(&script_path),
            )
            .as_bytes(),
        );
        std::fs::create_dir_all(script_path.parent().unwrap())?;
        storage::atomic_write(&script_path, bytes)?;
        let _ = std::fs::remove_file(&result);
        Ok((script_path, result))
    }

    #[cfg(windows)]
    fn record_declined(&self) -> Result<PermissionState> {
        let mut state = self.stored_permissions();
        state.declined = true;
        self.save_permission_state(&state)?;
        self.log("Windows yetki isteği onaylanmadı. ServerBond bir daha kendiliğinden sormaz; Ayarlar → Sistem ekranından yeniden isteyebilirsiniz.");
        Ok(self.permission_state())
    }

    #[cfg(windows)]
    fn store_result(
        &self,
        parsed: ScriptResult,
        programs: &[PathBuf],
        defender: bool,
    ) -> Result<PermissionState> {
        let helper = parsed
            .applied
            .iter()
            .any(|line| line.starts_with("Zamanlanmış görev:"));
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
            helper,
            declined: false,
        };
        self.save_permission_state(&state)?;
        for line in &state.applied {
            self.log(format!("Yetki verildi · {line}"));
        }
        for line in &state.failed {
            self.log(format!("Yetki verilemedi · {line}"));
        }
        Ok(self.permission_state())
    }

    #[cfg(windows)]
    fn wait_for_result(&self, result: &Path) -> Result<ScriptResult> {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            if result.is_file() {
                break;
            }
            if Instant::now() > deadline {
                bail!("Yetki sonucu yazılmadı. Günlükler ekranını kontrol edin.");
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        serde_json::from_slice(
            &storage::read_limited(result, 256 * 1024)
                .context("Yetki sonucu okunamadı. Hiçbir ayarın uygulandığı varsayılmadı.")?,
        )
        .context("Yetki sonucu anlaşılamadı.")
    }

    #[cfg(windows)]
    fn run_helper_task(&self) -> Result<()> {
        let mut cmd = command("schtasks");
        cmd.args(["/Run", "/TN", TASK]);
        let output = ManagedChild::output(cmd, Duration::from_secs(30))?;
        if !output.status.success() {
            bail!(
                "Zamanlanmış görev başlatılamadı: {}",
                String::from_utf8_lossy(&output.stderr)
                    .chars()
                    .take(400)
                    .collect::<String>()
            );
        }
        Ok(())
    }

    #[cfg(windows)]
    fn prompt_for_elevation(&self, script_path: &Path) -> Result<bool> {
        self.log("Windows yetki penceresi açılıyor. Onay bir kez istenir; sonraki kurulumlar bu pencereyi açmaz.");
        let mut cmd = command(crate::terminal::powershell_path());
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
        ])
        .arg(format!(
            "$process = Start-Process -FilePath {shell} -ArgumentList '-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',{script} -Verb RunAs -WindowStyle Hidden -Wait -PassThru; exit $process.ExitCode",
            shell = quote(&native(&crate::terminal::powershell_path())),
            script = quote(&native(script_path)),
        ));
        let output = ManagedChild::output(cmd, Duration::from_secs(300))?;
        if output.status.success() {
            return Ok(true);
        }
        let detail = String::from_utf8_lossy(&output.stderr);
        if detail.contains("canceled")
            || detail.contains("cancelled")
            || detail.contains("iptal")
            || detail.contains("1223")
        {
            return Ok(false);
        }
        bail!(
            "Yetki verilemedi: {} {}",
            output.status,
            detail.chars().take(600).collect::<String>()
        );
    }

    fn apply_permissions(
        &self,
        defender: bool,
        action: PermissionAction,
    ) -> Result<PermissionState> {
        #[cfg(not(windows))]
        {
            let _ = (defender, action);
            bail!("Windows yetkisi yalnızca Windows'ta uygulanır.");
        }
        #[cfg(windows)]
        {
            if action == PermissionAction::Skip {
                return Ok(self.permission_state());
            }
            let (script_path, result) = self.write_permission_script(defender)?;
            let accepted = match action {
                PermissionAction::Silent => {
                    self.log("Kayıtlı Windows yardımcısı izinleri yeniliyor; yeni onay penceresi açılmaz.");
                    self.run_helper_task()?;
                    true
                }
                PermissionAction::Prompt => self.prompt_for_elevation(&script_path)?,
                PermissionAction::Skip => true,
            };
            if !accepted {
                return self.record_declined();
            }
            let parsed = match self.wait_for_result(&result) {
                Ok(parsed) => parsed,
                Err(error) => {
                    let _ = std::fs::remove_file(&result);
                    return Err(error);
                }
            };
            let _ = std::fs::remove_file(&result);
            self.store_result(parsed, &self.managed_programs(), defender)
        }
    }

    /// First launch: one UAC. Later launches stay silent unless the user asks.
    pub fn ensure_permissions(&self) -> Result<PermissionState> {
        let _guard = self.gate()?;
        let state = self.permission_state();
        let action = next_action(
            state.granted,
            state.declined,
            state.helper,
            !state.pending.is_empty(),
            false,
        );
        match action {
            PermissionAction::Skip => Ok(state),
            PermissionAction::Silent | PermissionAction::Prompt => {
                self.apply_permissions(true, action)
            }
        }
    }

    /// Settings button or CLI. A declined first launch can be retried here.
    pub fn grant_permissions(&self, defender: bool) -> Result<PermissionState> {
        let _guard = self.gate()?;
        let state = self.permission_state();
        let action = next_action(
            state.granted,
            state.declined,
            state.helper,
            !state.pending.is_empty() || !state.granted,
            true,
        );
        self.apply_permissions(defender, action)
    }

    /// Called after a package install while the caller already holds the gate.
    /// Never opens a UAC prompt: that would undo the first-launch promise.
    pub(crate) fn refresh_permissions_quietly(&self) {
        let state = self.permission_state();
        if next_action(
            state.granted,
            state.declined,
            state.helper,
            !state.pending.is_empty(),
            false,
        ) != PermissionAction::Silent
        {
            return;
        }
        if let Err(error) =
            self.apply_permissions(state.defender_exclusion, PermissionAction::Silent)
        {
            self.log(format!(
                "Yeni program için Windows izni yenilenemedi: {error:#}"
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_launch_asks_once_later_changes_reuse_the_helper() {
        use PermissionAction::*;
        assert_eq!(next_action(false, false, false, false, false), Prompt);
        assert_eq!(next_action(false, true, false, false, false), Skip);
        assert_eq!(next_action(true, false, true, true, false), Silent);
        assert_eq!(next_action(true, false, true, false, false), Skip);
        assert_eq!(next_action(true, false, false, true, false), Skip);
        assert_eq!(next_action(true, false, false, true, true), Prompt);
        assert_eq!(next_action(false, true, false, false, true), Prompt);
        assert_eq!(next_action(true, false, true, true, true), Silent);
    }

    #[test]
    fn script_quotes_paths_registers_the_helper_and_covers_every_change() {
        let home = PathBuf::from(r"C:\Users\O'Brien\ServerBond");
        let programs = vec![home.join(r"bin\php\8.4.25\php-cgi.exe")];
        let helper = home.join(r"config\permissions-apply.ps1");
        let text = script(
            &home,
            &programs,
            Some(r"HOST\dev"),
            true,
            &home.join("config/result.json"),
            Some(&helper),
        );
        assert!(text.contains("'C:\\Users\\O''Brien\\ServerBond\\bin\\php\\8.4.25\\php-cgi.exe'"));
        assert!(text.contains("name='ServerBond: php-cgi.exe' dir=in action=allow"));
        assert!(text.contains("dir=out action=allow"));
        assert!(
            text.contains("icacls 'C:\\Users\\O''Brien\\ServerBond' /grant 'HOST\\dev:(OI)(CI)F'")
        );
        assert!(text.contains("Add-MpPreference -ExclusionPath 'C:\\Users\\O''Brien\\ServerBond'"));
        assert!(text.contains("Register-ScheduledTask -TaskName 'ServerBond Permissions'"));
        assert!(text.contains("-RunLevel Highest"));
        assert!(text.contains("permissions-apply.ps1"));
        assert!(text.contains("finally"));
        assert_eq!(
            text.matches("netsh advfirewall firewall add rule").count(),
            2
        );
    }

    #[test]
    fn script_leaves_out_changes_that_were_not_requested() {
        let home = PathBuf::from(r"C:\ServerBond");
        let text = script(&home, &[], None, false, &home.join("result.json"), None);
        assert!(!text.contains("netsh"));
        assert!(!text.contains("icacls"));
        assert!(!text.contains("Add-MpPreference"));
        assert!(!text.contains("Register-ScheduledTask"));
        assert!(text.contains("ConvertTo-Json"));
    }
}
