use crate::model::{Item, Settings};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

pub struct Database(pub Mutex<Connection>);
const COLUMNS: &str =
    "id,title,notes,kind,starts_at,ends_at,due_at,completed_at,snooze_until,version";
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
    })
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
            completed_at INTEGER, snooze_until INTEGER, version INTEGER NOT NULL DEFAULT 1
        );
        CREATE INDEX IF NOT EXISTS items_pending ON items(completed_at, starts_at, due_at);
        CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), json TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS deliveries (
            item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
            version INTEGER NOT NULL, stage TEXT NOT NULL, scheduled_at INTEGER NOT NULL,
            status TEXT NOT NULL, delivered_at INTEGER NOT NULL,
            PRIMARY KEY(item_id,version,stage,scheduled_at)
        );
        PRAGMA user_version=1;
        COMMIT;").map_err(|e|e.to_string())?;
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
                "SELECT {COLUMNS} FROM items ORDER BY COALESCE(starts_at,due_at),id"
            ))
            .map_err(|e| e.to_string())?;
        let items = s
            .query_map([], row)
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        Ok(items)
    }
    pub fn get(&self, id: &str) -> Result<Item, String> {
        self.lock()?
            .query_row(
                &format!("SELECT {COLUMNS} FROM items WHERE id=?1"),
                [id],
                row,
            )
            .map_err(|e| e.to_string())
    }
    pub fn save(&self, mut item: Item) -> Result<Item, String> {
        item.title = item.title.trim().into();
        item.validate()?;
        let c = self.lock()?;
        if item.id.is_empty() {
            item.id = uuid::Uuid::new_v4().to_string();
            item.version = 1;
            item.completed_at = None;
            item.snooze_until = None;
            c.execute("INSERT INTO items(id,title,notes,kind,starts_at,ends_at,due_at,version) VALUES (?1,?2,?3,?4,?5,?6,?7,1)",params![item.id,item.title,item.notes,item.kind,item.starts_at,item.ends_at,item.due_at]).map_err(|e|e.to_string())?;
        } else {
            let changed=c.execute("UPDATE items SET title=?2,notes=?3,kind=?4,starts_at=?5,ends_at=?6,due_at=?7,snooze_until=NULL,version=version+1 WHERE id=?1 AND version=?8",params![item.id,item.title,item.notes,item.kind,item.starts_at,item.ends_at,item.due_at,item.version]).map_err(|e|e.to_string())?;
            if changed == 0 {
                return Err("内容已在其他窗口更新，请关闭编辑框后重新编辑".into());
            }
            item.version += 1;
            item.snooze_until = None;
        }
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
    pub fn complete(&self, id: &str, completed: bool, now: i64) -> Result<(), String> {
        let n=self.lock()?.execute("UPDATE items SET completed_at=?2,snooze_until=NULL,version=version+1 WHERE id=?1 AND (completed_at IS NOT NULL) != ?3",params![id,if completed {Some(now)} else {None},completed]).map_err(|e|e.to_string())?;
        if n == 0 {
            self.get(id)?;
        }
        Ok(())
    }
    pub fn snooze(&self, id: &str, until: i64) -> Result<(), String> {
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
    pub fn set_settings(&self, s: &Settings) -> Result<(), String> {
        s.validate()?;
        self.lock()?.execute("INSERT INTO settings(id,json) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET json=excluded.json",[serde_json::to_string(s).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
        Ok(())
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
        d.complete(&edit.id, true, 123).unwrap();
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
