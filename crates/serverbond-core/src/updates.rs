//! Public release discovery is independent of the optional signed updater feed.
use anyhow::{bail, Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{io::Read, time::Duration};

pub const REPOSITORY: &str = "beyazitkolemen/serverbond-windows";
pub const RELEASES_URL: &str =
    "https://github.com/beyazitkolemen/serverbond-windows/releases/latest";
pub const RELEASE_API_URL: &str =
    "https://api.github.com/repos/beyazitkolemen/serverbond-windows/releases/latest";
pub const MANIFEST_URL: &str =
    "https://github.com/beyazitkolemen/serverbond-windows/releases/latest/download/latest.json";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseCheck {
    pub available: bool,
    pub current_version: String,
    pub version: String,
    pub notes: String,
    pub release_url: String,
    pub installer_url: Option<String>,
    pub install_mode: &'static str,
    #[serde(skip)]
    pub has_updater: bool,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    body: Option<String>,
    assets: Vec<Asset>,
}

fn validate_release_link(url: &str) -> Result<()> {
    if url == RELEASES_URL {
        return Ok(());
    }
    let prefix = format!("https://github.com/{REPOSITORY}/releases/");
    let path = url
        .strip_prefix(&prefix)
        .context("Güncelleme bağlantısı bu depoya ait değil.")?;
    let parts: Vec<_> = path.split('/').collect();
    let tag = match parts.as_slice() {
        ["tag", tag] | ["download", tag, _] => *tag,
        _ => bail!("Geçersiz güncelleme bağlantısı."),
    };
    let version = Version::parse(tag.strip_prefix('v').unwrap_or(tag))?;
    if !version.pre.is_empty() {
        bail!("Yalnızca kararlı sürüm bağlantıları açılabilir.");
    }
    if parts[0] == "download" && parts[2] != format!("ServerBond_{version}_x64-setup.exe") {
        bail!("Geçersiz kurulum paketi bağlantısı.");
    }
    Ok(())
}

pub fn open_release_link(url: &str) -> Result<()> {
    validate_release_link(url)?;
    #[cfg(windows)]
    {
        crate::process::command("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .context("Güncelleme bağlantısı tarayıcıda açılamadı.")?;
        Ok(())
    }
    #[cfg(not(windows))]
    bail!("Güncelleme bağlantısı Windows masaüstü uygulamasında açılabilir.")
}

pub fn check(current_version: &str) -> Result<ReleaseCheck> {
    let response = reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .user_agent(crate::install::download_user_agent())
        .build()?
        .get(RELEASE_API_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .context("GitHub sürümleri denetlenemedi. İnternet bağlantısını kontrol edin.")?;
    match response.status().as_u16() {
        403 | 429 => {
            bail!("GitHub erişimi sınırlandırdı. Güncellemeyi daha sonra yeniden denetleyin.")
        }
        404 => bail!("serverbond-windows deposunda yayımlanmış bir sürüm bulunamadı."),
        _ => {}
    }
    let response = response
        .error_for_status()
        .context("GitHub sürüm bilgisi alınamadı.")?;
    let mut bytes = Vec::new();
    response.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        bail!("GitHub sürüm yanıtı boyut sınırını aşıyor.");
    }
    parse_release(&bytes, current_version)
}

fn parse_release(bytes: &[u8], current_version: &str) -> Result<ReleaseCheck> {
    let release: Release =
        serde_json::from_slice(bytes).context("GitHub sürüm bilgisi geçersiz.")?;
    let version = Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name),
    )
    .context("GitHub sürüm etiketi geçersiz.")?;
    let current = Version::parse(current_version).context("Kurulu uygulamanın sürümü geçersiz.")?;
    if release.draft || release.prerelease || !version.pre.is_empty() {
        bail!("GitHub yayını kararlı bir sürüm değil.");
    }
    let release_url = format!(
        "https://github.com/{REPOSITORY}/releases/tag/{}",
        release.tag_name
    );
    let base = format!(
        "https://github.com/{REPOSITORY}/releases/download/{}",
        release.tag_name
    );
    let installer = format!("ServerBond_{version}_x64-setup.exe");
    let has_asset = |name: &str| {
        release.assets.iter().any(|asset| {
            asset.name == name && asset.browser_download_url == format!("{base}/{name}")
        })
    };
    let installer_url = has_asset(&installer).then(|| format!("{base}/{installer}"));
    let has_updater = installer_url.is_some()
        && has_asset("latest.json")
        && has_asset(&format!("{installer}.sig"));
    Ok(ReleaseCheck {
        available: version.cmp_precedence(&current).is_gt(),
        current_version: current_version.into(),
        version: version.to_string(),
        notes: release.body.unwrap_or_default(),
        release_url,
        installer_url,
        install_mode: "manual",
        has_updater,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn browser_links_only_allow_this_repositories_releases_and_installers() {
        for url in [RELEASES_URL,
            "https://github.com/beyazitkolemen/serverbond-windows/releases/tag/v1.2.0",
            "https://github.com/beyazitkolemen/serverbond-windows/releases/download/v1.2.0/ServerBond_1.2.0_x64-setup.exe"] {
            validate_release_link(url).unwrap();
        }
        for url in ["file:///C:/Windows/System32/cmd.exe", "https://example.com/",
            "https://github.com/beyazitkolemen/serverbond-windows/releases/tag/v1.2.0?redirect=bad",
            "https://github.com/beyazitkolemen/serverbond-windows/releases/download/v1.2.0/other.exe",
            "https://github.com/beyazitkolemen/serverbond-windows/releases/tag/../../other",
            "https://github.com/beyazitkolemen/serverbond-windows/releases/tag/v1.2.0\n"] {
            assert!(validate_release_link(url).is_err(), "{url}");
        }
    }

    #[test]
    #[ignore = "reads the current public GitHub release over HTTPS"]
    fn live_release_discovery_uses_serverbond_windows() {
        let release = check("1.1.0").unwrap();
        assert!(release
            .release_url
            .starts_with("https://github.com/beyazitkolemen/serverbond-windows/releases/tag/"));
        assert!(release.available);
        assert!(release.installer_url.is_some());
        println!(
            "GitHub release: {}; update from {} available: {}; signed assets: {}",
            release.version, release.current_version, release.available, release.has_updater
        );
    }

    fn release(tag: &str, names: &[&str]) -> Vec<u8> {
        serde_json::to_vec(&json!({"tag_name": tag, "draft":false,"prerelease":false,"body":"notes",
            "assets":names.iter().map(|name| json!({"name":name,"browser_download_url":format!("https://github.com/{REPOSITORY}/releases/download/{tag}/{name}")})).collect::<Vec<_>>()
        })).unwrap()
    }

    #[test]
    fn unsigned_release_is_discovered_without_a_manifest() {
        let result = parse_release(
            &release("v1.2.0", &["ServerBond_1.2.0_x64-setup.exe"]),
            "1.1.0",
        )
        .unwrap();
        assert!(result.available);
        assert!(!result.has_updater);
        assert_eq!(result.install_mode, "manual");
        assert_eq!(result.installer_url.unwrap(), "https://github.com/beyazitkolemen/serverbond-windows/releases/download/v1.2.0/ServerBond_1.2.0_x64-setup.exe");
    }

    #[test]
    fn versions_use_semver_and_never_offer_a_downgrade() {
        for current in ["1.2.0", "1.2.0+local", "1.3.0", "2.0.0"] {
            assert!(
                !parse_release(&release("v1.2.0", &[]), current)
                    .unwrap()
                    .available
            );
        }
        assert!(
            parse_release(&release("v1.10.0", &[]), "1.9.0")
                .unwrap()
                .available
        );
        assert!(
            parse_release(&release("v1.2.0", &[]), "1.2.0-rc.1")
                .unwrap()
                .available
        );
        assert!(parse_release(&release("v1.2.0-rc.1", &[]), "1.1.0").is_err());
        assert!(parse_release(&release("../../other", &[]), "1.1.0").is_err());
    }

    #[test]
    fn automatic_updates_require_matching_assets_from_this_repository() {
        let bytes = release(
            "v1.2.0",
            &[
                "ServerBond_1.2.0_x64-setup.exe",
                "ServerBond_1.2.0_x64-setup.exe.sig",
                "latest.json",
            ],
        );
        assert!(parse_release(&bytes, "1.1.0").unwrap().has_updater);
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["assets"][0]["browser_download_url"] = json!("https://example.com/installer.exe");
        let result = parse_release(&serde_json::to_vec(&value).unwrap(), "1.1.0").unwrap();
        assert!(!result.has_updater);
        assert!(result.installer_url.is_none());
        for field in ["draft", "prerelease"] {
            let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            value[field] = json!(true);
            assert!(parse_release(&serde_json::to_vec(&value).unwrap(), "1.1.0").is_err());
        }
    }

    #[test]
    fn desktop_updater_points_to_the_current_repository() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../../../src-tauri/tauri.conf.json")).unwrap();
        assert_eq!(config["plugins"]["updater"]["endpoints"][0], MANIFEST_URL);
    }
}
