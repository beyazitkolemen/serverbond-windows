use anyhow::{Context, Result};
use std::path::Path;

#[cfg(windows)]
fn transform(input: &[u8], protect: bool) -> Result<Vec<u8>> {
    use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};
    unsafe {
        let source = CRYPT_INTEGER_BLOB {
            cbData: input.len() as u32,
            pbData: input.as_ptr() as *mut u8,
        };
        let mut output: CRYPT_INTEGER_BLOB = std::mem::zeroed();
        let ok = if protect {
            CryptProtectData(
                &source,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &source,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let result = if output.cbData == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec()
        };
        LocalFree(output.pbData as *mut _);
        Ok(result)
    }
}

#[cfg(not(windows))]
fn transform(_: &[u8], _: bool) -> Result<Vec<u8>> {
    anyhow::bail!("Parola saklama yalnızca Windows DPAPI ile desteklenir.")
}

pub fn save(path: &Path, value: &str) -> Result<()> {
    crate::storage::atomic_write(path, transform(value.as_bytes(), true)?)
        .context("Parola şifrelenerek kaydedilemedi.")
}

pub fn read(path: &Path) -> Result<String> {
    String::from_utf8(transform(
        &crate::storage::read_limited(path, 64 * 1024).context("MySQL parolası bulunamadı.")?,
        false,
    )?)
    .context("Parola çözülemedi.")
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn password_is_encrypted_and_roundtrips_for_current_user() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("password.dpapi");
        let password = uuid::Uuid::new_v4().simple().to_string();
        super::save(&path, &password).unwrap();
        let encrypted = std::fs::read(&path).unwrap();
        assert!(!encrypted
            .windows(password.len())
            .any(|w| w == password.as_bytes()));
        assert_eq!(super::read(&path).unwrap(), password);
    }
}
