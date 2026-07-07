//! Data layer foundation: open the local SQLite database, apply embedded
//! migrations in order, expose a shared connection handle.

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

/// Numbered migrations embedded at compile time. Append only — never edit a
/// shipped migration.
const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../migrations/0001_init.sql")),
    (2, include_str!("../migrations/0002_build_targets.sql")),
    (3, include_str!("../migrations/0003_inventory_foundation.sql")),
    (4, include_str!("../migrations/0004_operation_foundation.sql")),
    (5, include_str!("../migrations/0005_reservation_foundation.sql")),
    (6, include_str!("../migrations/0006_blueprint_foundation.sql")),
    (7, include_str!("../migrations/0007_production_requirement_foundation.sql")),
];

pub struct Db {
    pub conn: Mutex<Connection>,
    pub path: PathBuf,
}

impl Db {
    pub fn open(app_data_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("failed to create app data dir: {e}"))?;
        let path = app_data_dir.join("rendernorth.db");

        let conn = Connection::open(&path).map_err(|e| format!("failed to open db: {e}"))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| format!("failed to set WAL: {e}"))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| format!("failed to enable foreign keys: {e}"))?;

        migrate(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
            path,
        })
    }

    pub fn schema_version(&self) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("failed to read schema version: {e}"))
    }
}

fn migrate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version    INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
         );",
    )
    .map_err(|e| format!("failed to create schema_migrations: {e}"))?;

    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("failed to read migration state: {e}"))?;

    for (version, sql) in MIGRATIONS {
        if *version <= current {
            continue;
        }
        conn.execute_batch("BEGIN;")
            .map_err(|e| format!("migration {version}: begin failed: {e}"))?;
        if let Err(e) = conn.execute_batch(sql) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(format!("migration {version} failed: {e}"));
        }
        if let Err(e) = conn.execute(
            "INSERT INTO schema_migrations (version) VALUES (?1)",
            [version],
        ) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(format!("migration {version}: record failed: {e}"));
        }
        conn.execute_batch("COMMIT;")
            .map_err(|e| format!("migration {version}: commit failed: {e}"))?;
    }

    Ok(())
}
