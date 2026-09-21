//! Windows prerequisites (x64, Visual C++ runtime, WebView2, free disk,
//! reserved ports) reported before installation so failures are explained
//! up front.

use crate::Manager;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
    pub help_url: Option<String>,
}

impl Requirement {
    fn new(
        id: &str,
        label: &str,
        status: &str,
        detail: impl Into<String>,
        help: Option<&str>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            status: status.into(),
            detail: detail.into(),
            help_url: help.map(str::to_owned),
        }
    }
}

#[derive(Clone, Copy)]
struct WindowsVersion {
    major: u32,
    minor: u32,
    build: u32,
}

#[cfg(windows)]
fn windows_version() -> Option<WindowsVersion> {
    use windows_sys::{
        Wdk::System::SystemServices::RtlGetVersion,
        Win32::System::SystemInformation::OSVERSIONINFOW,
    };
    // RtlGetVersion reports the installed OS rather than the version implied
    // by a compatibility manifest. The buffer remains valid throughout the call.
    let mut version: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
    version.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
    let result = unsafe { RtlGetVersion(&mut version) };
    (result >= 0).then_some(WindowsVersion {
        major: version.dwMajorVersion,
        minor: version.dwMinorVersion,
        build: version.dwBuildNumber,
    })
}

#[cfg(not(windows))]
fn windows_version() -> Option<WindowsVersion> {
    None
}

fn windows_requirement(version: Option<WindowsVersion>) -> Requirement {
    let (status, detail) = match version {
        Some(v) if v.major >= 10 => (
            "ok",
            format!("Windows NT {}.{} · derleme {}. Temel işletim sistemi gereksinimi karşılanıyor; bileşen desteği ayrıca değerlendirilir.", v.major, v.minor, v.build),
        ),
        Some(v) => (
            "error",
            format!("Windows NT {}.{} · derleme {}. Windows 10 / Server 2016 veya üzeri gerekir.", v.major, v.minor, v.build),
        ),
        None => (
            "error",
            "Windows sürümü doğrulanamadı. Windows 10 / Server 2016 veya üzeri gerekir.".into(),
        ),
    };
    Requirement::new("windows-version", "Windows sürümü", status, detail, None)
}

#[cfg(windows)]
fn vc_runtime() -> bool {
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32};
    ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"]
        .iter()
        .all(|dll| {
            let name = dll.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
            unsafe {
                let handle = LoadLibraryExW(
                    name.as_ptr(),
                    std::ptr::null_mut(),
                    LOAD_LIBRARY_SEARCH_SYSTEM32,
                );
                if handle.is_null() {
                    false
                } else {
                    FreeLibrary(handle);
                    true
                }
            }
        })
}

#[cfg(not(windows))]
fn vc_runtime() -> bool {
    false
}

