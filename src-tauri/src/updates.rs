//! One release check shared by the window and authenticated HTTP API.
use anyhow::{bail, Context, Result};
use serverbond_core::updates::ReleaseCheck;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

pub fn check(app: &AppHandle) -> Result<ReleaseCheck> {
    let mut release = serverbond_core::updates::check(&app.package_info().version.to_string())?;
    if release.available && release.has_updater {
        let update = tauri::async_runtime::block_on(
            app.updater_builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()?
                .check(),
        )?
        .context("GitHub yayını ile imzalı güncelleme bildirimi eşleşmiyor.")?;
        if update.version != release.version {
            bail!("GitHub yayını ile imzalı güncelleme sürümü eşleşmiyor. Daha sonra yeniden denetleyin.");
        }
        release.install_mode = "automatic";
    }
    Ok(release)
}
