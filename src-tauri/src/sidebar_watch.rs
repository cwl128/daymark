use crate::{sidebar, windows::RuntimeState};
use std::sync::atomic::Ordering;
use tauri::Manager;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    Expand,
    Collapse,
}
#[derive(Default)]
struct Watch {
    entered_at: Option<u64>,
    left_at: Option<u64>,
    pressed: bool,
    foreground: Option<u32>,
    expanded: Option<bool>,
}
impl Watch {
    fn sample(
        &mut self,
        state: &sidebar::SidebarState,
        inside: bool,
        pressed: bool,
        foreground: Option<u32>,
        now: u64,
    ) -> Option<Action> {
        let click = pressed && !self.pressed;
        let switched = self.foreground.is_some()
            && foreground.is_some()
            && self.foreground != foreground
            && foreground != Some(std::process::id());
        self.pressed = pressed;
        self.foreground = foreground;
        if self.expanded != Some(state.expanded) {
            self.entered_at = None;
            self.left_at = None;
            self.expanded = Some(state.expanded);
        }
        if !state.enabled || state.pinned {
            self.entered_at = None;
            self.left_at = None;
            return None;
        }
        if !state.expanded {
            self.left_at = None;
            if !inside {
                self.entered_at = None;
                return None;
            }
            let entered = *self.entered_at.get_or_insert(now);
            return (now.saturating_sub(entered) >= 250).then_some(Action::Expand);
        }
        self.entered_at = None;
        if !state.auto_hide {
            self.left_at = None;
            return None;
        }
        if switched || (click && !inside) {
            return Some(Action::Collapse);
        }
        if inside {
            self.left_at = None;
            return None;
        }
        let left = *self.left_at.get_or_insert(now);
        (now.saturating_sub(left) >= 350).then_some(Action::Collapse)
    }
}

fn pointer_inside(window: &tauri::WebviewWindow) -> Option<bool> {
    // All three APIs use desktop physical coordinates, including Retina / DPI
    // scaling and displays positioned to the left or above the main display.
    let point = window.app_handle().cursor_position().ok()?;
    let pos = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some(
        point.x >= pos.x as f64
            && point.y >= pos.y as f64
            && point.x < pos.x as f64 + size.width as f64
            && point.y < pos.y as f64 + size.height as f64,
    )
}

fn desktop_input() -> (bool, Option<u32>) {
    #[cfg(target_os = "macos")]
    {
        objc2::rc::autoreleasepool(|_| {
            use objc2_app_kit::{NSEvent, NSWorkspace};
            (
                NSEvent::pressedMouseButtons() != 0,
                NSWorkspace::sharedWorkspace()
                    .frontmostApplication()
                    .map(|app| app.processIdentifier() as u32),
            )
        })
    }
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::{
            Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON},
            WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
        };
        let mut process = 0;
        // Only current mouse-button state and the foreground process number.
        // No keyboard, application content, names or history are collected.
        let pressed = unsafe {
            GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut process));
            GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 || GetAsyncKeyState(VK_RBUTTON.0 as i32) < 0
        };
        (pressed, (process != 0).then_some(process))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    (false, None)
}

pub fn start(app: &tauri::AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || {
        let clock = std::time::Instant::now();
        let mut watch = Watch::default();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(25));
            let Ok(state) = sidebar::state(&handle) else {
                continue;
            };
            if !state.enabled {
                watch = Watch::default();
                std::thread::sleep(std::time::Duration::from_millis(175));
                continue;
            }
            let generation = handle
                .state::<RuntimeState>()
                .sidebar_generation
                .load(Ordering::SeqCst);
            let Some(window) = handle.get_webview_window("sidebar") else {
                continue;
            };
            let inside_result = pointer_inside(&window);
            let (pressed, foreground) = desktop_input();
            let Some(inside) = inside_result else {
                continue;
            };
            let Some(action) = watch.sample(
                &state,
                inside,
                pressed,
                foreground,
                clock.elapsed().as_millis() as u64,
            ) else {
                continue;
            };
            let target = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                let Ok(current) = sidebar::state(&target) else {
                    return;
                };
                if !current.enabled
                    || current.pinned
                    || current.expanded != state.expanded
                    || target
                        .state::<RuntimeState>()
                        .sidebar_generation
                        .load(Ordering::SeqCst)
                        != generation
                {
                    return;
                }
                if action == Action::Collapse && !current.auto_hide {
                    return;
                }
                let _ = sidebar::expand(&target, action == Action::Expand);
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state(expanded: bool) -> sidebar::SidebarState {
        sidebar::SidebarState {
            enabled: true,
            expanded,
            auto_hide: true,
            ..Default::default()
        }
    }
    #[test]
    fn hover_leave_delays_cancel_when_pointer_returns() {
        let mut w = Watch::default();
        let closed = state(false);
        assert_eq!(w.sample(&closed, true, false, Some(1), 0), None);
        assert_eq!(w.sample(&closed, true, false, Some(1), 249), None);
        assert_eq!(w.sample(&closed, false, false, Some(1), 250), None);
        assert_eq!(w.sample(&closed, true, false, Some(1), 300), None);
        assert_eq!(
            w.sample(&closed, true, false, Some(1), 550),
            Some(Action::Expand)
        );
        let open = state(true);
        assert_eq!(w.sample(&open, false, false, Some(1), 600), None);
        assert_eq!(w.sample(&open, true, false, Some(1), 949), None);
        assert_eq!(w.sample(&open, false, false, Some(1), 1000), None);
        assert_eq!(w.sample(&open, false, false, Some(1), 1349), None);
        assert_eq!(
            w.sample(&open, false, false, Some(1), 1350),
            Some(Action::Collapse)
        );
    }
    #[test]
    fn external_click_or_application_change_collapses_without_sidebar_focus() {
        let mut w = Watch::default();
        let open = state(true);
        w.sample(&open, true, false, Some(1), 0);
        assert_eq!(
            w.sample(&open, false, true, Some(1), 1),
            Some(Action::Collapse)
        );
        w.sample(&open, true, false, Some(1), 2);
        assert_eq!(
            w.sample(&open, true, false, Some(2), 3),
            Some(Action::Collapse)
        );
        // Clicking an internal task checkbox must not collapse the panel.
        assert_eq!(w.sample(&open, true, true, Some(2), 4), None);
    }
    #[test]
    fn pinned_and_auto_hide_off_ignore_external_clicks() {
        let mut w = Watch::default();
        let mut open = state(true);
        open.pinned = true;
        assert_eq!(w.sample(&open, false, true, Some(1), 0), None);
        assert_eq!(w.sample(&open, false, false, Some(2), 1000), None);
        open.pinned = false;
        open.auto_hide = false;
        assert_eq!(w.sample(&open, false, true, Some(3), 2000), None);
        assert_eq!(w.sample(&open, false, false, Some(3), 5000), None);
    }
}
