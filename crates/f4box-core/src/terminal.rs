use crate::{install, model::php_package, Manager};
use anyhow::{Context, Result};
use base64::Engine;
use std::{path::PathBuf, process::Command};

fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub(crate) fn powershell_path() -> PathBuf {
    PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
        .join("System32/WindowsPowerShell/v1.0/powershell.exe")
}

impl Manager {
    /// The same initialization is used for interactive terminals and integration checks.
    pub fn project_terminal_script(&self, id: &str) -> Result<String> {
        let project = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .projects
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .context("Proje bulunamadı.")?;
        let package = php_package(&project.php_version)?;
        let directory = self.home.join("bin/php").join(&project.php_version);
        install::validate_installation(&directory, &package)
            .context("Önce projenin PHP sürümünü kurun.")?;
        let composer = self.executable("composer")?;
        let ini = self.write_php_config_for(&project.php_version)?;
        let path = dunce::canonicalize(&project.path).context("Proje klasörü bulunamadı.")?;
        let q = |p: &std::path::Path| literal(&p.to_string_lossy());
        let php = q(&directory.join("php.exe"));
        let ext = q(&directory.join("ext"));
        Ok(format!(
            "$ErrorActionPreference = 'Stop'\n$env:PATH = {} + ';' + $env:PATH\n$env:PHPRC = {}\n$env:F4BOX_PHP_EXT = {ext}\n$env:PHP_INI_SCAN_DIR = ''\nfunction global:php {{ & {php} -c {} @args }}\nfunction global:composer {{ & {php} -c {} {} @args }}\nSet-Location -LiteralPath {}\n",
            q(&directory), q(&ini), q(&ini), q(&ini), q(&composer), q(&path)
        ))
    }

    pub fn open_project_terminal(&self, id: &str) -> Result<()> {
        let _guard = self.gate()?;
        let script = self.project_terminal_script(id)?;
        let bytes = script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        let mut cmd = Command::new(powershell_path());
        cmd.args([
            "-NoLogo",
            "-NoProfile",
            "-NoExit",
            "-EncodedCommand",
            &encoded,
        ]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NEW_CONSOLE);
        }
        cmd.spawn().context("Proje terminali açılamadı.")?;
        self.log("Projenin PHP ve Composer terminali açıldı.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn powershell_literals_escape_quotes_without_interpolation() {
        assert_eq!(
            super::literal("C:\\Türkçe O'Brien\\$env:TEMP`x"),
            "'C:\\Türkçe O''Brien\\$env:TEMP`x'"
        );
    }
}
