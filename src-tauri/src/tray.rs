use crate::{
    desktop::{self, Desktop},
    State,
};
use anyhow::{Context, Result};
use std::sync::atomic::Ordering;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager as _, Wry,
};

pub struct TrayMenu {
    pub menu: Menu<Wry>,
    status: MenuItem<Wry>,
    start: MenuItem<Wry>,
    stop: MenuItem<Wry>,
    restart: MenuItem<Wry>,
    pma: MenuItem<Wry>,
    mail: MenuItem<Wry>,
    quit: MenuItem<Wry>,
    autostart: CheckMenuItem<Wry>,
    close: CheckMenuItem<Wry>,
    services: Vec<(String, MenuItem<Wry>, MenuItem<Wry>)>,
}

pub fn setup(app: &AppHandle) -> Result<()> {
    let item = |id: &str, text: &str| MenuItem::with_id(app, id, text, true, None::<&str>);
    let open = item("open", "F4Box'ı aç")?;
    let status = item("status", "Ortam denetleniyor…")?;
    status.set_enabled(false)?;
    let start = item("start", "Tümünü başlat")?;
    let stop = item("stop", "Tümünü durdur")?;
    let restart = item("restart", "Tümünü yeniden başlat")?;
    let pma = item("pma", "phpMyAdmin'i aç")?;
    let mail = item("mail", "Gelen kutusunu aç")?;
    let settings = item("settings", "Ayarlar / Özellikler")?;
    let update = item("update", "Güncellemeleri denetle")?;
    let logs = item("logs", "Günlükleri göster")?;
    let folder = item("folder", "Veri klasörünü aç")?;
    let quit = item("quit", "Çıkış — servisleri durdur")?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Windows ile başlat",
        true,
        false,
        None::<&str>,
    )?;
    let close = CheckMenuItem::with_id(
        app,
        "close-to-tray",
        "Pencereyi kapatınca tepsiye küçült",
        true,
        true,
        None::<&str>,
    )?;
    let separator = || PredefinedMenuItem::separator(app);
    let services_menu = Submenu::new(app, "Servisler", true)?;
    let mut services = Vec::new();
    for (id, name) in [
        ("php", "PHP"),
        ("mysql", "MySQL"),
        ("caddy", "Web sunucusu"),
        ("cloudflared", "Cloudflare tüneli"),
    ] {
        let start = item(&format!("start:{id}"), "Başlat")?;
        let stop = item(&format!("stop:{id}"), "Durdur")?;
        services_menu.append(&Submenu::with_items(app, name, true, &[&start, &stop])?)?;
        services.push((id.into(), start, stop));
    }
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &status,
            &separator()?,
            &start,
            &stop,
            &restart,
            &services_menu,
            &separator()?,
            &pma,
            &mail,
            &settings,
            &update,
            &logs,
            &folder,
            &separator()?,
            &autostart,
            &close,
            &separator()?,
            &quit,
        ],
    )?;
    let icon = app
        .default_window_icon()
        .context("Tepsi simgesi bulunamadı.")?
        .clone();
    TrayIconBuilder::with_id("f4box-tray")
        .icon(icon)
        .tooltip("F4Box · Windows Laravel üretimi")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| dispatch(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                desktop::show(tray.app_handle(), None);
            }
        })
        .build(app)?;
    app.manage(TrayMenu {
        menu,
        status,
        start,
        stop,
        restart,
        pma,
        mail,
        quit,
        autostart,
        close,
        services,
    });
    app.state::<Desktop>()
        .tray_available
        .store(true, Ordering::Release);
    let app = app.clone();
    // Rust keeps status current even when Windows suspends hidden webview timers.
    std::thread::spawn(move || {
        while !app.state::<Desktop>().exit_ready.load(Ordering::Acquire) {
            let manager = app.state::<State>();
            if let Ok(snapshot) = manager.contain(|| manager.snapshot()) {
                let _ = refresh(&app, &snapshot);
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
    Ok(())
}

fn refresh(app: &AppHandle, snapshot: &f4box_core::model::Snapshot) -> Result<()> {
    let menu = app.state::<TrayMenu>();
    let desktop = app.state::<Desktop>().status();
    let busy = snapshot.busy || desktop.quitting;
    let healthy = snapshot.recovery_issue.is_none();
    let ready = ["php", "mysql", "caddy"].iter().all(|id| {
        snapshot
            .packages
            .iter()
            .any(|p| p.package.id == *id && p.installed)
    });
    let running = snapshot.packages.iter().filter(|p| p.running).count();
    let status = if desktop.quitting {
        "F4Box kapanıyor…".into()
    } else if !healthy {
        "Kurtarma gerekiyor — F4Box'ı açın".into()
    } else if busy {
        "İşlem devam ediyor…".into()
    } else if running == 3 {
        "Ortam çalışıyor".into()
    } else if snapshot.any_running {
        format!("Ortam kısmen çalışıyor ({running}/3)")
    } else {
        "Ortam durduruldu".into()
    };
    menu.status.set_text(&status)?;
    if let Some(tray) = app.tray_by_id("f4box-tray") {
        tray.set_tooltip(Some(format!("F4Box · {status}")))?;
    }
    menu.start
        .set_enabled(!busy && healthy && ready && running < 3)?;
    menu.stop.set_enabled(!busy && snapshot.any_running)?;
    menu.restart.set_enabled(!busy && healthy && ready)?;
    menu.quit.set_enabled(!busy)?;
    menu.pma.set_enabled(
        !busy
            && healthy
            && running == 3
            && snapshot.settings.phpmyadmin.enabled
            && snapshot
                .packages
                .iter()
                .any(|p| p.package.id == "phpmyadmin" && p.installed),
    )?;
    menu.mail
        .set_enabled(!busy && healthy && snapshot.mail.running)?;
    menu.autostart
        .set_enabled(!desktop.quitting && desktop.autostart.is_some())?;
    menu.autostart
        .set_checked(desktop.autostart.unwrap_or(false))?;
    menu.close.set_enabled(!desktop.quitting)?;
    menu.close.set_checked(desktop.preferences.close_to_tray)?;
    for (id, start, stop) in &menu.services {
        if id == "cloudflared" {
            start.set_enabled(
                !busy && healthy && snapshot.tunnel.token_saved && !snapshot.tunnel.running,
            )?;
            stop.set_enabled(!busy && snapshot.tunnel.running)?;
            continue;
        }
        if let Some(package) = snapshot.packages.iter().find(|p| p.package.id == *id) {
            start.set_enabled(!busy && healthy && package.installed && !package.running)?;
            stop.set_enabled(!busy && package.running)?;
        }
    }
    Ok(())
}

fn dispatch(app: &AppHandle, id: &str) {
    match id {
        "open" => desktop::show(app, None),
        "settings" => desktop::show(app, Some("settings")),
        "update" => {
            desktop::show(app, Some("settings"));
            let _ = app.emit("desktop:check-update", ());
        }
        "logs" => desktop::show(app, Some("logs")),
        "quit" => desktop::request_exit(app),
        "status" => {}
        _ => {
            let app = app.clone();
            let id = id.to_string();
            tauri::async_runtime::spawn_blocking(move || {
                let manager = app.state::<State>();
                let result = manager.contain(|| match id.as_str() {
                    "start" => manager.start("all"),
                    "stop" => manager.stop("all"),
                    "restart" => manager.restart(),
                    "pma" => manager.open_phpmyadmin(),
                    "mail" => manager.open_mail(),
                    "folder" => manager.open_home(),
                    "autostart" | "close-to-tray" => {
                        let desktop = app.state::<Desktop>();
                        let mut status = desktop.status();
                        let enabled = status
                            .autostart
                            .context("Windows başlangıç ayarı okunamadı.")?;
                        if id == "close-to-tray" {
                            status.preferences.close_to_tray = !status.preferences.close_to_tray;
                        }
                        desktop.save(
                            status.preferences,
                            if id == "autostart" { !enabled } else { enabled },
                            &manager.home,
                        )
                    }
                    other if other.starts_with("start:") => manager.start(&other[6..]),
                    other if other.starts_with("stop:") => manager.stop(&other[5..]),
                    _ => Ok(()),
                });
                if let Err(error) = result {
                    desktop::report(&app, format!("Tepsi işlemi tamamlanamadı: {error:#}"));
                }
                if let Ok(snapshot) = manager.snapshot() {
                    let _ = refresh(&app, &snapshot);
                }
            });
        }
    }
}
