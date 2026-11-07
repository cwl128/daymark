use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DailyRecurrence {
    pub start_date: String,
    pub time: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OccurrenceState {
    pub completed_at: Option<i64>,
    pub snooze_until: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub kind: String,
    pub starts_at: Option<i64>,
    pub ends_at: Option<i64>,
    pub due_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub snooze_until: Option<i64>,
    pub version: i64,
    #[serde(default)]
    pub scheduled_at: Option<i64>,
    #[serde(default)]
    pub recurrence: Option<DailyRecurrence>,
    #[serde(default)]
    pub occurrence_date: Option<String>,
    #[serde(default)]
    pub occurrence_states: BTreeMap<String, OccurrenceState>,
}
impl Item {
    pub fn target(&self) -> Option<i64> {
        if self.kind == "task" {
            self.scheduled_at.or(self.due_at).or(self.starts_at)
        } else {
            self.starts_at
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() || self.title.chars().count() > 200 {
            return Err("标题需为 1–200 个字符".into());
        }
        if self.notes.chars().count() > 10_000 {
            return Err("备注不能超过 10000 字符".into());
        }
        if !["event", "task"].contains(&self.kind.as_str()) {
            return Err("类型无效".into());
        }
        for time in [self.starts_at, self.ends_at, self.due_at, self.scheduled_at]
            .into_iter()
            .flatten()
        {
            if !(0..=253402300799).contains(&time) {
                return Err("时间超出有效范围".into());
            }
        }
        if self.kind == "event" && (self.starts_at.is_none() || self.ends_at.is_none()) {
            return Err("日程需要开始和结束时间".into());
        }
        if let Some(recurrence) = &self.recurrence {
            if self.kind != "task" {
                return Err("每日重复只用于任务".into());
            }
            crate::recurrence::parse_date(&recurrence.start_date)?;
            crate::recurrence::parse_clock(&recurrence.time)?;
        }
        if let (Some(start), Some(end)) = (self.starts_at, self.ends_at) {
            if end <= start {
                return Err("结束时间必须晚于开始时间".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub notification_minutes: Vec<i64>,
    pub sound: bool,
    pub fullscreen: bool,
    pub sidebar_side: String,
    pub sidebar_enabled: bool,
    pub sidebar_auto_hide: bool,
    pub theme_mode: String,
    pub accent_color: String,
    pub background_light: String,
    pub background_dark: String,
    pub onboarding_version: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            notification_minutes: vec![10, 5],
            sound: true,
            fullscreen: true,
            sidebar_side: "right".into(),
            sidebar_enabled: false,
            sidebar_auto_hide: true,
            theme_mode: "system".into(),
            accent_color: "#4f6bed".into(),
            background_light: "#f8fafc".into(),
            background_dark: "#111827".into(),
            onboarding_version: 0,
        }
    }
}
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsPatch {
    pub notification_minutes: Option<Vec<i64>>,
    pub sound: Option<bool>,
    pub fullscreen: Option<bool>,
    pub sidebar_side: Option<String>,
    pub sidebar_enabled: Option<bool>,
    pub sidebar_auto_hide: Option<bool>,
    pub theme_mode: Option<String>,
    pub accent_color: Option<String>,
    pub background_light: Option<String>,
    pub background_dark: Option<String>,
    pub onboarding_version: Option<u32>,
}
impl SettingsPatch {
    pub fn apply(self, s: &mut Settings) {
        macro_rules! patch { ($($field:ident),*) => { $(if let Some(value)=self.$field {s.$field=value;})* }; }
        patch!(
            notification_minutes,
            sound,
            fullscreen,
            sidebar_side,
            sidebar_enabled,
            sidebar_auto_hide,
            theme_mode,
            accent_color,
            background_light,
            background_dark,
            onboarding_version
        );
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !["left", "right"].contains(&self.sidebar_side.as_str()) {
            return Err("侧边位置无效".into());
        }
        if !["system", "light", "dark"].contains(&self.theme_mode.as_str()) {
            return Err("主题无效".into());
        }
        for color in [
            &self.accent_color,
            &self.background_light,
            &self.background_dark,
        ] {
            if color.len() != 7
                || !color.starts_with('#')
                || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            {
                return Err("请选择有效的颜色".into());
            }
        }
        if self.notification_minutes.len() > 4
            || self
                .notification_minutes
                .iter()
                .any(|n| !(2..=1440).contains(n))
        {
            return Err("系统通知提前时间需为 2–1440 分钟，最多 4 档".into());
        }
        let mut unique = self.notification_minutes.clone();
        unique.sort();
        unique.dedup();
        if unique.len() != self.notification_minutes.len() {
            return Err("提醒时间不能重复".into());
        }
        Ok(())
    }
}
