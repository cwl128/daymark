use crate::{db::Database, model::Item};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub item: Item,
    pub scheduled_at: i64,
}
#[derive(Default)]
pub struct RuntimeState {
    pub active: Mutex<Option<Alert>>,
    pub expanded: Mutex<bool>,
    pub last_error: Mutex<Option<String>>,
}

pub fn show_main(app: &tauri::AppHandle) -> Result<(), String> {
    let w = app.get_webview_window("main").ok_or("主窗口不存在")?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())
}
pub fn sidebar(app: &tauri::AppHandle, expanded: bool) -> Result<(), String> {
    let window = if let Some(w) = app.get_webview_window("sidebar") {
        w
    } else {
        WebviewWindowBuilder::new(
            app,
            "sidebar",
            WebviewUrl::App("index.html?window=sidebar".into()),
        )
        .title("Daymark · 今日安排")
        .inner_size(320., 620.)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?
    };
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .or(app.primary_monitor().map_err(|e| e.to_string())?)
        .ok_or("未找到屏幕")?;
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let width = if expanded { 320. } else { 28. };
    let height = (area.size.height as f64 / scale - 32.).clamp(200., 680.);
    let side = app.state::<Database>().settings()?.sidebar_side;
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    let x = if side == "left" {
        area.position.x
    } else {
        area.position.x + area.size.width as i32 - (width * scale).round() as i32
    };
    window
        .set_position(PhysicalPosition::new(
            x,
            area.position.y + ((area.size.height as f64 - height * scale) / 2.).round() as i32,
        ))
        .map_err(|e| e.to_string())?;
    *app.state::<RuntimeState>()
        .expanded
        .lock()
        .map_err(|_| "侧边状态不可用")? = expanded;
    window.show().map_err(|e| e.to_string())?;
    let _ = app.emit("sidebar-state", expanded);
    Ok(())
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
