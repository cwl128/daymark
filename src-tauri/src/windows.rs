use crate::{db::Database, model::Item};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub item: Item,
    pub scheduled_at: i64,
}
#[derive(Default)]
pub struct RuntimeState {
    pub active: Mutex<Option<Alert>>,
    pub sidebar: Mutex<crate::sidebar::SidebarState>,
    pub sidebar_generation: std::sync::atomic::AtomicU64,
    pub notification_test: Mutex<crate::notifications::TestState>,
    pub last_error: Mutex<Option<String>>,
}

pub fn show_main(app: &tauri::AppHandle) -> Result<(), String> {
    let w = app.get_webview_window("main").ok_or("主窗口不存在")?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())
}
pub fn hide_alert(app: &tauri::AppHandle) -> Result<(), String> {
    *app.state::<RuntimeState>()
        .active
        .lock()
        .map_err(|_| "提醒状态不可用")? = None;
    if let Some(w) = app.get_webview_window("reminder") {
        w.hide().map_err(|e| e.to_string())?;
    }
    let _ = app.emit("alert-changed", ());
    Ok(())
}
pub fn reconcile_alert(app: &tauri::AppHandle) -> Result<(), String> {
    let active = app
        .state::<RuntimeState>()
        .active
        .lock()
        .map_err(|_| "提醒状态不可用")?
        .clone();
    if let Some(alert) = active {
        let current = app.state::<Database>().get(&alert.item.id);
        if current.as_ref().map_or(true, |i| {
            i.completed_at.is_some() || i.version != alert.item.version
        }) {
            hide_alert(app)?;
        }
    }
    Ok(())
}
pub fn show_alert(app: &tauri::AppHandle, alert: Alert) -> Result<bool, String> {
    if app
        .state::<RuntimeState>()
        .active
        .lock()
        .map_err(|_| "提醒状态不可用")?
        .is_some()
    {
        return Ok(false);
    }
    let current = app.state::<Database>().get(&alert.item.id)?;
    if current.completed_at.is_some() || current.version != alert.item.version {
        return Ok(false);
    }
    let window = if let Some(w) = app.get_webview_window("reminder") {
        w
    } else {
        WebviewWindowBuilder::new(
            app,
            "reminder",
            WebviewUrl::App("index.html?window=reminder".into()),
        )
        .title("Daymark · 日程提醒")
        .inner_size(900., 640.)
        .decorations(false)
        .always_on_top(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?
    };
    *app.state::<RuntimeState>()
        .active
        .lock()
        .map_err(|_| "提醒状态不可用")? = Some(alert);
    if let Err(e) = window
        .show()
        .and_then(|_| window.set_fullscreen(true))
        .and_then(|_| window.set_focus())
    {
        *app.state::<RuntimeState>()
            .active
            .lock()
            .map_err(|_| "提醒状态不可用")? = None;
        return Err(e.to_string());
    }
    let _ = app.emit("alert-changed", ());
    Ok(true)
}
