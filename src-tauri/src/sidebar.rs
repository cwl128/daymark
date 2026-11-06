use crate::{db::Database, windows::RuntimeState};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SidebarState {
    pub enabled: bool,
    pub expanded: bool,
    pub pinned: bool,
    #[serde(skip)]
    pub content_height: f64,
    #[serde(skip)]
    pub auto_hide: bool,
}
pub fn state(app: &tauri::AppHandle) -> Result<SidebarState, String> {
    Ok(app
        .state::<RuntimeState>()
        .sidebar
        .lock()
        .map_err(|_| "侧栏状态不可用")?
        .clone())
}
pub struct WorkArea {
    pub scale: f64,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
pub fn geometry(
    expanded: bool,
    content_height: f64,
    area: WorkArea,
    left: bool,
) -> (f64, f64, i32, i32) {
    let WorkArea {
        scale,
        x,
        y,
        width,
        height,
    } = area;
    let scale = if scale.is_finite() && scale > 0. {
        scale
    } else {
        1.
    };
    let w = if expanded {
        280_f64.min(width as f64 / scale)
    } else {
        8_f64.min(width as f64 / scale)
    };
    let available = (height as f64 / scale - 32.).max(1.);
    let h = if expanded {
        content_height.clamp(160., 420.).min(available)
    } else {
        56_f64.min(available)
    };
    let px = if left {
        x
    } else {
        x + width as i32 - (w * scale).round() as i32
    };
    let py = y + ((height as f64 - h * scale) / 2.).round() as i32;
    (w, h, px, py)
}
pub fn cancel_pending(app: &tauri::AppHandle) {
    app.state::<RuntimeState>()
        .sidebar_generation
        .fetch_add(1, Ordering::SeqCst);
}
pub fn apply(app: &tauri::AppHandle) -> Result<(), String> {
    let settings = app.state::<Database>().settings()?;
    {
        let runtime = app.state::<RuntimeState>();
        let mut s = runtime.sidebar.lock().map_err(|_| "侧栏状态不可用")?;
        s.enabled = settings.sidebar_enabled;
        s.auto_hide = settings.sidebar_auto_hide;
        if !s.enabled {
            s.expanded = false;
            s.pinned = false;
        }
    }
    let s = state(app)?;
    if !s.enabled {
        if let Some(w) = app.get_webview_window("sidebar") {
            w.hide().map_err(|e| e.to_string())?;
        }
        app.emit("sidebar-state", s).map_err(|e| e.to_string())?;
        return Ok(());
    }
    let window = if let Some(w) = app.get_webview_window("sidebar") {
        w
    } else {
        WebviewWindowBuilder::new(
            app,
            "sidebar",
            WebviewUrl::App("index.html?window=sidebar".into()),
        )
        .title("Daymark · 今日安排")
        .inner_size(8., 56.)
        .min_inner_size(1., 1.)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focusable(false)
        .focused(false)
        .accept_first_mouse(true)
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
    let (w, h, x, y) = geometry(
        s.expanded,
        if s.content_height > 0. {
            s.content_height
        } else {
            280.
        },
        WorkArea {
            scale: monitor.scale_factor(),
            x: area.position.x,
            y: area.position.y,
            width: area.size.width,
            height: area.size.height,
        },
        settings.sidebar_side == "left",
    );
    // Becoming focusable does not activate the window. Hover preserves the
    // current application's keyboard focus; a real click can focus naturally.
    window
        .set_focusable(s.expanded)
        .map_err(|e| e.to_string())?;
    window
        .set_size(LogicalSize::new(w, h))
        .map_err(|e| e.to_string())?;
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    if !window.is_visible().map_err(|e| e.to_string())? {
        window.show().map_err(|e| e.to_string())?;
    }
    app.emit("sidebar-state", s).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn expand(app: &tauri::AppHandle, expanded: bool) -> Result<(), String> {
    cancel_pending(app);
    {
        let runtime = app.state::<RuntimeState>();
        let mut s = runtime.sidebar.lock().map_err(|_| "侧栏状态不可用")?;
        s.expanded = expanded;
        if !expanded {
            s.pinned = false;
        }
    }
    apply(app)
}
pub fn action(app: &tauri::AppHandle, action: &str) -> Result<(), String> {
    cancel_pending(app);
    match action {
        "toggle" => {
            let expanded = state(app)?.expanded;
            expand(app, !expanded)?;
        }
        "collapse" => expand(app, false)?,
        "pin" => {
            let runtime = app.state::<RuntimeState>();
            let mut s = runtime.sidebar.lock().map_err(|_| "侧栏状态不可用")?;
            s.pinned = !s.pinned;
            s.expanded = true;
            drop(s);
            apply(app)?;
        }
        _ => return Err("无效侧栏操作".into()),
    }
    Ok(())
}
pub fn hover(app: tauri::AppHandle, inside: bool) -> Result<(), String> {
    cancel_pending(&app);
    let s = state(&app)?;
    let settings = app.state::<Database>().settings()?;
    if !s.enabled || s.pinned || (!inside && !settings.sidebar_auto_hide) {
        return Ok(());
    }
    let generation = app
        .state::<RuntimeState>()
        .sidebar_generation
        .load(Ordering::SeqCst);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(if inside {
            250
        } else {
            350
        }));
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if handle
                .state::<RuntimeState>()
                .sidebar_generation
                .load(Ordering::SeqCst)
                == generation
            {
                let _ = expand(&handle, inside);
            }
        });
    });
    Ok(())
}
pub fn focus_lost(app: &tauri::AppHandle) {
    if let (Ok(s), Ok(settings)) = (state(app), app.state::<Database>().settings()) {
        if s.expanded && !s.pinned && settings.sidebar_auto_hide {
            let _ = expand(app, false);
        }
    }
}
pub fn resize(app: &tauri::AppHandle, height: f64) -> Result<(), String> {
    if !height.is_finite() {
        return Err("侧栏高度无效".into());
    }
    {
        let runtime = app.state::<RuntimeState>();
        let mut s = runtime.sidebar.lock().map_err(|_| "侧栏状态不可用")?;
        if (s.content_height - height).abs() < 2. {
            return Ok(());
        }
        s.content_height = height.clamp(160., 420.);
    }
    if state(app)?.expanded {
        apply(app)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collapsed_is_small_at_every_scale() {
        for scale in [1., 1.5, 2.] {
            let (w, h, x, y) = geometry(
                false,
                680.,
                WorkArea {
                    scale,
                    x: -1920,
                    y: 24,
                    width: 1920,
                    height: 1080,
                },
                false,
            );
            assert_eq!((w, h), (8., 56.));
            assert_eq!(x, -(8. * scale).round() as i32);
            assert!(y >= 24);
        }
    }
    #[test]
    fn panel_clamps_to_work_area_and_handles_left_monitor() {
        let (w, h, x, y) = geometry(
            true,
            900.,
            WorkArea {
                scale: 2.,
                x: -1920,
                y: 24,
                width: 1920,
                height: 700,
            },
            true,
        );
        assert_eq!(w, 280.);
        assert_eq!(h, 318.);
        assert_eq!(x, -1920);
        assert!(y >= 24);
        assert!((y as f64 + h * 2.) <= 724.);
    }
}
