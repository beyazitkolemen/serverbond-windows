//! Tauri entry point. Every `#[tauri::command]` is a thin adapter: parse the
//! action string into a domain enum, run the `Manager` call on a blocking
//! thread inside `contain`, and return the error text the interface shows.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod appearance;
mod desktop;
mod startup;
mod tray;

use std::sync::atomic::Ordering;
use tauri::Manager as _;

use serverbond_core::{
    model::{
        DiscoveredProject, Project, ProjectRelease, ProjectSchedule, QueueWorker, Settings,
        Snapshot,
    },
    EnvironmentAction, GithubAction, Manager, ProjectEnv, ProjectGitStatus, ReleaseRecord,
    ToolAction,
};
use std::{path::PathBuf, sync::Arc};

type State = Arc<Manager>;

async fn blocking<T: Send + 'static>(
    manager: State,
    work: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || manager.contain(work))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn snapshot(state: tauri::State<'_, State>) -> Result<Snapshot, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.snapshot()).await
}

#[tauri::command]
async fn install(state: tauri::State<'_, State>, id: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.install(&id)).await
}
#[tauri::command]
async fn repair(state: tauri::State<'_, State>, id: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.repair(&id)).await
}
#[tauri::command]
async fn repair_php(state: tauri::State<'_, State>, version: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.repair_php(&version)).await
}
#[tauri::command]
async fn select_php(state: tauri::State<'_, State>, version: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.select_php(&version)).await
}
#[tauri::command]
async fn select_project_php(
    state: tauri::State<'_, State>,
    id: String,
    version: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.select_project_php(&id, &version)
    })
    .await
}
#[tauri::command]
async fn open_project_terminal(state: tauri::State<'_, State>, id: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.open_project_terminal(&id)).await
}
#[tauri::command]
async fn repair_project_php(
    state: tauri::State<'_, State>,
    id: String,
    version: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.repair_project_php(&id, &version)
    })
    .await
}
#[tauri::command]
async fn requirements(
    state: tauri::State<'_, State>,
) -> Result<Vec<serverbond_core::requirements::Requirement>, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || Ok(state.requirements())).await
}
#[tauri::command]
async fn open_runtime_download(state: tauri::State<'_, State>) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.open_runtime_download()).await
}
#[tauri::command]
async fn service(state: tauri::State<'_, State>, id: String, action: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        match action.parse::<EnvironmentAction>()? {
            EnvironmentAction::Start => state.start(&id),
            EnvironmentAction::Stop => state.stop(&id),
        }
    })
    .await
}
#[tauri::command]
async fn add_project(
    state: tauri::State<'_, State>,
    name: String,
    path: String,
    create: bool,
) -> Result<Project, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        if create {
            state.create_project(name, PathBuf::from(path))
        } else {
            state.add_project(name, PathBuf::from(path))
        }
    })
    .await
}
#[tauri::command]
async fn remove_project(state: tauri::State<'_, State>, id: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.remove_project(&id)).await
}
#[tauri::command]
async fn save_settings(state: tauri::State<'_, State>, settings: Settings) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.save_settings(settings)?;
        // Preferences are already persisted; a listener problem is reported
        // without undoing the save.
        if let Err(error) = state.ensure_api() {
            state.log(format!("Yönetim API'si güncellenemedi: {error:#}"));
            return Err(error);
        }
        Ok(())
    })
    .await
}
#[tauri::command]
async fn api_status(
    state: tauri::State<'_, State>,
) -> Result<serverbond_core::api::ApiStatus, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || Ok(state.api_status())).await
}

#[tauri::command]
fn api_documentation() -> serde_json::Value {
    serverbond_core::api::documentation()
}

