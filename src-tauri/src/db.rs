use crate::model::{Item, OccurrenceState, Settings, SettingsPatch};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

pub struct Database(pub Mutex<Connection>);
const COLUMNS: &str =
    "id,title,notes,kind,starts_at,ends_at,due_at,completed_at,snooze_until,version,scheduled_at,recurrence";
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Item> {
    Ok(Item {
        id: r.get(0)?,
        title: r.get(1)?,
        notes: r.get(2)?,
        kind: r.get(3)?,
        starts_at: r.get(4)?,
        ends_at: r.get(5)?,
        due_at: r.get(6)?,
        completed_at: r.get(7)?,
        snooze_until: r.get(8)?,
        version: r.get(9)?,
        scheduled_at: r.get(10)?,
        recurrence: r
            .get::<_, Option<String>>(11)?
            .map(|raw| {
                serde_json::from_str(&raw).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        11,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })
            })
            .transpose()?,
        ..Item::default()
    })
}
fn load_states(c: &Connection, item: &mut Item) -> Result<(), String> {
    let mut statement=c.prepare("SELECT occurrence_date,completed_at,snooze_until FROM occurrence_states WHERE item_id=?1").map_err(|e|e.to_string())?;
    item.occurrence_states = statement
        .query_map([&item.id], |r| {
            Ok((
                r.get(0)?,
                OccurrenceState {
                    completed_at: r.get(1)?,
                    snooze_until: r.get(2)?,
                },
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())?;
    Ok(())
}
impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        Self::from_connection(Connection::open(path).map_err(|e| e.to_string())?)
    }
    pub fn from_connection(c: Connection) -> Result<Self, String> {
        c.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
        BEGIN;
        CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY, title TEXT NOT NULL, notes TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL CHECK(kind IN ('event','task')), starts_at INTEGER, ends_at INTEGER, due_at INTEGER,
            completed_at INTEGER, snooze_until INTEGER, version INTEGER NOT NULL DEFAULT 1, scheduled_at INTEGER, recurrence TEXT
        );
        CREATE INDEX IF NOT EXISTS items_pending ON items(completed_at, starts_at, due_at);
        CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), json TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS deliveries (
            item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
            version INTEGER NOT NULL, stage TEXT NOT NULL, scheduled_at INTEGER NOT NULL,
            status TEXT NOT NULL, delivered_at INTEGER NOT NULL,
            PRIMARY KEY(item_id,version,stage,scheduled_at)
        );
        ").map_err(|e|e.to_string())?;
        let columns = c
            .prepare("PRAGMA table_info(items)")
            .map_err(|e| e.to_string())?
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        for (name, kind) in [("scheduled_at", "INTEGER"), ("recurrence", "TEXT")] {
            if !columns.iter().any(|column| column == name) {
                c.execute(&format!("ALTER TABLE items ADD COLUMN {name} {kind}"), [])
                    .map_err(|e| e.to_string())?;
            }
        }
        c.execute_batch(
            "CREATE TABLE IF NOT EXISTS occurrence_states (
            item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
            occurrence_date TEXT NOT NULL, completed_at INTEGER, snooze_until INTEGER,
            PRIMARY KEY(item_id,occurrence_date)); PRAGMA user_version=2; COMMIT;",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self(Mutex::new(c)))
    }
    pub fn lock(&self) -> Result<MutexGuard<'_, Connection>, String> {
        self.0
            .lock()
            .map_err(|_| "数据库锁不可用，请重启应用".into())
    }
    pub fn list(&self) -> Result<Vec<Item>, String> {
        let c = self.lock()?;
        let mut s = c
            .prepare(&format!(
                "SELECT {COLUMNS} FROM items ORDER BY COALESCE(scheduled_at,starts_at,due_at),id"
            ))
            .map_err(|e| e.to_string())?;
        let mut items = s
            .query_map([], row)
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        for item in &mut items {
            load_states(&c, item)?;
        }
        Ok(items)
    }
    pub fn get(&self, id: &str) -> Result<Item, String> {
        let c = self.lock()?;
        let mut item = c
            .query_row(
                &format!("SELECT {COLUMNS} FROM items WHERE id=?1"),
                [id],
                row,
            )
            .map_err(|e| e.to_string())?;
        load_states(&c, &mut item)?;
        Ok(item)
    }
    pub fn current(&self, instance: &Item) -> Result<Item, String> {
        let item = self.get(&instance.id)?;
        if let Some(date) = &instance.occurrence_date {
            crate::recurrence::on_date(&item, crate::recurrence::parse_date(date)?, &chrono::Local)
                .ok_or_else(|| "这次重复任务已不再存在".into())
        } else {
            Ok(item)
        }
    }
    pub fn save(&self, mut item: Item) -> Result<Item, String> {
        item.title = item.title.trim().into();
        item.validate()?;
        if item.recurrence.is_some() {
            item.scheduled_at = None;
            item.completed_at = None;
        }
        let recurrence = item
            .recurrence
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| e.to_string())?;
        let mut c = self.lock()?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        if item.id.is_empty() {
            item.id = uuid::Uuid::new_v4().to_string();
            item.version = 1;
            item.completed_at = None;
            item.snooze_until = None;
            tx.execute("INSERT INTO items(id,title,notes,kind,starts_at,ends_at,due_at,scheduled_at,recurrence,version) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,1)",params![item.id,item.title,item.notes,item.kind,item.starts_at,item.ends_at,item.due_at,item.scheduled_at,recurrence]).map_err(|e|e.to_string())?;
        } else {
            let changed=tx.execute("UPDATE items SET title=?2,notes=?3,kind=?4,starts_at=?5,ends_at=?6,due_at=?7,scheduled_at=?9,recurrence=?10,completed_at=CASE WHEN ?10 IS NOT NULL THEN NULL ELSE completed_at END,snooze_until=NULL,version=version+1 WHERE id=?1 AND version=?8",params![item.id,item.title,item.notes,item.kind,item.starts_at,item.ends_at,item.due_at,item.version,item.scheduled_at,recurrence]).map_err(|e|e.to_string())?;
            if changed == 0 {
                return Err("内容已在其他窗口更新，请关闭编辑框后重新编辑".into());
            }
            item.version += 1;
            item.snooze_until = None;
        }
        tx.execute(
            "UPDATE occurrence_states SET snooze_until=NULL WHERE item_id=?1",
            [&item.id],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        drop(c);
        self.get(&item.id)
    }
    pub fn delete(&self, id: &str, version: i64) -> Result<(), String> {
        let n = self
            .lock()?
            .execute(
                "DELETE FROM items WHERE id=?1 AND version=?2",
                params![id, version],
            )
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("内容已更新，请刷新后重试".into());
        }
        Ok(())
    }
    pub fn complete(
        &self,
        id: &str,
        completed: bool,
        now: i64,
        date: Option<&str>,
    ) -> Result<(), String> {
        let item = self.get(id)?;
        if item.recurrence.is_some() {
            let date = date
                .map(str::to_string)
                .unwrap_or_else(|| chrono::Local::now().date_naive().to_string());
            if crate::recurrence::on_date(
                &item,
                crate::recurrence::parse_date(&date)?,
                &chrono::Local,
            )
            .is_none()
            {
                return Err("重复日期无效".into());
            }
            self.lock()?.execute("INSERT INTO occurrence_states VALUES(?1,?2,?3,NULL) ON CONFLICT(item_id,occurrence_date) DO UPDATE SET completed_at=excluded.completed_at,snooze_until=NULL",params![id,date,if completed {Some(now)} else {None}]).map_err(|e|e.to_string())?;
            return Ok(());
        }
        let n=self.lock()?.execute("UPDATE items SET completed_at=?2,snooze_until=NULL,version=version+1 WHERE id=?1 AND (completed_at IS NOT NULL) != ?3",params![id,if completed {Some(now)} else {None},completed]).map_err(|e|e.to_string())?;
        if n == 0 {
            self.get(id)?;
        }
        Ok(())
    }
    pub fn snooze(&self, id: &str, until: i64, date: Option<&str>) -> Result<(), String> {
        let item = self.get(id)?;
        if item.recurrence.is_some() {
            let date = date.ok_or("重复任务缺少日期")?;
            let instance = crate::recurrence::on_date(
                &item,
                crate::recurrence::parse_date(date)?,
                &chrono::Local,
            )
            .ok_or("重复日期无效")?;
            if instance.completed_at.is_some() {
                return Err("任务已完成".into());
            }
            self.lock()?.execute("INSERT INTO occurrence_states VALUES(?1,?2,NULL,?3) ON CONFLICT(item_id,occurrence_date) DO UPDATE SET snooze_until=excluded.snooze_until",params![id,date,until]).map_err(|e|e.to_string())?;
            return Ok(());
        }
        let n=self.lock()?.execute("UPDATE items SET snooze_until=?2,version=version+1 WHERE id=?1 AND completed_at IS NULL",params![id,until]).map_err(|e|e.to_string())?;
        if n == 0 {
            return Err("日程已完成或已删除".into());
        }
        Ok(())
    }
    pub fn settings(&self) -> Result<Settings, String> {
        let raw: Option<String> = self
            .lock()?
            .query_row("SELECT json FROM settings WHERE id=1", [], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        raw.map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
            .unwrap_or_else(|| Ok(Settings::default()))
    }
    #[cfg(test)]
    pub fn set_settings(&self, s: &Settings) -> Result<(), String> {
        s.validate()?;
        self.lock()?.execute("INSERT INTO settings(id,json) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET json=excluded.json",[serde_json::to_string(s).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn patch_settings(&self, patch: SettingsPatch) -> Result<Settings, String> {
        let mut c = self.lock()?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        let raw: Option<String> = tx
            .query_row("SELECT json FROM settings WHERE id=1", [], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        let mut settings = match raw {
            Some(raw) => serde_json::from_str(&raw).map_err(|e| e.to_string())?,
            None => Settings::default(),
        };
        patch.apply(&mut settings);
        settings.validate()?;
        tx.execute("INSERT INTO settings(id,json) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET json=excluded.json",[serde_json::to_string(&settings).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(settings)
    }
    pub fn delivered(&self, id: &str, version: i64, stage: &str, at: i64) -> Result<bool, String> {
        self.lock()?.query_row("SELECT EXISTS(SELECT 1 FROM deliveries WHERE item_id=?1 AND version=?2 AND stage=?3 AND scheduled_at=?4)",params![id,version,stage,at],|r|r.get(0)).map_err(|e|e.to_string())
    }
    pub fn record(
        &self,
        id: &str,
        version: i64,
        stage: &str,
        at: i64,
        status: &str,
        now: i64,
    ) -> Result<(), String> {
        self.lock()?
            .execute(
                "INSERT OR IGNORE INTO deliveries VALUES (?1,?2,?3,?4,?5,?6)",
                params![id, version, stage, at, status, now],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Database {
        Database::from_connection(Connection::open_in_memory().unwrap()).unwrap()
    }
    pub fn example() -> Item {
        Item {
            id: "".into(),
            title: " 测试任务 ".into(),
            notes: "本地".into(),
            kind: "task".into(),
            starts_at: None,
            ends_at: None,
            due_at: Some(2_000_000_000),
            completed_at: None,
            snooze_until: None,
            version: 0,
            ..Item::default()
        }
    }
    #[test]
    fn crud_completion_and_conflict() {
        let d = db();
        let item = d.save(example()).unwrap();
        assert_eq!(item.title, "测试任务");
        let mut edit = item.clone();
        edit.title = "更新".into();
        let edit = d.save(edit).unwrap();
        assert!(d.save(item).is_err());
        d.complete(&edit.id, true, 123, None).unwrap();
        let completed = d.get(&edit.id).unwrap();
        assert_eq!(completed.completed_at, Some(123));
        assert!(d.delete(&edit.id, edit.version).is_err());
        d.delete(&edit.id, completed.version).unwrap();
        assert!(d.list().unwrap().is_empty());
    }
    #[test]
    fn validation_and_settings() {
        let d = db();
        let mut i = example();
        i.kind = "event".into();
        assert!(d.save(i).is_err());
        let mut s = Settings {
            notification_minutes: vec![5, 5],
            ..Settings::default()
        };
        assert!(d.set_settings(&s).is_err());
        s.notification_minutes = vec![15, 3];
        d.set_settings(&s).unwrap();
        assert_eq!(s, d.settings().unwrap());
    }
    #[test]
    fn persistence_and_delivery_dedup() {
        let path = std::env::temp_dir().join(format!("daymark-test-{}.db", uuid::Uuid::new_v4()));
        let id;
        {
            let d = Database::open(&path).unwrap();
            let i = d.save(example()).unwrap();
            id = i.id;
            d.record(&id, 1, "n5", 100, "sent", 101).unwrap();
            d.record(&id, 1, "n5", 100, "sent", 102).unwrap();
        }
        {
            let d = Database::open(&path).unwrap();
            assert_eq!(d.list().unwrap().len(), 1);
            assert!(d.delivered(&id, 1, "n5", 100).unwrap());
            d.delete(&id, 1).unwrap();
            assert!(!d.delivered(&id, 1, "n5", 100).unwrap());
        }
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn legacy_settings_and_items_survive_patches() {
        let d = Database::from_connection(Connection::open_in_memory().unwrap()).unwrap();
        d.lock().unwrap().execute("INSERT INTO settings VALUES(1,?1)",[r#"{"notificationMinutes":[15,3],"sound":false,"fullscreen":false,"sidebarSide":"left"}"#]).unwrap();
        let item = d.save(super::tests::example()).unwrap();
        d.complete(&item.id, true, 123, None).unwrap();
        let s = d.settings().unwrap();
        assert_eq!(s.accent_color, "#4f6bed");
        assert_eq!(s.onboarding_version, 0);
        assert!(!s.sound);
        d.patch_settings(SettingsPatch {
            theme_mode: Some("dark".into()),
            ..Default::default()
        })
        .unwrap();
        d.patch_settings(SettingsPatch {
            sidebar_enabled: Some(true),
            ..Default::default()
        })
        .unwrap();
        let s = d.settings().unwrap();
        assert_eq!(s.theme_mode, "dark");
        assert!(s.sidebar_enabled);
        assert_eq!(s.notification_minutes, vec![15, 3]);
        assert_eq!(s.sidebar_side, "left");
        assert!(!s.sound);
        assert_eq!(d.get(&item.id).unwrap().completed_at, Some(123));
        assert!(d
            .patch_settings(SettingsPatch {
                accent_color: Some("invalid".into()),
                ..Default::default()
            })
            .is_err());
        assert_eq!(d.settings().unwrap().theme_mode, "dark");
    }
}

#[cfg(test)]
mod routine_tests {
    use super::*;
    use crate::{model::DailyRecurrence, recurrence};
    #[test]
    fn untimed_single_time_and_daily_completion_are_independent_and_persist() {
        let path =
            std::env::temp_dir().join(format!("daymark-routine-{}.db", uuid::Uuid::new_v4()));
        let id;
        {
            let d = Database::open(&path).unwrap();
            let task = Item {
                kind: "task".into(),
                title: "只需要标题".into(),
                ..Default::default()
            };
            let plain = d.save(task.clone()).unwrap();
            assert!(plain.target().is_none());
            let point = d
                .save(Item {
                    scheduled_at: Some(2_000_000_000),
                    ..task.clone()
                })
                .unwrap();
            assert!(point.starts_at.is_none() && point.ends_at.is_none() && point.due_at.is_none());
            assert_eq!(point.target(), Some(2_000_000_000));
            let series = d
                .save(Item {
                    recurrence: Some(DailyRecurrence {
                        start_date: "2026-11-07".into(),
                        time: "21:00".into(),
                    }),
                    ..task
                })
                .unwrap();
            id = series.id.clone();
            d.complete(&id, true, 123, Some("2026-11-07")).unwrap();
            let updated = d.get(&id).unwrap();
            assert_eq!(updated.version, series.version);
            let today = recurrence::on_date(
                &updated,
                recurrence::parse_date("2026-11-07").unwrap(),
                &chrono::Utc,
            )
            .unwrap();
            let tomorrow = recurrence::on_date(
                &updated,
                recurrence::parse_date("2026-11-08").unwrap(),
                &chrono::Utc,
            )
            .unwrap();
            assert_eq!(today.completed_at, Some(123));
            assert_eq!(tomorrow.completed_at, None);
            assert!(crate::scheduler::stages(&today, &Settings::default()).is_empty());
            assert!(crate::scheduler::stages(&tomorrow, &Settings::default())
                .iter()
                .any(|s| s.name == "at" && s.at == tomorrow.target().unwrap()));
            d.record(
                &id,
                series.version,
                "at",
                today.target().unwrap(),
                "sent",
                123,
            )
            .unwrap();
            assert!(!d
                .delivered(&id, series.version, "at", tomorrow.target().unwrap())
                .unwrap());
            d.snooze(&id, 300, Some("2026-11-08")).unwrap();
            assert_eq!(
                d.get(&id).unwrap().occurrence_states["2026-11-08"].snooze_until,
                Some(300)
            );
            d.save(Item {
                title: "改名保留历史".into(),
                ..updated
            })
            .unwrap();
        }
        {
            let d = Database::open(&path).unwrap();
            let series = d.get(&id).unwrap();
            assert_eq!(
                series.occurrence_states["2026-11-07"].completed_at,
                Some(123)
            );
            assert_eq!(series.occurrence_states["2026-11-08"].completed_at, None);
            assert_eq!(series.occurrence_states["2026-11-08"].snooze_until, None);
            assert!(d.complete(&id, true, 123, Some("2026-11-06")).is_err());
            d.delete(&id, series.version).unwrap();
            assert_eq!(
                d.lock()
                    .unwrap()
                    .query_row("SELECT count(*) FROM occurrence_states", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn migrates_011_schema_without_replacing_legacy_tasks_or_settings() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE items(id TEXT PRIMARY KEY,title TEXT NOT NULL,notes TEXT NOT NULL,kind TEXT NOT NULL,starts_at INTEGER,ends_at INTEGER,due_at INTEGER,completed_at INTEGER,snooze_until INTEGER,version INTEGER NOT NULL);
            INSERT INTO items VALUES('old','旧任务','保留','task',NULL,NULL,2000000000,123,NULL,7);
            CREATE TABLE settings(id INTEGER PRIMARY KEY,json TEXT NOT NULL);
            INSERT INTO settings VALUES(1,'{\"notificationMinutes\":[15,3],\"sound\":false,\"sidebarSide\":\"left\"}');PRAGMA user_version=1;").unwrap();
        let d = Database::from_connection(c).unwrap();
        let item = d.get("old").unwrap();
        assert_eq!(item.version, 7);
        assert_eq!(item.completed_at, Some(123));
        assert_eq!(item.due_at, Some(2000000000));
        assert!(item.recurrence.is_none() && item.scheduled_at.is_none());
        assert_eq!(d.settings().unwrap().notification_minutes, vec![15, 3]);
        assert!(!d.settings().unwrap().sound);
        assert_eq!(
            d.lock()
                .unwrap()
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
}
