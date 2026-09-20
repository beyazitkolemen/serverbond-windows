#[cfg(any(test, windows))]
use crate::portable_path;
use crate::Manager;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub fn local_ca_candidates(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join("data/caddy/pki/authorities/local/root.crt"),
        home.join("config/Caddy/pki/authorities/local/root.crt"),
        home.join("config/caddy/pki/authorities/local/root.crt"),
    ]
}

pub fn find_local_ca(home: &Path) -> Option<PathBuf> {
    local_ca_candidates(home)
        .into_iter()
        .find(|path| path.is_file())
}

pub fn trust_marker_path(home: &Path) -> PathBuf {
    home.join("config/https-trusted.pem")
}

pub fn already_trusted(cert: &[u8], marker: &[u8]) -> bool {
    !cert.is_empty() && cert == marker
}

pub fn sha1_from_certutil_dump(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line
            .strip_prefix("Cert Hash(sha1):")
            .or_else(|| line.strip_prefix("Cert Hash(sha1) :"))
        else {
            continue;
        };
        let hex: String = rest.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if hex.len() == 40 {
            return Some(hex.to_ascii_lowercase());
        }
    }
    None
}

/// Current-user Root store. No UAC; Chrome/Edge pick this store up.
#[cfg(any(test, windows))]
pub fn user_trust_args(cert: &Path) -> Vec<String> {
    vec![
        "-user".into(),
        "-addstore".into(),
        "-f".into(),
        "Root".into(),
        portable_path(cert).replace('/', "\\"),
    ]
}

#[cfg(any(test, windows))]
pub fn user_untrust_args() -> Vec<String> {
    vec![
        "-user".into(),
        "-delstore".into(),
        "Root".into(),
        "Caddy Local Authority".into(),
    ]
}

impl Manager {
    pub fn https_certificate(&self) -> Option<PathBuf> {
        find_local_ca(&self.home)
    }

    pub fn trust_https(&self) -> Result<()> {
        let cert = find_local_ca(&self.home)
            .context("Yerel HTTPS sertifikası henüz yok. HTTPS açıkken ortamı bir kez başlatın.")?;
        self.trust_certificate(&cert)
    }

    pub(crate) fn try_trust_https(&self) {
        let cert = self.wait_for_local_ca();
        let Some(cert) = cert else {
            self.log("Yerel HTTPS hazır. Tarayıcı uyarısı görürseniz Ayarlar → Web sunucusu ekranından sertifikayı güven deposuna ekleyin.");
            return;
        };
        if self.marker_matches(&cert) {
            return;
        }
        if let Err(error) = self.trust_certificate(&cert) {
            self.log(format!(
                "Yerel HTTPS sertifikası güven deposuna eklenemedi: {error:#}. Ayarlar → Web sunucusu ekranından yeniden deneyin."
            ));
        }
    }

