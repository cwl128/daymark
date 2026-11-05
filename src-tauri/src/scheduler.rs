use crate::{
    db::Database,
    model::{Item, Settings},
    windows::{self, Alert, RuntimeState},
};
use std::collections::HashMap;
use tauri::Manager;

const GRACE: i64 = 15 * 60;
#[derive(Debug, Clone, PartialEq)]
pub struct Stage {
    pub name: String,
    pub at: i64,
    pub fullscreen: bool,
}
pub fn stages(item: &Item, s: &Settings) -> Vec<Stage> {
    if item.completed_at.is_some() {
        return vec![];
    }
    if let Some(at) = item.snooze_until {
        return vec![Stage {
            name: "snooze".into(),
            at,
            fullscreen: s.fullscreen,
        }];
    }
    let Some(target) = item.target() else {
        return vec![];
    };
    let mut stages: Vec<_> = s
        .notification_minutes
        .iter()
        .map(|minutes| Stage {
            name: format!("n{minutes}"),
            at: target - minutes * 60,
            fullscreen: false,
        })
        .collect();
    if s.fullscreen {
        stages.push(Stage {
            name: "fullscreen".into(),
            at: target - 60,
            fullscreen: true,
        });
    }
    stages.sort_by_key(|s| std::cmp::Reverse(s.at));
    stages
}

// The scheduler is independent of all webviews. Recompute against wall time on every tick;
// suspension, clock and timezone changes never leave a long-lived JS timeout stale.
pub fn start(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut retries = HashMap::<String, i64>::new();
        loop {
            let now = chrono::Utc::now().timestamp();
            retries.retain(|_, t| now >= *t && now - *t < GRACE);
            if let Err(error) = tick(&app, now, &mut retries) {
                eprintln!("reminder scheduler: {error}");
                if let Ok(mut s) = app.state::<RuntimeState>().last_error.lock() {
                    *s = Some(error);
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}
fn tick(
    app: &tauri::AppHandle,
    now: i64,
    retries: &mut HashMap<String, i64>,
) -> Result<(), String> {
    let db = app.state::<Database>();
    let settings = db.settings()?;
    for item in db.list()? {
        let due: Vec<_> = stages(&item, &settings)
            .into_iter()
            .filter(|s| s.at <= now)
            .collect();
        let Some(latest) = due.first() else {
            continue;
        };
        let key = format!("{}:{}:{}:{}", item.id, item.version, latest.name, latest.at);
        let already = db.delivered(&item.id, item.version, &latest.name, latest.at)?;
        if !already && now - latest.at <= GRACE {
            if retries.get(&key).is_some_and(|t| now - *t < 30) {
                continue;
            }
            let current = db.get(&item.id)?;
            if current.completed_at.is_some() || current.version != item.version {
                continue;
            }
            let result = if latest.fullscreen {
                let handle = app.clone();
                let alert = Alert {
                    item: item.clone(),
                    scheduled_at: latest.at,
                };
                let (sender, receiver) = std::sync::mpsc::channel();
                app.run_on_main_thread(move || {
                    let _ = sender.send(windows::show_alert(&handle, alert));
                })
                .map_err(|e| e.to_string())?;
                receiver
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .map_err(|e| e.to_string())?
            } else {
                send_notification(app, &item, settings.sound).map(|_| true)
            };
            match result {
                Ok(true) => {
                    db.record(&item.id, item.version, &latest.name, latest.at, "sent", now)?;
                    if let Ok(mut s) = app.state::<RuntimeState>().last_error.lock() {
                        *s = None;
                    }
                }
                Ok(false) => continue,
                Err(error) => {
                    retries.insert(key, now);
                    return Err(error);
                }
            }
        }
        // A more urgent stage wins after resume. Suppress earlier stages, including across restarts.
        for stage in due {
            if !db.delivered(&item.id, item.version, &stage.name, stage.at)? {
                db.record(
                    &item.id,
                    item.version,
                    &stage.name,
                    stage.at,
                    if now - stage.at > GRACE {
                        "expired"
                    } else {
                        "superseded"
                    },
                    now,
                )?;
            }
        }
    }
    Ok(())
}
pub fn send_notification(app: &tauri::AppHandle, item: &Item, sound: bool) -> Result<(), String> {
    let mut n = notify_rust::Notification::new();
    n.summary(&format!("Daymark · {}", item.title));
    let when = item
        .target()
        .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%m月%d日 %H:%M")
                .to_string()
        })
        .unwrap_or_default();
    n.body(&format!(
        "{when} · {}",
        if item.kind == "task" {
            "任务即将截止"
        } else {
            "日程即将开始"
        }
    ));
    #[cfg(target_os = "macos")]
    {
        let _ = notify_rust::set_application(if tauri::is_dev() {
            "com.apple.Terminal"
        } else {
            &app.config().identifier
        });
        if sound {
            n.sound_name("NSUserNotificationDefaultSoundName");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if !tauri::is_dev() {
            n.app_id(&app.config().identifier);
        }
        if sound {
            n.sound_name("Default");
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = app;
        if sound {
            n.sound_name("message-new-instant");
        } else {
            n.hint(notify_rust::Hint::SuppressSound(true));
        }
    }
    // Unlike the plugin's desktop fire-and-forget helper, return the native API error so
    // the scheduler can retry. API acceptance still cannot guarantee presentation through DND.
    n.show().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item() -> Item {
        Item {
            id: "a".into(),
            title: "测试".into(),
            notes: "".into(),
            kind: "event".into(),
            starts_at: Some(10_000),
            ends_at: Some(11_000),
            due_at: None,
            completed_at: None,
            snooze_until: None,
            version: 1,
        }
    }
    #[test]
    fn default_timing() {
        let s = stages(&item(), &Settings::default());
        assert_eq!(
            s.iter().map(|s| s.at).collect::<Vec<_>>(),
            vec![9940, 9700, 9400]
        );
    }
    #[test]
    fn completion_cancels_all() {
        let mut i = item();
        i.completed_at = Some(9_000);
        assert!(stages(&i, &Settings::default()).is_empty());
    }
    #[test]
    fn task_uses_deadline() {
        let mut i = item();
        i.kind = "task".into();
        i.due_at = Some(20_000);
        assert_eq!(stages(&i, &Settings::default())[0].at, 19_940);
    }
    #[test]
    fn snooze_is_exact_five_minutes_no_early_repeat() {
        let mut i = item();
        i.snooze_until = Some(12_000);
        let s = stages(&i, &Settings::default());
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].at, 12_000);
    }
    #[test]
    fn resume_picks_most_urgent_and_ignores_future() {
        let s = stages(&item(), &Settings::default());
        let due: Vec<_> = s.iter().filter(|s| s.at <= 9_980).collect();
        assert_eq!(due[0].name, "fullscreen");
        let before: Vec<_> = s.iter().filter(|s| s.at <= 9_500).collect();
        assert_eq!(before[0].name, "n10");
    }
    #[test]
    fn disabled_fullscreen_and_custom_minutes() {
        let s = Settings {
            fullscreen: false,
            notification_minutes: vec![20, 3],
            ..Settings::default()
        };
        let list = stages(&item(), &s);
        assert_eq!(list[0].at, 9820);
        assert!(list.iter().all(|s| !s.fullscreen));
    }
}