#[tauri::command]
async fn api_save(
    state: tauri::State<'_, State>,
    settings: serverbond_core::preferences::ApiSettings,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.save_api_settings(settings)?;
        state.ensure_api()
    })
    .await
}
#[tauri::command]
async fn api_token(state: tauri::State<'_, State>, action: String) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || match action.as_str() {
        "create" => state.create_api_token(),
        "forget" => state.clear_api_token().map(|_| String::new()),
        other => Err(anyhow::anyhow!("Bilinmeyen API jeton işlemi: {other}")),
    })
    .await
}
#[tauri::command]
async fn read_log(state: tauri::State<'_, State>, id: String) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.read_log(&id)).await
}
#[tauri::command]
async fn credentials(state: tauri::State<'_, State>) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.credentials()).await
}
#[tauri::command]
async fn change_mysql_password(
    state: tauri::State<'_, State>,
    password: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.change_mysql_password(&password)
    })
    .await
}
#[tauri::command]
async fn database(
    state: tauri::State<'_, State>,
    name: String,
    action: String,
    path: Option<String>,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || match action.as_str() {
        "create" => {
            state.create_database(&name)?;
            Ok("Veritabanı oluşturuldu.".into())
        }
        "backup" => state.backup_database(&name),
        "restore" => {
            let path = path.ok_or_else(|| anyhow::anyhow!("SQL dosyası gerekli."))?;
            state.restore_database(&name, PathBuf::from(path))?;
            Ok("Veritabanı geri yüklendi.".into())
        }
        _ => Err(anyhow::anyhow!("Bilinmeyen işlem")),
    })
    .await
}
#[tauri::command]
async fn discover_projects(
    state: tauri::State<'_, State>,
) -> Result<Vec<DiscoveredProject>, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.discover_projects()).await
}
#[tauri::command]
async fn import_projects(
    state: tauri::State<'_, State>,
    paths: Vec<String>,
) -> Result<Vec<Project>, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.import_projects(paths.into_iter().map(PathBuf::from).collect())
    })
    .await
}
#[tauri::command]
async fn https_trust(state: tauri::State<'_, State>, action: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || match action.as_str() {
        "trust" => state.trust_https(),
        "untrust" => state.untrust_https(),
        _ => Err(anyhow::anyhow!("Bilinmeyen işlem")),
    })
    .await
}
#[tauri::command]
async fn open_project(state: tauri::State<'_, State>, id: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.open_project(&id)).await
}
#[tauri::command]
async fn open_home(state: tauri::State<'_, State>) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.open_home()).await
}
#[tauri::command]
async fn open_phpmyadmin(state: tauri::State<'_, State>) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.open_phpmyadmin()).await
}

#[tauri::command]
async fn settings_defaults(state: tauri::State<'_, State>) -> Result<Settings, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || Ok(state.default_settings())).await
}
#[tauri::command]
async fn settings_previous(state: tauri::State<'_, State>) -> Result<Settings, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.previous_settings()).await
}
#[tauri::command]
async fn settings_validate_import(json: String) -> Result<Settings, String> {
    if json.len() > 256 * 1024 {
        return Err("Ayar dosyası 256 KB sınırını aşıyor.".into());
    }
    let settings: Settings =
        serde_json::from_str(&json).map_err(|e| format!("Geçersiz ayar dosyası: {e}"))?;
    settings.validate().map_err(|e| e.to_string())?;
    Ok(settings)
}

#[tauri::command]
async fn save_project_jobs(
    state: tauri::State<'_, State>,
    id: String,
    workers: Vec<QueueWorker>,
    schedule: ProjectSchedule,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.save_project_jobs(&id, workers, schedule)
    })
    .await
}
#[tauri::command]
async fn start_project_worker(
    state: tauri::State<'_, State>,
    id: String,
    worker_id: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.start_project_worker(&id, &worker_id)
    })
    .await
}
#[tauri::command]
async fn stop_project_worker(
    state: tauri::State<'_, State>,
    id: String,
    worker_id: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.stop_project_worker(&id, &worker_id)
    })
    .await
}
#[tauri::command]
async fn start_project_schedule(state: tauri::State<'_, State>, id: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.start_project_schedule(&id)).await
}
#[tauri::command]
async fn stop_project_schedule(state: tauri::State<'_, State>, id: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.stop_project_schedule(&id)).await
}
#[tauri::command]
async fn list_project_schedule(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.list_project_schedule(&id)).await
}
#[tauri::command]
async fn restart_project_worker(
    state: tauri::State<'_, State>,
    id: String,
    worker_id: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.restart_project_worker(&id, &worker_id)
    })
    .await
}
#[tauri::command]
async fn restart_project_schedule(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.restart_project_schedule(&id)).await
}
#[tauri::command]
async fn list_failed_jobs(state: tauri::State<'_, State>, id: String) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.list_failed_jobs(&id)).await
}
#[tauri::command]
async fn retry_failed_jobs(
    state: tauri::State<'_, State>,
    id: String,
    job: Option<String>,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.retry_failed_jobs(&id, job.as_deref())
    })
    .await
}
#[tauri::command]
async fn flush_failed_jobs(state: tauri::State<'_, State>, id: String) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.flush_failed_jobs(&id)).await
}
#[tauri::command]
async fn read_project_worker_log(
    state: tauri::State<'_, State>,
    id: String,
    worker_id: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.read_project_worker_log(&id, &worker_id)
    })
    .await
}
#[tauri::command]
async fn read_project_schedule_log(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.read_project_schedule_log(&id)).await
}
#[tauri::command]
async fn read_project_log(
    state: tauri::State<'_, State>,
    id: String,
    source: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.read_project_log(&id, &source)).await
}
#[tauri::command]
async fn save_project_release(
    state: tauri::State<'_, State>,
    id: String,
    release: ProjectRelease,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.save_project_release(&id, release)
    })
    .await
}
#[tauri::command]
async fn deploy_project(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<ReleaseRecord, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.deploy_project(&id)).await
}
#[tauri::command]
async fn list_project_releases(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<Vec<ReleaseRecord>, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.list_project_releases(&id)).await
}
#[tauri::command]
async fn read_project_env(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<ProjectEnv, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.read_project_env(&id)).await
}
#[tauri::command]
async fn save_project_env(
    state: tauri::State<'_, State>,
    id: String,
    content: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.save_project_env(&id, content)).await
}
#[tauri::command]
async fn project_git_status(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<ProjectGitStatus, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.project_git_status(&id)).await
}

