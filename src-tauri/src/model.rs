use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
impl Item {
    pub fn target(&self) -> Option<i64> {
        if self.kind == "task" {
            self.due_at.or(self.starts_at)
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
        for time in [self.starts_at, self.ends_at, self.due_at]
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
        if self.kind == "task" && self.target().is_none() {
            return Err("任务需要开始时间或截止时间".into());
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
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub notification_minutes: Vec<i64>,
    pub sound: bool,
    pub fullscreen: bool,
    pub sidebar_side: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            notification_minutes: vec![10, 5],
            sound: true,
            fullscreen: true,
            sidebar_side: "right".into(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !["left", "right"].contains(&self.sidebar_side.as_str()) {
            return Err("侧边位置无效".into());
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
