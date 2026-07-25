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
    (
        3,
        include_str!("../migrations/0003_inventory_foundation.sql"),
    ),
    (
        4,
        include_str!("../migrations/0004_operation_foundation.sql"),
    ),
    (
        5,
        include_str!("../migrations/0005_reservation_foundation.sql"),
    ),
    (
        6,
        include_str!("../migrations/0006_blueprint_foundation.sql"),
    ),
    (
        7,
        include_str!("../migrations/0007_production_requirement_foundation.sql"),
    ),
    (
        8,
        include_str!("../migrations/0008_real_production_planner.sql"),
    ),
    (9, include_str!("../migrations/0009_inventory_scope.sql")),
    (
        10,
        include_str!("../migrations/0010_demo_category_correction.sql"),
    ),
    (
        11,
        include_str!("../migrations/0011_esi_character_auth.sql"),
    ),
    (
        12,
        include_str!("../migrations/0012_character_asset_sync.sql"),
    ),
    (
        13,
        include_str!("../migrations/0013_character_blueprint_sync.sql"),
    ),
    (
        14,
        include_str!("../migrations/0014_location_resolution.sql"),
    ),
    (15, include_str!("../migrations/0015_market_valuation.sql")),
    (16, include_str!("../migrations/0016_type_volume.sql")),
    (
        17,
        include_str!("../migrations/0017_operation_procurement.sql"),
    ),
    (18, include_str!("../migrations/0018_quartermaster.sql")),
    (
        19,
        include_str!("../migrations/0019_source_aware_operation_blueprints.sql"),
    ),
    (
        20,
        include_str!("../migrations/0020_corporation_asset_sync.sql"),
    ),
    (21, include_str!("../migrations/0021_personal_commerce.sql")),
    (
        22,
        include_str!("../migrations/0022_commerce_activity_center.sql"),
    ),
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
        let conn = self
            .conn
            .lock()
            .map_err(|_| "db lock poisoned".to_string())?;
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

    // Bootstrap-level bookkeeping table, created the same way
    // schema_migrations is — directly in Rust, not a numbered `.sql`
    // migration file, so it never touches the migration registration
    // list (db.rs / production::repository / staticdata::jsonl all stay
    // untouched by this). Its only job: durably remember, across a crash
    // at any point, whether this exact database file was fresh the first
    // time this code ever ran against it. See docs/architecture/
    // DATA_OWNERSHIP.md for the full lifecycle — this is NOT a runtime
    // feature flag, NOT a replacement for schema_migrations, and nothing
    // outside this function ever reads it.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS install_state (
             key   TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );",
    )
    .map_err(|e| format!("failed to create install_state: {e}"))?;

    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("failed to read migration state: {e}"))?;

    // Written at most once, ever, per database file: only when this run
    // finds zero prior migration history. INSERT OR IGNORE makes this
    // safe to attempt on every startup with no effect after the first —
    // an upgraded install (current > 0 the first time this code runs
    // against it) never gets this row, ever, which is what guarantees
    // its demo rows are never touched by the cleanup below.
    if current == 0 {
        conn.execute(
            "INSERT OR IGNORE INTO install_state (key, value) VALUES ('demo_cleanup_pending', '1')",
            [],
        )
        .map_err(|e| format!("failed to record fresh-install marker: {e}"))?;
    }

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

    // Checked on EVERY call to migrate(), not just when current==0 above
    // — this is what makes it crash-safe regardless of when a crash
    // happens. If the marker is present (written durably, possibly on a
    // previous, interrupted run), cleanup runs; if it's absent (never
    // written, or already completed), this is a no-op.
    let cleanup_pending: bool = conn
        .query_row(
            "SELECT 1 FROM install_state WHERE key = 'demo_cleanup_pending'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|_| true)
        .unwrap_or(false);

    if cleanup_pending {
        run_demo_seed_cleanup(conn)?;
        conn.execute(
            "DELETE FROM install_state WHERE key = 'demo_cleanup_pending'",
            [],
        )
        .map_err(|e| format!("failed to clear fresh-install marker: {e}"))?;
    }

    Ok(())
}

