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
        if let Some(cert) = find_local_ca(&self.home) {
            if let Err(error) = self.trust_certificate(&cert) {
                self.log(format!(
                    "Yerel HTTPS sertifikası güven deposuna eklenemedi: {error:#}. Ayarlar → Web sunucusu ekranından yeniden deneyin."
                ));
            }
        } else {
            self.log("Yerel HTTPS hazır. Tarayıcı uyarısı görürseniz Ayarlar → Web sunucusu ekranından sertifikayı güven deposuna ekleyin.");
        }
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
}
