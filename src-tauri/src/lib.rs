mod db;
mod model;
mod notifications;
mod scheduler;
mod sidebar;
mod sidebar_watch;
mod windows;
use db::Database;
use model::{Item, Settings, SettingsPatch};
use tauri::{Emitter, Manager, State};
use windows::{Alert, RuntimeState};
fn changed(app: &tauri::AppHandle) {
    let _ = windows::reconcile_alert(app);
    let _ = app.emit("data-changed", ());
}
#[tauri::command]
fn list_items(db: State<'_, Database>) -> Result<Vec<Item>, String> {
    db.list()
}
#[tauri::command]
fn save_item(app: tauri::AppHandle, db: State<'_, Database>, item: Item) -> Result<Item, String> {
    let i = db.save(item)?;
    changed(&app);
    Ok(i)
}
#[tauri::command]
fn delete_item(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    id: String,
    version: i64,
) -> Result<(), String> {
    db.delete(&id, version)?;
    changed(&app);
    Ok(())
}
#[tauri::command]
fn complete_item(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    id: String,
    completed: bool,
) -> Result<(), String> {
    db.complete(&id, completed, chrono::Utc::now().timestamp())?;
    changed(&app);
    Ok(())
}
#[tauri::command]
fn get_settings(db: State<'_, Database>) -> Result<Settings, String> {
    db.settings()
}
fn apply_settings(app: &tauri::AppHandle, s: Settings) -> Result<Settings, String> {
    if !s.fullscreen {
        windows::hide_alert(app)?;
    }
    sidebar::cancel_pending(app);
    sidebar::apply(app)?;
    changed(app);
    Ok(s)
}
#[tauri::command]
async fn update_settings(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    patch: SettingsPatch,
) -> Result<Settings, String> {
    let s = db.patch_settings(patch)?;
    apply_settings(&app, s)
}
// Compatibility with 0.1.0 callers, now merging only supplied settings.
#[tauri::command]
async fn save_settings(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    settings: SettingsPatch,
) -> Result<Settings, String> {
    let s = db.patch_settings(settings)?;
    apply_settings(&app, s)
}
#[tauri::command]
fn sidebar_state(app: tauri::AppHandle) -> Result<sidebar::SidebarState, String> {
    sidebar::state(&app)
}
#[tauri::command]
async fn sidebar_action(app: tauri::AppHandle, action: String) -> Result<(), String> {
    sidebar::action(&app, &action)
}
#[tauri::command]
fn sidebar_hover(app: tauri::AppHandle, inside: bool) -> Result<(), String> {
    sidebar::hover(app, inside)
}
#[tauri::command]
fn sidebar_resize(app: tauri::AppHandle, height: f64) -> Result<(), String> {
    sidebar::resize(&app, height)
}
#[tauri::command]
fn show_main(app: tauri::AppHandle) -> Result<(), String> {
    windows::show_main(&app)
}
#[tauri::command]
fn get_alert(state: State<'_, RuntimeState>) -> Result<Option<Alert>, String> {
    Ok(state.active.lock().map_err(|_| "提醒状态不可用")?.clone())
}
#[tauri::command]
fn reminder_status(state: State<'_, RuntimeState>) -> Result<Option<String>, String> {
    Ok(state
        .last_error
        .lock()
        .map_err(|_| "提醒状态不可用")?
        .clone())
}
#[tauri::command]
fn act_alert(app: tauri::AppHandle, db: State<'_, Database>, action: String) -> Result<(), String> {
    let alert = app
        .state::<RuntimeState>()
        .active
        .lock()
        .map_err(|_| "提醒状态不可用")?
        .clone();
    if let Some(alert) = alert {
        match action.as_str() {
            "complete" => db.complete(&alert.item.id, true, chrono::Utc::now().timestamp())?,
            "snooze" => db.snooze(&alert.item.id, chrono::Utc::now().timestamp() + 300)?,
            "close" => {}
            _ => return Err("无效操作".into()),
        }
    }
    windows::hide_alert(&app)?;
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn notification_status(
    app: tauri::AppHandle,
) -> Result<notifications::NotificationStatus, String> {
    tauri::async_runtime::spawn_blocking(move || notifications::status(&app))
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn request_notification_permission(
    app: tauri::AppHandle,
) -> Result<notifications::NotificationStatus, String> {
    tauri::async_runtime::spawn_blocking(move || notifications::request(&app))
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn start_notification_test(
    app: tauri::AppHandle,
) -> Result<notifications::TestState, String> {
    tauri::async_runtime::spawn_blocking(move || notifications::start_test(app))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn notification_test_state(app: tauri::AppHandle) -> Result<notifications::TestState, String> {
    notifications::test_state(&app)
}
#[tauri::command]
fn cancel_notification_test(app: tauri::AppHandle) -> Result<(), String> {
    notifications::cancel_test(&app)
}
#[tauri::command]
async fn open_notification_settings() -> Result<(), String> {
    notifications::open_settings()
}
fn color(hex: &str) -> Option<tauri::window::Color> {
    if hex.len() != 7 || !hex.starts_with('#') {
        return None;
    }
    let value = u32::from_str_radix(&hex[1..], 16).ok()?;
    Some(tauri::window::Color(
        ((value >> 16) & 255) as u8,
        ((value >> 8) & 255) as u8,
        (value & 255) as u8,
        255,
    ))
}
#[tauri::command]
fn sync_theme(app: tauri::AppHandle, mode: String, background: String) -> Result<(), String> {
    let theme = match mode.as_str() {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        "system" => None,
        _ => return Err("主题无效".into()),
    };
    let bg = color(&background).ok_or("颜色无效")?;
    for window in app.webview_windows().values() {
        window.set_theme(theme).map_err(|e| e.to_string())?;
        window
            .set_background_color(Some(bg))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
fn ui_ready(window: tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        windows::show_main(window.app_handle())?;
    }
    Ok(())
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = windows::show_main(app);
        }))
        .manage(RuntimeState::default())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(Database::open(&dir.join("daymark.db")).map_err(std::io::Error::other)?);
            let s = app
                .state::<Database>()
                .settings()
                .map_err(std::io::Error::other)?;
            if let Some(w) = app.get_webview_window("main") {
                let dark = match s.theme_mode.as_str() {
                    "dark" => true,
                    "light" => false,
                    _ => w.theme()? == tauri::Theme::Dark,
                };
                w.set_background_color(color(if dark {
                    &s.background_dark
                } else {
                    &s.background_light
                }))?;
            }
            use tauri::{
                menu::{Menu, MenuItem},
                tray::TrayIconBuilder,
            };
            let open = MenuItem::with_id(app, "open", "打开 Daymark", true, None::<&str>)?;
            let side = MenuItem::with_id(app, "sidebar", "展开侧边栏", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出（停止提醒）", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &side, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .tooltip("Daymark · 离线日历与待办")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        let _ = windows::show_main(app);
                    }
                    "sidebar" => {
                        let patch = SettingsPatch {
                            sidebar_enabled: Some(true),
                            ..Default::default()
                        };
                        let _ = app.state::<Database>().patch_settings(patch);
                        let _ = sidebar::expand(app, true);
                        let _ = app.emit("data-changed", ());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            sidebar::apply(app.handle()).map_err(std::io::Error::other)?;
            sidebar_watch::start(app.handle());
            scheduler::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                if window.label() == "sidebar" {
                    sidebar::focus_lost(window.app_handle());
                }
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                match window.label() {
                    "main" => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    "sidebar" => {
                        api.prevent_close();
                        let _ = sidebar::expand(window.app_handle(), false);
                    }
                    "reminder" => {
                        api.prevent_close();
                        let _ = windows::hide_alert(window.app_handle());
                    }
                    _ => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_items,
            save_item,
            delete_item,
            complete_item,
            get_settings,
            save_settings,
            update_settings,
            sidebar_state,
            sidebar_action,
            sidebar_hover,
            sidebar_resize,
            show_main,
            get_alert,
            act_alert,
            reminder_status,
            notification_status,
            request_notification_permission,
            start_notification_test,
            notification_test_state,
            cancel_notification_test,
            open_notification_settings,
            sync_theme,
            ui_ready
        ])
        .run(tauri::generate_context!())
        .expect("Daymark startup failed");
}