/// Removes seeded demo/example rows from a fresh install only — never
/// called directly except from `migrate()`'s marker-gated dispatch above,
/// which guarantees this only ever runs against a database that had zero
/// migration history before this process started. Every statement here
/// is idempotent (deleting already-gone rows is a harmless no-op), so a
/// crash partway through and a retry on the next startup is always safe.
///
/// Deliberately excluded, permanently: `inventory_categories`,
/// `inventory_states` — these are product vocabulary (label taxonomy),
/// not seeded example instance data, and remain in every install,
/// fresh or upgraded, forever. See docs/architecture/DATA_OWNERSHIP.md.
///
/// Deliberately excluded: `build_projects` and its schema (the Build
/// Targets concept) — the concept and its backing tables are retained
/// pending a product decision on its future role; only demo *rows*
/// within it are removed here, via the same cascade the schema already
/// defines (`build_requirement_groups`/`missing_materials`/
/// `recommendations` all have `ON DELETE CASCADE` from `build_projects`).
fn run_demo_seed_cleanup(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "DELETE FROM production_requirement_sources
            WHERE requirement_id IN (SELECT id FROM production_requirements);
         DELETE FROM production_requirements;
         DELETE FROM production_requirement_groups;

         DELETE FROM operation_blueprint_requirements
            WHERE operation_id IN (SELECT operation_id FROM operations WHERE is_demo = 1);

         DELETE FROM reservation_events
            WHERE from_operation_id IN (SELECT operation_id FROM operations WHERE is_demo = 1)
               OR to_operation_id IN (SELECT operation_id FROM operations WHERE is_demo = 1);
         DELETE FROM reservation_conflicts
            WHERE item_id IN (SELECT item_id FROM inventory_items WHERE source = 'demo');
         DELETE FROM inventory_reservations
            WHERE (operation_id IS NOT NULL AND operation_id IN (SELECT operation_id FROM operations WHERE is_demo = 1))
               OR item_id IN (SELECT item_id FROM inventory_items WHERE source = 'demo');

         DELETE FROM operation_timeline
            WHERE operation_id IN (SELECT operation_id FROM operations WHERE is_demo = 1);
         DELETE FROM operation_dependencies
            WHERE operation_id IN (SELECT operation_id FROM operations WHERE is_demo = 1)
               OR depends_on_operation_id IN (SELECT operation_id FROM operations WHERE is_demo = 1);

         DELETE FROM inventory_allocations
            WHERE item_id IN (SELECT item_id FROM inventory_items WHERE source = 'demo');

         DELETE FROM operations WHERE is_demo = 1;
         DELETE FROM blueprints WHERE is_demo = 1;
         DELETE FROM inventory_items WHERE source = 'demo';
         DELETE FROM inventory_locations
            WHERE location_id NOT IN (SELECT location_id FROM inventory_items WHERE location_id IS NOT NULL);
         DELETE FROM characters WHERE is_demo = 1;

         -- Cascades automatically to build_requirement_groups,
         -- missing_materials, and recommendations (ON DELETE CASCADE).
         -- The build_projects table itself, and the Build Targets
         -- concept it backs, are NOT dropped from the schema.
         DELETE FROM build_projects;
         DELETE FROM factory_snapshot;",
    )
    .map_err(|e| format!("demo seed cleanup failed: {e}"))?;

    Ok(())
}

