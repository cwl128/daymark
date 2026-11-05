mod db;
mod model;
mod scheduler;
mod windows;
use db::Database;
use model::{Item, Settings};
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
#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    settings: Settings,
) -> Result<(), String> {
    db.set_settings(&settings)?;
    if !settings.fullscreen {
        windows::hide_alert(&app)?;
    }
    if app.get_webview_window("sidebar").is_some() {
        let expanded = *app
            .state::<RuntimeState>()
            .expanded
            .lock()
            .map_err(|_| "侧边状态不可用")?;
        windows::sidebar(&app, expanded)?;
    }
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn set_sidebar(app: tauri::AppHandle, expanded: bool) -> Result<(), String> {
    windows::sidebar(&app, expanded)
}
#[tauri::command]
fn sidebar_state(state: State<'_, RuntimeState>) -> Result<bool, String> {
    Ok(*state.expanded.lock().map_err(|_| "侧边状态不可用")?)
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
async fn test_notification(app: tauri::AppHandle) -> Result<(), String> {
    let settings = app.state::<Database>().settings()?;
    let item = Item {
        id: "test".into(),
        title: "测试通知".into(),
        notes: "".into(),
        kind: "event".into(),
        starts_at: Some(chrono::Utc::now().timestamp() + 600),
        ends_at: None,
        due_at: None,
        completed_at: None,
        snooze_until: None,
        version: 0,
    };
    tauri::async_runtime::spawn_blocking(move || {
        scheduler::send_notification(&app, &item, settings.sound)
    })
    .await
    .map_err(|e| e.to_string())?
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = windows::show_main(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .manage(RuntimeState::default())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(Database::open(&dir.join("daymark.db")).map_err(std::io::Error::other)?);
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
                        let _ = windows::sidebar(app, true);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            scheduler::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                match window.label() {
                    "main" => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    "sidebar" => {
                        api.prevent_close();
                        let _ = windows::sidebar(window.app_handle(), false);
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
            set_sidebar,
            sidebar_state,
            show_main,
            get_alert,
            act_alert,
            test_notification,
            reminder_status
        ])
        .run(tauri::generate_context!())
        .expect("Daymark startup failed");
}
