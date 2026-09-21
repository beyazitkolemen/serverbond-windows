//! Windows startup registration through `HKCU\…\Run`: read, write and
//! remove the autostart command that launches ServerBond with `--autostart`.

use anyhow::{bail, Context, Result};
use std::path::Path;
use winreg::{enums::*, RegKey, RegValue};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const NAME: &str = "ServerBond";
const LEGACY_NAME: &str = "F4Box";

pub struct Registration {
    run_key: String,
    approved_key: String,
    run: Option<RegValue>,
    approved: Option<RegValue>,
}

fn read_named(path: &str, name: &str) -> Result<Option<RegValue>> {
    match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(path)
        .and_then(|key| key.get_raw_value(name))
    {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("Windows başlangıç ayarı okunamadı."),
    }
}

fn read(path: &str) -> Result<Option<RegValue>> {
    match read_named(path, NAME)? {
        Some(value) => Ok(Some(value)),
        None => read_named(path, LEGACY_NAME),
    }
}

fn write_named(path: &str, name: &str, value: Option<&RegValue>) -> Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(path)?;
    if let Some(value) = value {
        key.set_raw_value(name, value)?;
    } else if let Err(error) = key.delete_value(name) {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error.into());
        }
    }
    Ok(())
}

fn write(path: &str, value: Option<&RegValue>) -> Result<()> {
    write_named(path, NAME, value)
}

impl Registration {
    pub fn read() -> Result<Self> {
        Self::read_at(RUN, APPROVED)
    }
    fn read_at(run_key: &str, approved_key: &str) -> Result<Self> {
        Ok(Self {
            run_key: run_key.into(),
            approved_key: approved_key.into(),
            run: read(run_key)?,
            approved: read(approved_key)?,
        })
    }
    pub fn enabled(&self) -> bool {
        self.run.is_some()
            && !self
                .approved
                .as_ref()
                .is_some_and(|v| matches!(v.bytes.first(), Some(3 | 7)))
    }
    pub fn needs_change(&self, enabled: bool, command: &str) -> bool {
        if !enabled {
            return self.run.is_some();
        }
        use winreg::types::FromRegValue;
        !self.enabled()
            || self
                .run
                .as_ref()
                .and_then(|v| String::from_reg_value(v).ok())
                .as_deref()
                != Some(command)
    }
    pub fn restore(&self) -> Result<()> {
        // Attempt both even if one key is inaccessible.
        let run = write(&self.run_key, self.run.as_ref());
        let approved = write(&self.approved_key, self.approved.as_ref());
        run.and(approved)
    }
    pub fn apply(&self, enabled: bool, command: &str) -> Result<()> {
        let result = (|| {
            if enabled {
                if command.encode_utf16().count() > 260 {
                    bail!("Windows başlangıç komutu çok uzun. ServerBond ve veri klasörünü daha kısa bir konumda kullanın.");
                }
                let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(&self.run_key)?;
                key.set_value(NAME, &command)?;
                let mut bytes = vec![0; 12];
                bytes[0] = 2;
                write(
                    &self.approved_key,
                    Some(&RegValue {
                        vtype: REG_BINARY,
                        bytes,
                    }),
                )?;
            } else {
                write(&self.run_key, None)?;
                write(&self.approved_key, None)?;
            }
            let _ = write_named(&self.run_key, LEGACY_NAME, None);
            let _ = write_named(&self.approved_key, LEGACY_NAME, None);
            Ok(())
        })();
        if result.is_err() {
            self.restore().context("Başlangıç kaydı değiştirilemedi ve eski kayıt geri yüklenemedi; Windows Başlangıç Uygulamaları'nı kontrol edin.")?;
        }
        result
    }
}

pub fn command(exe: &Path, home: &Path) -> String {
    format!(
        "{} --autostart --data-home {}",
        quote(&exe.to_string_lossy()),
        quote(&home.to_string_lossy())
    )
}

// Windows command-line quoting, including trailing backslashes in drive roots.
fn quote(value: &str) -> String {
    let mut result = String::from("\"");
    let mut slashes = 0;
    for c in value.chars() {
        if c == '\\' {
            slashes += 1;
            continue;
        }
        result.extend(std::iter::repeat_n(
            '\\',
            if c == '"' { slashes * 2 + 1 } else { slashes },
        ));
        result.push(c);
        slashes = 0;
    }
    result.extend(std::iter::repeat_n('\\', slashes * 2));
    result.push('"');
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registration_enable_disable_and_exact_rollback_in_non_startup_test_keys() {
        // No Windows startup keys are changed: these are inert, disposable HKCU test keys.
        let name = format!(
            r"Software\ServerBondTests\registration-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let run = format!(r"{name}\Command");
        let approved = format!(r"{name}\Approval");
        let old = Registration::read_at(&run, &approved).unwrap();
        assert!(!old.enabled());
        assert!(!old.needs_change(false, "unused"));
        let command = command(
            Path::new(r"C:\Program Files\ServerBond\ServerBond.exe"),
            Path::new(r"C:\Türkçe Test"),
        );
        old.apply(true, &command).unwrap();
        let enabled = Registration::read_at(&run, &approved).unwrap();
        assert!(enabled.enabled());
        assert!(!enabled.needs_change(true, &command));
        assert!(enabled.needs_change(true, "different executable"));
        let original_run = enabled.run.as_ref().unwrap().bytes.clone();
        let original_approval = enabled.approved.as_ref().unwrap().bytes.clone();
        enabled.apply(false, &command).unwrap();
        assert!(!Registration::read_at(&run, &approved).unwrap().enabled());
        enabled.restore().unwrap();
        let restored = Registration::read_at(&run, &approved).unwrap();
        assert_eq!(restored.run.unwrap().bytes, original_run);
        assert_eq!(restored.approved.unwrap().bytes, original_approval);
        assert!(enabled.apply(true, &"x".repeat(261)).is_err());
        assert!(Registration::read_at(&run, &approved).unwrap().enabled());
        old.restore().unwrap();
        assert!(!Registration::read_at(&run, &approved).unwrap().enabled());
        RegKey::predef(HKEY_CURRENT_USER)
            .delete_subkey_all(&name)
            .unwrap();
    }
    #[test]
    fn startup_command_roundtrips_spaces_unicode_and_drive_root() {
        for home in [r"C:\Türkçe O'Brien\F4 Box", r"C:\"] {
            let exe = r"C:\Program Files\ServerBond\ServerBond.exe";
            let text = command(Path::new(exe), Path::new(home));
            let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
            unsafe {
                let mut count = 0;
                let args =
                    windows_sys::Win32::UI::Shell::CommandLineToArgvW(wide.as_ptr(), &mut count);
                assert!(!args.is_null());
                assert_eq!(count, 4);
                let values: Vec<String> = std::slice::from_raw_parts(args, count as usize)
                    .iter()
                    .map(|p| {
                        let mut len = 0;
                        while *p.add(len) != 0 {
                            len += 1;
                        }
                        String::from_utf16_lossy(std::slice::from_raw_parts(*p, len))
                    })
                    .collect();
                windows_sys::Win32::Foundation::LocalFree(args.cast());
                assert_eq!(values, [exe, "--autostart", "--data-home", home]);
            }
        }
    }
}