// ============================================================
// Tests for the fresh-install detection + demo cleanup mechanism.
// See docs/architecture/DATA_OWNERSHIP.md for the full lifecycle this
// verifies. Written and reasoned through carefully, and the underlying
// cleanup SQL was independently verified against real SQLite with
// foreign keys enforced before being written into `run_demo_seed_cleanup`
// — but NOT run through `cargo test` in the authoring environment (no
// cargo available there). Run `cargo test` locally to confirm.
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    fn demo_row_counts(conn: &Connection) -> Vec<(&'static str, i64)> {
        let tables = [
            "operations",
            "operation_timeline",
            "operation_dependencies",
            "operation_blueprint_requirements",
            "production_requirements",
            "production_requirement_groups",
            "production_requirement_sources",
            "inventory_reservations",
            "reservation_events",
            "reservation_conflicts",
            "inventory_items",
            "inventory_allocations",
            "inventory_locations",
            "characters",
            "blueprints",
            "build_projects",
            "build_requirement_groups",
            "missing_materials",
            "recommendations",
            "factory_snapshot",
        ];
        tables
            .iter()
            .map(|t| {
                let n: i64 = conn
                    .query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
                    .unwrap();
                (*t, n)
            })
            .collect()
    }

    #[test]
    fn fresh_install_ends_with_no_demo_data_but_keeps_vocabulary() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();

        migrate(&conn).expect("migrate should succeed on a fresh database");

        for (table, count) in demo_row_counts(&conn) {
            assert_eq!(
                count, 0,
                "table '{table}' must be empty after fresh-install cleanup, found {count} rows"
            );
        }

        // Vocabulary/taxonomy tables are not demo data and must survive.
        let categories: i64 = conn
            .query_row("SELECT COUNT(*) FROM inventory_categories", [], |r| {
                r.get(0)
            })
            .unwrap();
        let states: i64 = conn
            .query_row("SELECT COUNT(*) FROM inventory_states", [], |r| r.get(0))
            .unwrap();
        assert!(
            categories > 0,
            "inventory_categories is product vocabulary, must survive fresh-install cleanup"
        );
        assert!(
            states > 0,
            "inventory_states is product vocabulary, must survive fresh-install cleanup"
        );

        // The marker must have been created and then cleared — no
        // permanent runtime dependency left behind.
        let marker: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM install_state WHERE key = 'demo_cleanup_pending'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(marker, 0, "the fresh-install marker must be cleared once cleanup succeeds, not left pending forever");
    }

    #[test]
    fn interrupted_cleanup_resumes_safely_on_next_call() {
        // Simulates a crash mid-cleanup: migrate() runs once (marker
        // written, migrations applied, cleanup started), then we manually
        // put the marker back as if cleanup never finished, and confirm
        // calling migrate() again completes it — proving a crash between
        // "marker written" and "marker cleared" always self-heals on the
        // next startup, never leaves a permanently half-cleaned database.
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();

        migrate(&conn).expect("first migrate should succeed");
        for (_, count) in demo_row_counts(&conn) {
            assert_eq!(count, 0);
        }

        // Re-seed some demo rows directly and re-insert the marker, as if
        // a crash happened after migrations committed but partway through
        // cleanup (some tables cleaned, some not — here, simulated by
        // reintroducing rows into just one demo table).
        conn.execute(
            "INSERT INTO operations (goal, priority, status, progress, notes, deadline, is_demo)
             VALUES ('Simulated Leftover Demo Op', 1, 'planned', 0, '', NULL, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO install_state (key, value) VALUES ('demo_cleanup_pending', '1')",
            [],
        )
        .unwrap();

        migrate(&conn).expect("second migrate (resume) should succeed");

        let demo_ops: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM operations WHERE is_demo = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            demo_ops, 0,
            "resumed cleanup must finish removing demo rows left over from the simulated crash"
        );
        let marker: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM install_state WHERE key = 'demo_cleanup_pending'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            marker, 0,
            "marker must be cleared again after the resumed cleanup succeeds"
        );
    }

    #[test]
    fn upgraded_database_keeps_demo_rows_untouched() {
        // Simulates an existing install: apply every migration directly
        // (bypassing migrate()'s marker-write logic entirely, exactly as
        // if this database already had real history before this code
        // ever existed), THEN call migrate(). The marker must never be
        // written retroactively, so cleanup must never run against it.
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                 version    INTEGER PRIMARY KEY,
                 applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
             );",
        )
        .unwrap();
        for (version, sql) in MIGRATIONS {
            conn.execute_batch(sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                [version],
            )
            .unwrap();
        }

        let demo_ops_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM operations WHERE is_demo = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            demo_ops_before > 0,
            "fixture must have pre-existing demo rows for this test to be meaningful"
        );

        // Now call migrate() for the "first time" this code sees this
        // already-populated database — current will be > 0, so the
        // marker must never be written.
        migrate(&conn).expect("migrate should succeed on an already-migrated database");

        let demo_ops_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM operations WHERE is_demo = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            demo_ops_before, demo_ops_after,
            "an upgraded install's demo rows must never be touched"
        );

        let marker: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM install_state WHERE key = 'demo_cleanup_pending'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            marker, 0,
            "the marker must never be written for a database that already had migration history"
        );
    }

    #[test]
    fn migration_0019_preserves_existing_manual_blueprint_selection() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (_, sql) in MIGRATIONS.iter().filter(|(version, _)| *version <= 18) {
            conn.execute_batch(sql).unwrap();
        }
        conn.execute(
            "INSERT INTO eve_categories(category_id,name) VALUES(900,'Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO eve_groups(group_id,category_id,name) VALUES(900,900,'Test')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO eve_types(type_id,name,group_id,is_manufacturable) VALUES(900,'Test Product',900,1)",[]).unwrap();
        conn.execute("INSERT INTO operations(goal,priority,status,progress,notes,is_demo) VALUES('Legacy',1,'planned',0,'',0)",[]).unwrap();
        let operation_id = conn.last_insert_rowid();
        conn.execute("INSERT INTO blueprints(blueprint_id,type_name,is_copy,me_level,te_level,is_demo) VALUES(900,'Test Product',0,10,20,0)",[]).unwrap();
        conn.execute("INSERT INTO operation_build_targets(operation_id,type_id,quantity_requested,blueprint_mode,owned_blueprint_id) VALUES(?1,900,1,'owned',900)",[operation_id]).unwrap();
        conn.execute_batch(include_str!(
            "../migrations/0019_source_aware_operation_blueprints.sql"
        ))
        .unwrap();
        let values:(String,i64)=conn.query_row(
            "SELECT selected_blueprint_source,manual_blueprint_id FROM operation_build_targets WHERE operation_id=?1",
            [operation_id],|r|Ok((r.get(0)?,r.get(1)?))
        ).unwrap();
        assert_eq!(values, ("manual".into(), 900));
    }

    #[test]
    fn migration_0020_is_additive_and_preserves_existing_inventory() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (_, sql) in MIGRATIONS.iter().filter(|(version, _)| *version <= 19) {
            conn.execute_batch(sql).unwrap();
        }
        conn.execute(
            "INSERT INTO characters(character_id,name,is_demo,scopes_granted,enabled,authorization_status) VALUES(42,'Pilot',0,'',1,'authorized')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO eve_categories(category_id,name) VALUES(99991,'Migration Test')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO eve_groups(group_id,category_id,name) VALUES(99991,99991,'Migration Test')",[]).unwrap();
        conn.execute("INSERT INTO eve_types(type_id,name,group_id,is_manufacturable) VALUES(99991,'Migration Test Type',99991,0)",[]).unwrap();
        conn.execute(
            "INSERT INTO character_assets(character_id,item_id,type_id,quantity,location_id,location_type,location_flag,is_singleton,synced_at) VALUES(42,1001,99991,50,60003760,'station','Hangar',0,'2026-07-17T12:00:00Z')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO manual_inventory_entries(type_id,quantity,location_name) VALUES(99991,25,'Jita')",
            [],
        ).unwrap();

        conn.execute_batch(include_str!(
            "../migrations/0020_corporation_asset_sync.sql"
        ))
        .unwrap();

        let personal: i64 = conn
            .query_row(
                "SELECT quantity FROM character_assets WHERE character_id=42 AND item_id=1001",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let manual: i64 = conn
            .query_row(
                "SELECT quantity FROM manual_inventory_entries WHERE type_id=99991",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((personal, manual), (50, 25));
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM corporation_assets", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn migration_0021_is_additive_and_preserves_existing_personal_data() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (_, sql) in MIGRATIONS.iter().filter(|(version, _)| *version <= 20) {
            conn.execute_batch(sql).unwrap();
        }
        conn.execute(
            "INSERT INTO characters(character_id,name,is_demo,scopes_granted,enabled,authorization_status) VALUES(42,'Pilot',0,'esi-assets.read_assets.v1',1,'authorized')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO eve_categories(category_id,name) VALUES(99992,'Commerce Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO eve_groups(group_id,category_id,name) VALUES(99992,99992,'Commerce Test')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO eve_types(type_id,name,group_id,is_manufacturable) VALUES(99992,'Commerce Test Type',99992,0)",[]).unwrap();
        conn.execute(
            "INSERT INTO character_assets(character_id,item_id,type_id,quantity,location_id,location_type,location_flag,is_singleton,synced_at) VALUES(42,2001,99992,7,60003760,'station','Hangar',0,'2026-07-18T12:00:00Z')",
            [],
        ).unwrap();

        conn.execute_batch(include_str!("../migrations/0021_personal_commerce.sql"))
            .unwrap();

        let quantity: i64 = conn
            .query_row(
                "SELECT quantity FROM character_assets WHERE character_id=42 AND item_id=2001",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(quantity, 7);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM character_market_orders", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM character_contracts", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn migration_0022_is_additive_and_preserves_commerce_snapshots() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (_, sql) in MIGRATIONS.iter().filter(|(version, _)| *version <= 21) {
            conn.execute_batch(sql).unwrap();
        }
        conn.execute(
            "INSERT INTO characters(character_id,name,is_demo,scopes_granted,enabled,authorization_status) VALUES(42,'Pilot',0,'esi-markets.read_character_orders.v1',1,'authorized')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO character_market_orders(character_id,order_id,type_id,is_buy_order,is_corporation,location_id,region_id,price_isk,volume_total,volume_remain,issued_at,duration_days,order_range,source,synced_at) VALUES(42,9001,34,0,0,60003760,10000002,'10.00',100,25,'2026-07-01T00:00:00Z',30,'station','ESI Character Market Orders','2026-07-25T00:00:00Z')",
            [],
        ).unwrap();
        conn.execute_batch(include_str!("../migrations/0022_commerce_activity_center.sql"))
            .unwrap();
        assert_eq!(
            conn.query_row("SELECT volume_remain FROM character_market_orders WHERE order_id=9001", [], |row| row.get::<_, i64>(0)).unwrap(),
            25
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM character_market_order_history", [], |row| row.get::<_, i64>(0)).unwrap(),
            0
        );
    }
}