impl Manager {
    pub fn requirements(&self) -> Vec<Requirement> {
        let mut checks = Vec::new();
        let recovery = self.recovery_issue();
        checks.push(Requirement::new(
            "state",
            "Uygulama durumu",
            if recovery.is_some() { "error" } else { "ok" },
            recovery.unwrap_or_else(|| "Yapılandırma geçerli; iç hata algılanmadı.".into()),
            None,
        ));
        let mysql = self.check_mysql_data();
        checks.push(Requirement::new(
            "mysql-data",
            "MySQL veri ve parola tutarlılığı",
            if mysql.is_ok() { "ok" } else { "error" },
            mysql.err().map_or_else(
                || "Veri ve parola dosyalarında bilinen bir tutarsızlık yok.".into(),
                |e| format!("{e:#}"),
            ),
            None,
        ));
        let supported = cfg!(all(windows, target_arch = "x86_64"));
        checks.push(Requirement::new(
            "platform",
            "Windows x64",
            if supported { "ok" } else { "error" },
            format!("{} / {}", std::env::consts::OS, std::env::consts::ARCH),
            None,
        ));
        checks.push(windows_requirement(windows_version()));
        let runtime = vc_runtime();
        checks.push(Requirement::new("vc-runtime", "Visual C++ çalışma zamanı", if runtime { "ok" } else { "error" }, if runtime { "Gerekli x64 çalışma zamanı DLL dosyaları yüklenebiliyor." } else { "Microsoft Visual C++ x64 Redistributable kurulmalı; kurulumdan sonra yeniden denetleyin." }, Some("https://aka.ms/vs/17/release/vc_redist.x64.exe")));
        let writable = tempfile::NamedTempFile::new_in(&self.home)
            .map(|mut file| {
                use std::io::Write;
                file.write_all(b"ServerBond write check")?;
                file.as_file().sync_all()
            })
            .and_then(|r| r);
        checks.push(Requirement::new(
            "storage-write",
            "Veri klasörüne yazma",
            if writable.is_ok() { "ok" } else { "error" },
            writable
                .err()
                .map_or_else(|| self.home.display().to_string(), |e| e.to_string()),
            None,
        ));
        match fs2::available_space(&self.home) {
            Ok(bytes) => checks.push(Requirement::new("disk", "Boş disk alanı", if bytes >= 3 * 1024 * 1024 * 1024 { "ok" } else { "warning" }, format!("{:.1} GB kullanılabilir. Tam kurulum ve arşivler için en az 3 GB önerilir; proje ve veritabanları ek alan kullanır.", bytes as f64 / 1_073_741_824.0), None)),
            Err(error) => checks.push(Requirement::new("disk", "Boş disk alanı", "warning", error.to_string(), None)),
        }
        let shell = crate::terminal::powershell_path().is_file();
        checks.push(Requirement::new(
            "terminal",
            "Windows PowerShell",
            if shell { "ok" } else { "warning" },
            if shell {
                "Proje terminali kullanılabilir."
            } else {
                "Windows PowerShell bulunamadı; proje terminali açılamaz."
            },
            None,
        ));
        let config = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let mut processes = self.processes.lock().unwrap_or_else(|e| e.into_inner());
        let mut ports = vec![
            ("php", "Varsayılan PHP portu", config.settings.php_port),
            ("mysql", "MySQL portu", config.settings.mysql_port),
            ("caddy", "Web sunucusu portu", config.settings.web_port),
        ];
        if config.settings.web.https {
            ports.push((
                "caddy-https",
                "Yerel HTTPS portu",
                config.settings.web.https_port,
            ));
        }
        for (id, label, port) in ports {
            let owned = processes.get_mut(id).is_some_and(|p| p.alive());
            let responsive = !owned
                || std::net::TcpStream::connect_timeout(
                    &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                    std::time::Duration::from_millis(300),
                )
                .is_ok();
            let free =
                (owned && responsive) || (!owned && crate::services::port_free(port).is_ok());
            checks.push(Requirement::new(
                id,
                label,
                if free { "ok" } else { "error" },
                format!(
                    "127.0.0.1:{port} — {}",
                    if owned && !responsive {
                        "süreç açık ancak port yanıt vermiyor; günlükleri kontrol edin"
                    } else if owned {
                        "ServerBond tarafından kullanılıyor"
                    } else if free {
                        "kullanılabilir"
                    } else {
                        "başka bir uygulama kullanıyor; Ayarlar'dan değiştirin"
                    }
                ),
                None,
            ));
        }
        checks
    }

    pub(crate) fn check_install_requirements(&self) -> anyhow::Result<()> {
        let errors = self
            .requirements()
            .into_iter()
            .filter(|r| {
                r.status == "error"
                    && ["platform", "windows-version", "vc-runtime", "storage-write"]
                        .contains(&r.id.as_str())
            })
            .map(|r| format!("{}: {}", r.label, r.detail))
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            anyhow::bail!(
                "Kurulum gereksinimleri karşılanmıyor: {}",
                errors.join("; ")
            );
        }
        Ok(())
    }

    pub fn open_runtime_download(&self) -> anyhow::Result<()> {
        crate::process::command("rundll32.exe")
            .args([
                "url.dll,FileProtocolHandler",
                "https://aka.ms/vs/17/release/vc_redist.x64.exe",
            ])
            .spawn()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_pre_windows_10_releases() {
        for (minor, build) in [(1, 7601), (2, 9200), (3, 9600)] {
            let check = windows_requirement(Some(WindowsVersion {
                major: 6,
                minor,
                build,
            }));
            assert_eq!(check.status, "error");
            assert!(check.detail.contains("Windows 10 / Server 2016"));
        }
    }

    #[test]
    fn accepts_nt_10_client_and_server_builds() {
        // Win 10 RTM, Server 2016/2019/2022, Win 10 22H2, Win 11/Server 2025.
        for build in [10240, 14393, 17763, 20348, 19045, 22000, 26100] {
            let check = windows_requirement(Some(WindowsVersion {
                major: 10,
                minor: 0,
                build,
            }));
            assert_eq!(check.status, "ok", "build {build}");
            assert!(check.detail.contains(&build.to_string()));
        }
    }

    #[test]
    fn unknown_windows_version_is_not_reported_as_compatible() {
        assert_eq!(windows_requirement(None).status, "error");
    }

    #[cfg(windows)]
    #[test]
    fn reads_actual_windows_version() {
        let version = windows_version().expect("RtlGetVersion failed");
        assert!(version.major > 0);
        assert!(version.build > 0);
    }
}