#[tauri::command]
async fn tunnel(state: tauri::State<'_, State>, action: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || match action.as_str() {
        "forget" => state.clear_tunnel_token(),
        other => match other.parse::<ToolAction>()? {
            ToolAction::Install => state.install_tunnel(),
            ToolAction::Repair => state.repair_tunnel(),
            ToolAction::Start => state.start_tunnel(),
            ToolAction::Stop => state.stop_tunnel(),
            ToolAction::Open => Err(anyhow::anyhow!(
                "Tünelin tarayıcıda açılacak bir arayüzü yok"
            )),
        },
    })
    .await
}
#[tauri::command]
async fn mail(state: tauri::State<'_, State>, action: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        match action.parse::<ToolAction>()? {
            ToolAction::Install => state.install_mail(),
            ToolAction::Repair => state.repair_mail(),
            ToolAction::Start => state.start_mail(),
            ToolAction::Stop => state.stop_mail(),
            ToolAction::Open => state.open_mail(),
        }
    })
    .await
}
#[tauri::command]
async fn postgres(
    state: tauri::State<'_, State>,
    action: String,
    password: Option<String>,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || match action.as_str() {
        "credentials" => state.postgres_credentials(),
        "password" => state
            .change_postgres_password(
                password
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("Yeni PostgreSQL parolası gerekli."))?,
            )
            .map(|_| String::new()),
        other => match other.parse::<ToolAction>()? {
            ToolAction::Install => state.install_postgres().map(|_| String::new()),
            ToolAction::Repair => state.repair_postgres().map(|_| String::new()),
            ToolAction::Start => state.start_postgres().map(|_| String::new()),
            ToolAction::Stop => state.stop_postgres().map(|_| String::new()),
            ToolAction::Open => Err(anyhow::anyhow!(
                "PostgreSQL'in tarayıcıda açılacak bir arayüzü yok"
            )),
        },
    })
    .await
}
#[tauri::command]
async fn redis(state: tauri::State<'_, State>, action: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        match action.parse::<ToolAction>()? {
            ToolAction::Install => state.install_redis(),
            ToolAction::Repair => state.repair_redis(),
            ToolAction::Start => state.start_redis(),
            ToolAction::Stop => state.stop_redis(),
            ToolAction::Open => Err(anyhow::anyhow!(
                "Redis'in tarayıcıda açılacak bir arayüzü yok"
            )),
        }
    })
    .await
}
#[tauri::command]
async fn github(
    state: tauri::State<'_, State>,
    action: String,
    token: Option<String>,
    repository: Option<String>,
    name: Option<String>,
    branch: Option<String>,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        match action.parse::<GithubAction>()? {
            GithubAction::Save => state
                .save_github_token(
                    token
                        .as_deref()
                        .ok_or_else(|| anyhow::anyhow!("GitHub jetonu gerekli."))?,
                )
                .map(|_| String::new()),
            GithubAction::Forget => state.clear_github_token().map(|_| String::new()),
            GithubAction::Import => {
                let repository = repository
                    .as_deref()
                    .map(str::trim)
                    .ok_or_else(|| anyhow::anyhow!("GitHub deposu gerekli."))?;
                let name = name.unwrap_or_default();
                let branch = branch.unwrap_or_default();
                // Any other https:// remote (GitLab, Bitbucket, self-hosted) goes
                // through the same clone path with the same prompt-free git setup.
                if repository.starts_with("https://") && !repository.contains("github.com/") {
                    state.import_git_project(repository, name, branch)
                } else {
                    state.import_github_project(repository, name, branch)
                }
                .map(|_| String::new())
            }
        }
    })
    .await
}
#[tauri::command]
async fn node(state: tauri::State<'_, State>, action: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        match action.parse::<ToolAction>()? {
            ToolAction::Install => state.install_node(),
            ToolAction::Repair => state.repair_node(),
            ToolAction::Start | ToolAction::Stop | ToolAction::Open => Err(anyhow::anyhow!(
                "Node.js bir süreç değildir; yalnızca kur/onar."
            )),
        }
    })
    .await
}
#[tauri::command]
async fn save_tunnel_token(
    state: tauri::State<'_, State>,
    token: String,
    start: bool,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        if start {
            state.apply_tunnel(&token)
        } else {
            state.save_tunnel_token(&token)
        }
    })
    .await
}
#[tauri::command]
async fn save_tunnel_auto_start(
    state: tauri::State<'_, State>,
    auto_start: bool,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || {
        state.save_tunnel_auto_start(auto_start)
    })
    .await
}
#[tauri::command]
async fn grant_permissions(
    state: tauri::State<'_, State>,
    defender: bool,
) -> Result<serverbond_core::permissions::PermissionState, String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.grant_permissions(defender)).await
}