    fn wait_for_local_ca(&self) -> Option<PathBuf> {
        for _ in 0..15 {
            if let Some(path) = find_local_ca(&self.home) {
                return Some(path);
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        find_local_ca(&self.home)
    }

    fn marker_matches(&self, cert: &Path) -> bool {
        let Ok(current) = std::fs::read(cert) else {
            return false;
        };
        let Ok(marker) = std::fs::read(trust_marker_path(&self.home)) else {
            return false;
        };
        already_trusted(&current, &marker)
    }

    #[cfg(windows)]
    fn mark_trusted(&self, cert: &Path) -> Result<()> {
        crate::storage::atomic_write(&trust_marker_path(&self.home), std::fs::read(cert)?)?;
        Ok(())
    }

    #[cfg(windows)]
    fn clear_trust_marker(&self) {
        let _ = std::fs::remove_file(trust_marker_path(&self.home));
    }

    fn trust_certificate(&self, cert: &Path) -> Result<()> {
        #[cfg(windows)]
        {
            use crate::process::{command, ManagedChild};
            use std::time::Duration;
            let mut cmd = command("certutil");
            cmd.args(user_trust_args(cert));
            let output = ManagedChild::output(cmd, Duration::from_secs(30))?;
            if !output.status.success() {
                bail!(
                    "Sertifika kullanıcı güven deposuna eklenemedi: {}",
                    String::from_utf8_lossy(&output.stderr)
                        .chars()
                        .chain(String::from_utf8_lossy(&output.stdout).chars())
                        .take(400)
                        .collect::<String>()
                );
            }
            self.mark_trusted(cert)?;
            self.log("Yerel HTTPS sertifikası kullanıcı güven deposuna eklendi.");
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = cert;
            bail!("Sertifika güveni Windows kullanıcı deposuna yazılır.");
        }
    }

    pub fn untrust_https(&self) -> Result<()> {
        #[cfg(windows)]
        {
            use crate::process::{command, ManagedChild};
            use std::time::Duration;
            if let Some(cert) = find_local_ca(&self.home) {
                let mut dump = command("certutil");
                dump.arg("-dump")
                    .arg(portable_path(&cert).replace('/', "\\"));
                if let Ok(output) = ManagedChild::output(dump, Duration::from_secs(15)) {
                    let text = format!(
                        "{}{}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    );
                    if let Some(hash) = sha1_from_certutil_dump(&text) {
                        let mut cmd = command("certutil");
                        cmd.args(["-user", "-delstore", "Root", &hash]);
                        let removed = ManagedChild::output(cmd, Duration::from_secs(30))?;
                        if removed.status.success() {
                            self.clear_trust_marker();
                            self.log(
                                "Yerel HTTPS sertifikası kullanıcı güven deposundan kaldırıldı.",
                            );
                            return Ok(());
                        }
                    }
                }
            }
            let mut cmd = command("certutil");
            cmd.args(user_untrust_args());
            let output = ManagedChild::output(cmd, Duration::from_secs(30))?;
            if !output.status.success() {
                bail!(
                    "Sertifika güven deposundan kaldırılamadı: {}",
                    String::from_utf8_lossy(&output.stderr)
                        .chars()
                        .take(400)
                        .collect::<String>()
                );
            }
            self.clear_trust_marker();
            self.log("Yerel HTTPS sertifikası kullanıcı güven deposundan kaldırıldı.");
            Ok(())
        }
        #[cfg(not(windows))]
        {
            bail!("Sertifika güveni Windows kullanıcı deposundan kaldırılır.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ca_candidates_stay_under_the_data_directory() {
        let home = Path::new("C:/F4Box Home");
        let paths = local_ca_candidates(home);
        assert!(paths.iter().all(|path| path.starts_with(home)));
        assert!(paths.iter().all(|path| path.ends_with("root.crt")));
    }

    #[test]
    fn user_store_trust_does_not_touch_the_machine_root() {
        let args = user_trust_args(Path::new(
            "C:/F4Box/data/caddy/pki/authorities/local/root.crt",
        ));
        assert_eq!(args[0], "-user");
        assert!(args.contains(&"Root".into()));
        assert!(!args
            .iter()
            .any(|arg| arg.eq_ignore_ascii_case("-enterprise")));
        assert_eq!(user_untrust_args()[0], "-user");
    }

    #[test]
    fn trust_marker_compares_the_certificate_bytes() {
        assert!(already_trusted(b"pem", b"pem"));
        assert!(!already_trusted(b"pem", b"other"));
        assert!(!already_trusted(b"", b""));
    }

    #[test]
    fn certutil_dump_hash_is_normalized() {
        let dump = "Subject: CN=Caddy Local Authority\nCert Hash(sha1): 12 34 56 78 90 ab cd ef 01 23 45 67 89 ab cd ef 01 23 45 67\n";
        assert_eq!(
            sha1_from_certutil_dump(dump).as_deref(),
            Some("1234567890abcdef0123456789abcdef01234567")
        );
        assert!(sha1_from_certutil_dump("no hash here").is_none());
    }
}
