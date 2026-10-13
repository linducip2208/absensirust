//! Repository SQLite: semua query memakai parameter binding.
//! Transaksi dipakai untuk operasi penting (ingest, koreksi, assignment, dsb).

use crate::errors::{AppError, AppResult};
use crate::timex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Repo {
    pub(crate) conn: Mutex<Connection>,
    pub path: PathBuf,
}

impl Repo {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path).map_err(|e| AppError::Db(e.to_string()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        crate::db::migrations::ensure_migrations(&conn).map_err(|e| AppError::Db(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        })
    }

    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory().map_err(|e| AppError::Db(e.to_string()))?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        crate::db::migrations::ensure_migrations(&conn).map_err(|e| AppError::Db(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
        })
    }

    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|e| AppError::Db(format!("lock: {e}")))
    }

    pub fn query_rows<T, F>(
        &self,
        sql: &str,
        p: &[&dyn rusqlite::ToSql],
        map: F,
    ) -> AppResult<Vec<T>>
    where
        F: Fn(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        let g = self.lock()?;
        let mut stmt = g.prepare(sql)?;
        let rows = stmt.query_map(p, map)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn exec(&self, sql: &str, p: &[&dyn rusqlite::ToSql]) -> AppResult<usize> {
        let g = self.lock()?;
        Ok(g.execute(sql, p)?)
    }

    pub fn query_one<T, F>(
        &self,
        sql: &str,
        p: &[&dyn rusqlite::ToSql],
        map: F,
    ) -> AppResult<Option<T>>
    where
        F: FnOnce(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        let g = self.lock()?;
        let mut stmt = g.prepare(sql)?;
        Ok(stmt.query_row(p, map).optional()?)
    }

    pub fn count(
        &self,
        table: &str,
        extra_where: &str,
        p: &[&dyn rusqlite::ToSql],
    ) -> AppResult<i64> {
        let allowed = [
            "employees",
            "devices",
            "raw_events",
            "attendance_results",
            "leave_requests",
            "overtime_requests",
            "shifts",
            "branches",
            "departments",
        ];
        if !allowed.contains(&table) {
            return Err(AppError::Validation("tabel tidak dikenal".into()));
        }
        let sql = format!("SELECT COUNT(*) FROM {table} {extra_where}");
        Ok(self.query_one(&sql, p, |r| r.get(0))?.unwrap_or(0))
    }

    // ---------- generic helpers dipakai commands ----------
    pub fn insert_audit(
        &self,
        actor: &str,
        action: &str,
        entity: &str,
        entity_id: &str,
        detail: &str,
    ) -> AppResult<()> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = timex::now_wib_string();
        self.exec(
            "INSERT INTO audit_log (id, actor, action, entity, entity_id, detail, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            &[&id, &actor, &action, &entity, &entity_id, &detail, &now],
        )?;
        Ok(())
    }

    pub fn paginate(
        &self,
        base_sql: &str,
        count_sql: &str,
        p: &[&dyn rusqlite::ToSql],
        page: i64,
        per_page: i64,
    ) -> AppResult<(Vec<serde_json::Value>, i64)> {
        let total: i64 = self.query_one(count_sql, p, |r| r.get(0))?.unwrap_or(0);
        let page = page.max(1);
        let per_page = per_page.clamp(1, 200);
        let offset = (page - 1) * per_page;
        let sql = format!("{base_sql} LIMIT {per_page} OFFSET {offset}");
        let g = self.lock()?;
        let mut stmt = g.prepare(&sql)?;
        let cols: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let rows = stmt
            .query_map(p, |row| {
                let mut m = serde_json::Map::new();
                for (i, c) in cols.iter().enumerate() {
                    let v: rusqlite::types::Value = row.get(i)?;
                    m.insert(c.clone(), rusqlite_value_to_json(v));
                }
                Ok(serde_json::Value::Object(m))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok((rows, total))
    }

    pub fn transaction_ingest_events(&self, events: &[RawEventIn]) -> AppResult<IngestReport> {
        let mut g = self.lock()?;
        let tx = g.transaction()?;
        let mut inserted = 0i64;
        let mut duplicates = 0i64;
        let mut unmapped = 0i64;
        let now = timex::now_wib_string();
        for e in events {
            validate_event_time(&e.event_time)?;
            let mapped: Option<String> = tx.query_row(
                "SELECT employee_id FROM employee_device_map WHERE device_id=?1 AND device_user_id=?2",
                params![e.device_id, e.device_user_id],
                |r| r.get(0),
            ).optional()?;
            if mapped.is_none() {
                unmapped += 1;
            }
            let id = uuid::Uuid::new_v4().to_string();
            let res = tx.execute(
                "INSERT OR IGNORE INTO raw_events (id, device_id, vendor_record_id, device_user_id, event_time, direction, payload_json, status, ingested_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,'pending',?8)",
                params![id, e.device_id, e.vendor_record_id, e.device_user_id, e.event_time, e.direction, e.payload_json, now],
            )?;
            if res == 1 {
                inserted += 1;
            } else {
                duplicates += 1;
            }
        }
        tx.commit()?;
        Ok(IngestReport {
            inserted,
            duplicates,
            unmapped,
        })
    }
}

fn validate_event_time(s: &str) -> AppResult<()> {
    if timex::parse_wib(s).is_none() {
        return Err(AppError::Validation(format!(
            "event_time tidak valid (harap WIB YYYY-MM-DD HH:MM:SS): {s}"
        )));
    }
    Ok(())
}

fn rusqlite_value_to_json(v: rusqlite::types::Value) -> serde_json::Value {
    use rusqlite::types::Value::*;
    match v {
        Null => serde_json::Value::Null,
        Integer(i) => serde_json::json!(i),
        Real(f) => serde_json::json!(f),
        Text(t) => serde_json::Value::String(t),
        Blob(b) => serde_json::json!(format!("<blob {} bytes>", b.len())),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEventIn {
    pub device_id: String,
    pub vendor_record_id: String,
    pub device_user_id: String,
    pub event_time: String,
    pub direction: String,
    pub payload_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestReport {
    pub inserted: i64,
    pub duplicates: i64,
    pub unmapped: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrations_apply_on_empty() {
        let r = Repo::open_in_memory().unwrap();
        let v: i64 = r
            .query_one("SELECT COUNT(*) FROM schema_migrations", &[], |x| x.get(0))
            .unwrap()
            .unwrap();
        assert!(v >= 7);
    }
    #[test]
    fn ingest_is_idempotent() {
        let r = Repo::open_in_memory().unwrap();
        r.exec("INSERT INTO devices (id,name,vendor,model,adapter_name,sdk_version,status,created_at,updated_at) VALUES ('d1','Sim','Mock','M1','mock','1.0','offline','2026-01-01 00:00:00','2026-01-01 00:00:00')", &[]).unwrap();
        let ev = RawEventIn {
            device_id: "d1".into(),
            vendor_record_id: "REC-1".into(),
            device_user_id: "U1".into(),
            event_time: "2026-05-01 07:05:00".into(),
            direction: "in".into(),
            payload_json: "{}".into(),
        };
        let a = r
            .transaction_ingest_events(std::slice::from_ref(&ev))
            .unwrap();
        let b = r.transaction_ingest_events(&[ev]).unwrap();
        assert_eq!(a.inserted, 1);
        assert_eq!(b.duplicates, 1);
        assert_eq!(b.unmapped, 1);
    }
    #[test]
    fn ingest_rejects_bad_time() {
        let r = Repo::open_in_memory().unwrap();
        let ev = RawEventIn {
            device_id: "d1".into(),
            vendor_record_id: "R".into(),
            device_user_id: "U".into(),
            event_time: "not-a-time".into(),
            direction: "in".into(),
            payload_json: "{}".into(),
        };
        assert!(r.transaction_ingest_events(&[ev]).is_err());
    }
}