#[tauri::command]
async fn recover_configuration(state: tauri::State<'_, State>) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(state.clone(), move || state.recover_configuration()).await
}

#[tauri::command]
async fn desktop_status(app: tauri::AppHandle) -> Result<desktop::DesktopStatus, String> {
    let state = app.state::<State>().inner().clone();
    blocking(state, move || Ok(app.state::<desktop::Desktop>().status())).await
}

#[tauri::command]
fn appearance_save(
    app: tauri::AppHandle,
    theme: appearance::Theme,
    initialize_only: bool,
) -> Result<appearance::Theme, String> {
    appearance::save(&app, theme, initialize_only).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn desktop_save(
    app: tauri::AppHandle,
    preferences: desktop::Preferences,
    autostart: bool,
) -> Result<(), String> {
    let state = app.state::<State>().inner().clone();
    blocking(state.clone(), move || {
        app.state::<desktop::Desktop>()
            .save(preferences, autostart, &state.home)
    })
    .await
}

#[tauri::command]
fn desktop_navigation(app: tauri::AppHandle) -> Option<String> {
    app.state::<desktop::Desktop>()
        .pending_page
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}

#[tauri::command]
fn desktop_action(app: tauri::AppHandle, action: String) -> Result<(), String> {
    match action.as_str() {
        "hide" => {
            if !app
                .state::<desktop::Desktop>()
                .tray_available
                .load(Ordering::Acquire)
            {
                return Err("Sistem tepsisi kullanılamıyor; pencere gizlenmedi.".into());
            }
            if let Some(window) = app.get_webview_window("main") {
                window.hide().map_err(|e| e.to_string())?;
            }
        }
        "exit" => desktop::request_exit(&app),
        "menu" => {
            let menu = app
                .try_state::<tray::TrayMenu>()
                .ok_or("Tepsi menüsü kullanılamıyor.")?;
            let window = app
                .get_webview_window("main")
                .ok_or("Pencere bulunamadı.")?;
            window.popup_menu(&menu.menu).map_err(|e| e.to_string())?;
        }
        _ => return Err("Bilinmeyen masaüstü işlemi.".into()),
    }
    Ok(())
}

fn launch_home() -> anyhow::Result<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--data-home" {
            let path = PathBuf::from(
                args.next()
                    .ok_or_else(|| anyhow::anyhow!("Veri klasörü belirtilmedi."))?,
            );
            if !path.is_absolute() {
                anyhow::bail!("Veri klasörü mutlak bir yol olmalı.");
            }
            return Ok(path);
        }
    }
    Ok(Manager::default_home())
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if !args.iter().any(|arg| arg == "--autostart") {
                desktop::show(app, None);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let manager = Arc::new(Manager::open_recovering(launch_home()?)?);
            let desktop = desktop::Desktop::new(&manager.home);
            app.manage(manager.clone());
            app.manage(desktop);
            app.manage(appearance::Appearance::new(&manager.home));
            manager.attach_desktop_api(Arc::new(api::Host::new(app.handle().clone())));
            if let Err(error) = manager.ensure_api() {
                manager.log(format!("Yönetim API'si başlatılamadı: {error:#}"));
            }
            if let Err(error) = tray::setup(app.handle()) {
                manager.log(format!(
                    "Tepsi kullanılamıyor; pencere açık tutuluyor: {error:#}"
                ));
            }
            let status = app.state::<desktop::Desktop>().status();
            let minimized = desktop::start_hidden(
                std::env::args_os().any(|arg| arg == "--autostart"),
                &status,
                manager.recovery_issue().is_some(),
            );
            if !minimized {
                desktop::show(app.handle(), None);
            }
            if manager.recovery_issue().is_none() {
                let app = app.handle().clone();
                let start = manager.snapshot()?.settings.start_on_launch;
                tauri::async_runtime::spawn_blocking(move || {
                    if let Err(error) = manager.contain(|| manager.ensure_permissions()) {
                        manager.log(format!("Windows izinleri uygulanamadı: {error:#}"));
                    }
                    if start {
                        if let Err(error) = manager.contain(|| manager.start("all")) {
                            desktop::report(
                                &app,
                                format!("Otomatik başlangıç başarısız: {error:#}"),
                            );
                        }
                    }
                    if let Err(error) = manager.contain(|| manager.prepare_launch_tools()) {
                        manager.log(format!("Açılış araçları hazırlanamadı: {error:#}"));
                    }
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let desktop = window.state::<desktop::Desktop>();
                api.prevent_close();
                if desktop.tray_available.load(Ordering::Acquire)
                    && desktop.preferences().close_to_tray
                {
                    if let Err(error) = window.hide() {
                        desktop::report(window.app_handle(), error);
                    }
                } else {
                    desktop::request_exit(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            desktop_status,
            appearance_save,
            desktop_save,
            desktop_action,
            desktop_navigation,
            recover_configuration,
            api_status,
            api_documentation,
            api_save,
            api_token,
            install,
            repair,
            repair_php,
            select_php,
            select_project_php,
            repair_project_php,
            open_project_terminal,
            requirements,
            open_runtime_download,
            service,
            add_project,
            remove_project,
            save_settings,
            settings_defaults,
            settings_previous,
            settings_validate_import,
            read_log,
            credentials,
            change_mysql_password,
            database,
            discover_projects,
            import_projects,
            https_trust,
            open_project,
            open_home,
            open_phpmyadmin,
            save_project_jobs,
            start_project_worker,
            stop_project_worker,
            start_project_schedule,
            stop_project_schedule,
            restart_project_worker,
            restart_project_schedule,
            list_project_schedule,
            list_failed_jobs,
            retry_failed_jobs,
            flush_failed_jobs,
            read_project_worker_log,
            read_project_schedule_log,
            read_project_log,
            read_project_env,
            save_project_env,
            save_project_release,
            deploy_project,
            list_project_releases,
            project_git_status,
            tunnel,
            mail,
            postgres,
            redis,
            github,
            node,
            save_tunnel_token,
            save_tunnel_auto_start,
            grant_permissions
        ])
        .build(tauri::generate_context!());
    let app = match app {
        Ok(app) => app,
        Err(error) => {
            show_startup_error(&format!("ServerBond penceresi açılamadı: {error}"));
            return;
        }
    };
    app.run(move |app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if !app
                .state::<desktop::Desktop>()
                .exit_ready
                .load(Ordering::Acquire)
            {
                api.prevent_exit();
                desktop::request_exit(app);
            }
        }
    });
}

fn show_startup_error(error: &str) {
    eprintln!("{error}");
    #[cfg(windows)]
    {
        let message: Vec<u16> = error.encode_utf16().chain(Some(0)).collect();
        let title: Vec<u16> = "ServerBond".encode_utf16().chain(Some(0)).collect();
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                std::ptr::null_mut(),
                message.as_ptr(),
                title.as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
    }
}
