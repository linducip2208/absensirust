//! Migrasi versioned. Setiap migrasi tercatat di tabel schema_migrations.
//! Format timestamp: TEXT WIB "YYYY-MM-DD HH:MM:SS".

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "init_core",
        sql: include_str!("migrations/001_init.sql"),
    },
    Migration {
        version: 2,
        name: "shifts_schedule",
        sql: include_str!("migrations/002_shifts.sql"),
    },
    Migration {
        version: 3,
        name: "devices_sync",
        sql: include_str!("migrations/003_devices.sql"),
    },
    Migration {
        version: 4,
        name: "attendance_results",
        sql: include_str!("migrations/004_attendance.sql"),
    },
    Migration {
        version: 5,
        name: "leaves_overtime_users",
        sql: include_str!("migrations/005_hr.sql"),
    },
    Migration {
        version: 6,
        name: "seed_defaults",
        sql: include_str!("migrations/006_seed.sql"),
    },
    Migration {
        version: 7,
        name: "settings_swaps",
        sql: include_str!("migrations/007_settings_swaps.sql"),
    },
    Migration {
        version: 8,
        name: "backup_auto",
        sql: include_str!("migrations/008_backup_auto.sql"),
    },
];

pub fn ensure_migrations(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;
    let applied: Vec<i64> = conn
        .prepare("SELECT version FROM schema_migrations")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    for m in MIGRATIONS {
        if !applied.contains(&m.version) {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(m.sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, datetime('now','localtime'))",
                rusqlite::params![m.version, m.name],
            )?;
            tx.commit()?;
        }
    }
    Ok(())
}
