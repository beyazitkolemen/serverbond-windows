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
        let runtime = vc_runtime();
        checks.push(Requirement::new("vc-runtime", "Visual C++ çalışma zamanı", if runtime { "ok" } else { "error" }, if runtime { "Gerekli x64 çalışma zamanı DLL dosyaları yüklenebiliyor." } else { "Microsoft Visual C++ x64 Redistributable kurulmalı; kurulumdan sonra yeniden denetleyin." }, Some("https://aka.ms/vs/17/release/vc_redist.x64.exe")));
        let writable = tempfile::NamedTempFile::new_in(&self.home)
            .map(|mut file| {
                use std::io::Write;
                file.write_all(b"F4Box write check")?;
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
                        "F4Box tarafından kullanılıyor"
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
                    && ["platform", "vc-runtime", "storage-write"].contains(&r.id.as_str())
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
