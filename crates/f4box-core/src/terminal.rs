//! "Open terminal here" for a project: a Windows Terminal / PowerShell
//! session whose PATH contains the project's PHP, Composer and optional
//! Node, built from safely quoted PowerShell literals.

use crate::{install, model::php_package, Manager};
use anyhow::{Context, Result};
use base64::Engine;
use std::{path::PathBuf, process::Command};

/// A PowerShell single-quoted literal. Besides the ASCII apostrophe, the
/// tokenizer also ends such a string at the typographic quotes U+2018–U+201B,
/// so a folder like `C:\Users\O’Brien` must be spliced in as a character code.
pub(crate) fn literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for c in value.chars() {
        match c {
            '\'' => out.push_str("''"),
            '\u{2018}'..='\u{201F}' => {
                out.push_str(&format!("'+[char]0x{:04X}+'", c as u32));
            }
            _ => out.push(c),
        }
    }
    out.push('\'');
    out
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
        // Node is optional; its npm and npx wrappers only join PATH once installed.
        let mut path_dirs = vec![q(&directory)];
        if let Ok(node) = self.tool_directory(crate::node::ID) {
            path_dirs.push(q(&node));
        }
        if let Ok(postgres) = self.tool_executable(crate::postgres::ID) {
            if let Some(bin) = postgres.parent() {
                path_dirs.push(q(bin));
            }
        }
        if let Ok(redis) = self.tool_executable(crate::redis::ID) {
            if let Some(bin) = redis.parent() {
                path_dirs.push(q(bin));
            }
        }
        let prefix = path_dirs.join(" + ';' + ");
        Ok(format!(
            "$ErrorActionPreference = 'Stop'\n$env:PATH = {prefix} + ';' + $env:PATH\n$env:PHPRC = {}\n$env:F4BOX_PHP_EXT = {ext}\n$env:PHP_INI_SCAN_DIR = ''\nfunction global:php {{ & {php} -c {} @args }}\nfunction global:composer {{ & {php} -c {} {} @args }}\nSet-Location -LiteralPath {}\n",
            q(&ini), q(&ini), q(&ini), q(&composer), q(&path)
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
