use crate::{db::Database, model::Item, windows::RuntimeState};
use serde::Serialize;
use tauri::{Emitter, Manager};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationStatus {
    pub authorization: String,
    pub alerts: String,
    pub sound: String,
    pub detail: String,
}
impl NotificationStatus {
    fn unknown(detail: impl Into<String>) -> Self {
        Self {
            authorization: "unknown".into(),
            alerts: "unknown".into(),
            sound: "unknown".into(),
            detail: detail.into(),
        }
    }
    pub fn can_send(&self) -> bool {
        ["authorized", "provisional"].contains(&self.authorization.as_str())
    }
}
pub fn status(app: &tauri::AppHandle) -> NotificationStatus {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        use mac_usernotifications::{AuthorizationStatus as A, NotificationSettingStatus as S};
        match mac_usernotifications::blocking::get_notification_settings() {
            Ok(s) => {
                let authorization = match s.authorization_status {
                    A::NotDetermined => "notDetermined",
                    A::Denied => "denied",
                    A::Authorized => "authorized",
                    A::Provisional | A::Ephemeral => "provisional",
                    A::Unknown => "unknown",
                };
                let setting = |s| {
                    match s {
                        S::Enabled => "enabled",
                        S::Disabled => "disabled",
                        _ => "unknown",
                    }
                    .to_string()
                };
                NotificationStatus {
                    authorization: authorization.into(),
                    alerts: setting(s.alert_enabled),
                    sound: setting(s.sound_enabled),
                    detail: match authorization {
                        "notDetermined" => {
                            "还没有申请通知权限。点击允许通知后，系统会询问你是否同意。"
                        }
                        "denied" => "Daymark 的通知已被关闭，请在系统通知设置中允许通知。",
                        "authorized" => {
                            "系统已允许通知；横幅和声音仍受通知样式、专注模式及共享屏幕设置影响。"
                        }
                        "provisional" => "通知可能只进入通知中心，请在系统设置中开启横幅。",
                        _ => "暂时无法确认系统通知权限。",
                    }
                    .into(),
                }
            }
            Err(e) => NotificationStatus::unknown(format!(
                "无法读取通知权限。请使用安装后的 Daymark，并确认应用完整性。详情：{e}"
            )),
        }
    }
    #[cfg(target_os = "windows")]
    {
        use windows::{
            core::HSTRING,
            UI::Notifications::{NotificationSetting, ToastNotificationManager},
        };
        let result = (|| -> windows::core::Result<_> {
            let n = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
                &app.config().identifier,
            ))?;
            n.Setting()
        })();
        match result {
            Ok(s) => {
                let enabled = s == NotificationSetting::Enabled;
                NotificationStatus {
                    authorization: if enabled { "authorized" } else { "denied" }.into(),
                    alerts: if enabled { "enabled" } else { "disabled" }.into(),
                    sound: "unknown".into(),
                    detail: if enabled {
                        "系统允许发送通知；请同时检查横幅、声音和勿扰设置。".into()
                    } else if s == NotificationSetting::DisabledByManifest {
                        "系统未识别 Daymark 的通知注册，请通过安装包重新安装后测试。".into()
                    } else if s == NotificationSetting::DisabledByGroupPolicy {
                        "通知被设备管理策略关闭，请联系设备管理员。".into()
                    } else {
                        "Daymark 的通知被系统关闭，请打开通知设置检查。".into()
                    },
                }
            }
            Err(e) => NotificationStatus::unknown(format!(
                "无法检查通知。请使用安装包安装 Daymark 后再测试。详情：{e}"
            )),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app;
        NotificationStatus::unknown("此平台暂不支持原生通知自测")
    }
}
pub fn request(app: &tauri::AppHandle) -> NotificationStatus {
    #[cfg(target_os = "macos")]
    {
        if status(app).authorization == "notDetermined" {
            if let Err(e) = mac_usernotifications::blocking::request_auth() {
                return NotificationStatus::unknown(format!("申请通知权限失败：{e}"));
            }
        }
    }
    status(app)
}
pub fn open_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let result = std::process::Command::new("/usr/bin/open")
            .arg("x-apple.systempreferences:com.apple.Notifications-Settings.extension")
            .status()
            .map_err(|e| e.to_string())?;
        if !result.success() {
            std::process::Command::new("/usr/bin/open")
                .args(["-b", "com.apple.systempreferences"])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        let uri = windows::Foundation::Uri::CreateUri(&windows::core::HSTRING::from(
            "ms-settings:notifications",
        ))
        .map_err(|e| e.to_string())?;
        let opened = windows::System::Launcher::LaunchUriAsync(&uri)
            .map_err(|e| e.to_string())?
            .get()
            .map_err(|e| e.to_string())?;
        if opened {
            Ok(())
        } else {
            Err("无法打开设置。请手动打开：设置 → 系统 → 通知 → Daymark".into())
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("请在操作系统设置中打开通知设置".into())
    }
}
pub fn send(app: &tauri::AppHandle, item: &Item, sound: bool) -> Result<(), String> {
    let permission = status(app);
    if !permission.can_send() {
        return Err(permission.detail);
    }
    let title = format!("Daymark · {}", item.title);
    let body = if item.id == "notification-test" {
        "这是一条测试通知。看到后，请返回 Daymark 选择“看到了”。".to_string()
    } else {
        let when = item
            .target()
            .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
            .map(|t| {
                t.with_timezone(&chrono::Local)
                    .format("%m月%d日 %H:%M")
                    .to_string()
            })
            .unwrap_or_default();
        format!(
            "{when} · {}",
            if item.kind == "task" {
                "任务即将截止"
            } else {
                "日程即将开始"
            }
        )
    };
    #[cfg(target_os = "macos")]
    {
        let mut n = mac_usernotifications::Notification::new()
            .title(title)
            .message(body);
        if sound {
            n = n.default_sound();
        }
        n.send_blocking()
            .map_err(|e| format!("系统未接受通知：{e}"))?;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        let mut n = notify_rust::Notification::new();
        n.summary(&title)
            .body(&body)
            .app_id(&app.config().identifier);
        if sound {
            n.sound_name("Default");
        }
        n.show().map_err(|e| format!("系统未接受通知：{e}"))?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (title, body, sound);
        Err("此平台暂不支持系统通知".into())
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestState {
    pub id: String,
    pub phase: String,
    pub due_at: Option<i64>,
    pub detail: String,
}
impl Default for TestState {
    fn default() -> Self {
        Self {
            id: String::new(),
            phase: "idle".into(),
            due_at: None,
            detail: String::new(),
        }
    }
}
pub fn test_state(app: &tauri::AppHandle) -> Result<TestState, String> {
    Ok(app
        .state::<RuntimeState>()
        .notification_test
        .lock()
        .map_err(|_| "通知测试状态不可用")?
        .clone())
}
fn set_test(app: &tauri::AppHandle, s: TestState) -> Result<(), String> {
    *app.state::<RuntimeState>()
        .notification_test
        .lock()
        .map_err(|_| "通知测试状态不可用")? = s.clone();
    let _ = app.emit("notification-test", s);
    Ok(())
}
pub fn start_test(app: tauri::AppHandle) -> Result<TestState, String> {
    let permission = status(&app);
    if !permission.can_send() {
        return Err(permission.detail);
    }
    // One backend-owned countdown survives switching pages, hiding the main window or webview throttling.
    let s = TestState {
        id: uuid::Uuid::new_v4().to_string(),
        phase: "countdown".into(),
        due_at: Some(chrono::Utc::now().timestamp_millis() + 5000),
        detail: "5 秒后发送，可切到其他应用查看。".into(),
    };
    set_test(&app, s.clone())?;
    let id = s.id.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(5));
        let Ok(current) = test_state(&app) else {
            return;
        };
        if current.id != id || current.phase != "countdown" {
            return;
        }
        let result = app.state::<Database>().settings().and_then(|settings| {
            send(
                &app,
                &Item {
                    id: "notification-test".into(),
                    title: "通知自测".into(),
                    notes: String::new(),
                    kind: "event".into(),
                    starts_at: Some(chrono::Utc::now().timestamp()),
                    ends_at: None,
                    due_at: None,
                    completed_at: None,
                    snooze_until: None,
                    version: 0,
                },
                settings.sound,
            )
        });
        let Ok(latest) = test_state(&app) else {
            return;
        };
        if latest.id != id {
            return;
        }
        let (phase, detail) = match result {
            Ok(()) => ("accepted", "已交给系统。你看到测试通知了吗？".to_string()),
            Err(e) => ("failed", e),
        };
        let _ = set_test(
            &app,
            TestState {
                id,
                phase: phase.into(),
                due_at: None,
                detail,
            },
        );
    });
    Ok(s)
}
pub fn cancel_test(app: &tauri::AppHandle) -> Result<(), String> {
    set_test(app, TestState::default())
}
