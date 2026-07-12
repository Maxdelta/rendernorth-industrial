//! `OperationRepository` — the only place that writes SQL against the
//! operation tables. Mirrors `inventory::repository::InventoryRepository`.

use super::models::{
    OperationDependency, OperationDetail, OperationHealth, OperationSummary,
    OperationTimelineEntry,
};
use rusqlite::Connection;

/// The `is_blocked` derivation shared by every query in this file: an
/// operation is blocked if its own status says so, or if anything it
/// depends on isn't complete yet. Never a hand-set literal per row.
const IS_BLOCKED_EXPR: &str = "(
    o.status = 'blocked' OR EXISTS (
        SELECT 1 FROM operation_dependencies d
        JOIN operations dep ON dep.operation_id = d.depends_on_operation_id
        WHERE d.operation_id = o.operation_id AND dep.status != 'completed'
    )
)";

pub struct OperationRepository<'a> {
    conn: &'a Connection,
}

impl<'a> OperationRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn summary_sql(where_clause: &str, order_by: &str) -> String {
        format!(
            "SELECT o.operation_id, o.goal, o.target_type_name, o.priority, o.status,
                    o.progress, o.deadline, {IS_BLOCKED_EXPR} AS is_blocked, o.is_demo
             FROM operations o
             {where_clause}
             {order_by}"
        )
    }

    fn map_summary(row: &rusqlite::Row) -> rusqlite::Result<OperationSummary> {
        let is_demo: i64 = row.get(8)?;
        Ok(OperationSummary {
            operation_id: row.get(0)?,
            goal: row.get(1)?,
            target_type_name: row.get(2)?,
            priority: row.get(3)?,
            status: row.get(4)?,
            progress: row.get(5)?,
            deadline: row.get(6)?,
            is_blocked: row.get(7)?,
            is_demo: is_demo != 0,
        })
    }

    /// Normal runtime: real operations only. Demo rows (is_demo = 1),
    /// whether from an upgraded install's leftover seed or otherwise,
    /// never appear in normal application queries. See
    /// docs/architecture/DATA_OWNERSHIP.md.
    pub fn list_summaries(&self) -> Result<Vec<OperationSummary>, String> {
        let sql = Self::summary_sql("WHERE o.is_demo = 0", "ORDER BY o.priority ASC, o.operation_id ASC");
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], Self::map_summary)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn priority_queue(&self) -> Result<Vec<OperationSummary>, String> {
        let sql = Self::summary_sql(
            "WHERE o.status != 'completed' AND o.is_demo = 0",
            "ORDER BY o.priority ASC, o.operation_id ASC",
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], Self::map_summary)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn blocked(&self) -> Result<Vec<OperationSummary>, String> {
        let sql = Self::summary_sql(
            &format!("WHERE {IS_BLOCKED_EXPR} AND o.is_demo = 0"),
            "ORDER BY o.priority ASC, o.operation_id ASC",
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], Self::map_summary)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn upcoming_completions(&self, limit: i64) -> Result<Vec<OperationSummary>, String> {
        let sql = Self::summary_sql(
            "WHERE o.deadline IS NOT NULL AND o.status != 'completed' AND o.is_demo = 0",
            "ORDER BY o.deadline ASC LIMIT ?1",
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([limit], Self::map_summary)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn health(&self) -> Result<OperationHealth, String> {
        let (total, active, blocked): (i64, i64, i64) = self
            .conn
            .query_row(
                &format!(
                    "SELECT COUNT(*),
                            SUM(CASE WHEN o.status = 'active' THEN 1 ELSE 0 END),
                            SUM(CASE WHEN {IS_BLOCKED_EXPR} THEN 1 ELSE 0 END)
                     FROM operations o
                     WHERE o.is_demo = 0"
                ),
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    ))
                },
            )
            .map_err(|e| format!("operation health query failed: {e}"))?;

        let healthy_fraction = if total > 0 {
            (total - blocked) as f64 / total as f64
        } else {
            0.0
        };

        Ok(OperationHealth {
            total_operations: total,
            active_operations: active,
            blocked_operations: blocked,
            healthy_fraction,
        })
    }

    /// Normal runtime: refuses a demo operation the same way `list_summaries`
    /// et al. never surface one — a stale or hand-typed URL pointing at a
    /// demo operation_id must not become a backdoor into hidden data.
    pub fn get_detail(&self, operation_id: i64) -> Result<OperationDetail, String> {
        let (goal, target_type_name, priority, status, progress, notes, deadline, is_blocked, is_demo): (
            String,
            Option<String>,
            i64,
            String,
            f64,
            String,
            Option<String>,
            bool,
            i64,
        ) = self
            .conn
            .query_row(
                &format!(
                    "SELECT goal, target_type_name, priority, status, progress, notes,
                            deadline, {IS_BLOCKED_EXPR} AS is_blocked, o.is_demo
                     FROM operations o WHERE o.operation_id = ?1 AND o.is_demo = 0"
                ),
                [operation_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .map_err(|e| format!("operation {operation_id} not found: {e}"))?;

        let mut dependencies = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT d.depends_on_operation_id, dep.goal, dep.status, d.reason
                     FROM operation_dependencies d
                     JOIN operations dep ON dep.operation_id = d.depends_on_operation_id
                     WHERE d.operation_id = ?1",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([operation_id], |row| {
                    Ok(OperationDependency {
                        depends_on_operation_id: row.get(0)?,
                        depends_on_goal: row.get(1)?,
                        depends_on_status: row.get(2)?,
                        reason: row.get(3)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                dependencies.push(row.map_err(|e| e.to_string())?);
            }
        }

        let mut timeline = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, label, status, sort_order, target_date
                     FROM operation_timeline WHERE operation_id = ?1 ORDER BY sort_order",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([operation_id], |row| {
                    Ok(OperationTimelineEntry {
                        id: row.get(0)?,
                        label: row.get(1)?,
                        status: row.get(2)?,
                        sort_order: row.get(3)?,
                        target_date: row.get(4)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                timeline.push(row.map_err(|e| e.to_string())?);
            }
        }

        Ok(OperationDetail {
            operation_id,
            goal,
            target_type_name,
            priority,
            status,
            progress,
            notes,
            deadline,
            is_blocked,
            is_demo: is_demo != 0,
            dependencies,
            timeline,
        })
    }

    /// Real operation creation (Sprint 008). Inserts a genuine, non-demo
    /// operation and, when a build target was selected, its
    /// `operation_build_targets` row — in one transaction, so a partially
    /// created operation can never exist. `blueprint_source` defaults to
    /// "assumed" with ME/TE 0 when not otherwise specified.
    pub fn create(&self, input: &super::models::NewOperationInput) -> Result<i64, String> {
        self.conn
            .execute_batch("BEGIN;")
            .map_err(|e| format!("failed to begin operation-create transaction: {e}"))?;

        let result: Result<i64, String> = (|| {
            self.conn
                .execute(
                    "INSERT INTO operations (goal, priority, status, progress, notes, deadline, is_demo)
                     VALUES (?1, ?2, 'planned', 0, ?3, ?4, 0)",
                    (
                        &input.goal,
                        input.priority,
                        input.notes.as_deref().unwrap_or(""),
                        &input.deadline,
                    ),
                )
                .map_err(|e| format!("failed to insert operation: {e}"))?;
            let operation_id = self.conn.last_insert_rowid();

            if let Some(type_id) = input.type_id {
                let blueprint_mode = input.blueprint_mode.as_deref().unwrap_or("assumed");
                self.conn
                    .execute(
                        "INSERT INTO operation_build_targets
                            (operation_id, type_id, quantity_requested, blueprint_mode,
                             owned_blueprint_id, assumed_is_bpc, assumed_me, assumed_te, assumed_runs, inventory_scope)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        (
                            operation_id,
                            type_id,
                            input.quantity_requested.unwrap_or(1),
                            blueprint_mode,
                            input.owned_blueprint_id,
                            input.assumed_is_bpc.unwrap_or(false) as i64,
                            input.assumed_me.unwrap_or(0),
                            input.assumed_te.unwrap_or(0),
                            input.assumed_runs,
                            input.inventory_scope.as_deref().unwrap_or("all_included_inventory"),
                        ),
                    )
                    .map_err(|e| format!("failed to insert build target: {e}"))?;
            }

            Ok(operation_id)
        })();

        match result {
            Ok(id) => {
                self.conn
                    .execute_batch("COMMIT;")
                    .map_err(|e| format!("failed to commit operation-create transaction: {e}"))?;
                Ok(id)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK;");
                Err(e)
            }
        }
    }

    /// Deletes a real operation and every row it owns, in one transaction.
    /// Demo operations are refused before any transaction even begins —
    /// not just filtered out of a WHERE clause at the end — so there is
    /// no window where a demo operation's dependent rows could be
    /// partially removed. Tables NOT touched by design: `eve_*`/
    /// `blueprint_products`/`blueprint_materials` (imported static data),
    /// `manual_inventory_entries`, `blueprints` (owned blueprints),
    /// `app_meta` (settings), and every other operation's own rows.
    pub fn delete(&self, operation_id: i64) -> Result<(), String> {
        let is_demo: Option<i64> = self
            .conn
            .query_row("SELECT is_demo FROM operations WHERE operation_id = ?1", [operation_id], |row| row.get(0))
            .ok();

        match is_demo {
            None => return Err(format!("operation {operation_id} not found")),
            Some(1) => return Err("demo operations cannot be deleted".into()),
            _ => {}
        }

        self.conn
            .execute_batch("BEGIN;")
            .map_err(|e| format!("failed to begin delete transaction: {e}"))?;

        let result: Result<(), String> = (|| {
            // Children before parent, in dependency order — none of these
            // tables have ON DELETE CASCADE, so order matters with foreign
            // keys enforced. operation_dependencies is bidirectional: a
            // row can reference this operation as either the dependent
            // side or the thing another operation depends on, so both
            // columns are checked.
            self.conn
                .execute("DELETE FROM operation_timeline WHERE operation_id = ?1", [operation_id])
                .map_err(|e| format!("failed to delete operation timeline: {e}"))?;
            self.conn
                .execute(
                    "DELETE FROM operation_dependencies WHERE operation_id = ?1 OR depends_on_operation_id = ?1",
                    [operation_id],
                )
                .map_err(|e| format!("failed to delete operation dependencies: {e}"))?;
            self.conn
                .execute("DELETE FROM inventory_reservations WHERE operation_id = ?1", [operation_id])
                .map_err(|e| format!("failed to delete reservations: {e}"))?;
            self.conn
                .execute(
                    "DELETE FROM reservation_events WHERE from_operation_id = ?1 OR to_operation_id = ?1",
                    [operation_id],
                )
                .map_err(|e| format!("failed to delete reservation events: {e}"))?;
            self.conn
                .execute("DELETE FROM operation_blueprint_requirements WHERE operation_id = ?1", [operation_id])
                .map_err(|e| format!("failed to delete blueprint requirements: {e}"))?;
            self.conn
                .execute("DELETE FROM production_requirement_groups WHERE operation_id = ?1", [operation_id])
                .map_err(|e| format!("failed to delete production requirement groups: {e}"))?;
            self.conn
                .execute("DELETE FROM production_requirements WHERE operation_id = ?1", [operation_id])
                .map_err(|e| format!("failed to delete production requirements: {e}"))?;
            self.conn
                .execute("DELETE FROM operation_build_targets WHERE operation_id = ?1", [operation_id])
                .map_err(|e| format!("failed to delete build target: {e}"))?;
            self.conn
                .execute("DELETE FROM requirement_calculation_snapshots WHERE operation_id = ?1", [operation_id])
                .map_err(|e| format!("failed to delete calculation snapshots: {e}"))?;

            // The is_demo = 0 here is a second, belt-and-suspenders guard
            // — the real guard already returned early above before this
            // transaction started. Affecting 0 rows at this point would
            // mean the operation vanished between the check and here,
            // which the "no such operation" case below catches.
            let changed = self
                .conn
                .execute("DELETE FROM operations WHERE operation_id = ?1 AND is_demo = 0", [operation_id])
                .map_err(|e| format!("failed to delete operation: {e}"))?;
            if changed == 0 {
                return Err(format!("operation {operation_id} was not deleted — it may no longer exist or is a demo operation"));
            }

            Ok(())
        })();

        match result {
            Ok(()) => {
                self.conn
                    .execute_batch("COMMIT;")
                    .map_err(|e| format!("failed to commit delete transaction: {e}"))?;
                Ok(())
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK;");
                Err(e)
            }
        }
    }
}

// ============================================================
// Tests. Written and reasoned through carefully, and the exact deletion
// order was independently verified against real SQLite with foreign keys
// enforced before being written into this file (see Sprint 009.1's audit
// notes) — but NOT run through `cargo test` in the authoring environment
// (no cargo available there). Run `cargo test` locally to confirm.
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    const MIGRATIONS: &[&str] = &[
        include_str!("../../migrations/0001_init.sql"),
        include_str!("../../migrations/0002_build_targets.sql"),
        include_str!("../../migrations/0003_inventory_foundation.sql"),
        include_str!("../../migrations/0004_operation_foundation.sql"),
        include_str!("../../migrations/0005_reservation_foundation.sql"),
        include_str!("../../migrations/0006_blueprint_foundation.sql"),
        include_str!("../../migrations/0007_production_requirement_foundation.sql"),
        include_str!("../../migrations/0008_real_production_planner.sql"),
        include_str!("../../migrations/0009_inventory_scope.sql"),
        include_str!("../../migrations/0010_demo_category_correction.sql"),
        include_str!("../../migrations/0011_esi_character_auth.sql"),
    ];

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for m in MIGRATIONS {
            conn.execute_batch(m).expect("apply migration");
        }
        conn
    }

    /// Seeds one real operation with rows in every operation-scoped table
    /// this sprint's deletion touches, plus a sibling real operation that
    /// depends on it (exercising operation_dependencies' bidirectional
    /// column) and a manual inventory entry + real static data that must
    /// both survive regardless of what happens to the operation.
    fn seed_real_operation_with_full_dependents(conn: &Connection) -> (i64, i64) {
        conn.execute_batch(
            "INSERT INTO eve_categories (category_id, name) VALUES (1, 'Component');
             INSERT INTO eve_groups (group_id, category_id, name) VALUES (1, 1, 'Sample');
             INSERT INTO eve_types (type_id, name, group_id, is_manufacturable) VALUES (100, 'Widget', 1, 1);",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO operations (goal, priority, status, progress, notes, deadline, is_demo)
             VALUES ('Real Op To Delete', 2, 'planned', 0, '', NULL, 0)",
            [],
        )
        .unwrap();
        let op_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO operations (goal, priority, status, progress, notes, deadline, is_demo)
             VALUES ('Other Real Op', 2, 'planned', 0, '', NULL, 0)",
            [],
        )
        .unwrap();
        let other_op_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO operation_build_targets
                (operation_id, type_id, quantity_requested, blueprint_mode, assumed_me, assumed_te, assumed_is_bpc)
             VALUES (?1, 100, 5, 'assumed', 0, 0, 0)",
            [op_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO operation_timeline (operation_id, label, status, sort_order) VALUES (?1, 'Started', 'done', 1)",
            [op_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO operation_dependencies (operation_id, depends_on_operation_id, reason)
             VALUES (?1, ?2, 'test dependency')",
            (op_id, other_op_id),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO operation_blueprint_requirements (operation_id, type_name, required_me, required_te, reason)
             VALUES (?1, 'Widget', 0, 0, 'test')",
            [op_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO production_requirement_groups (operation_id, category_key) VALUES (?1, 'minerals')",
            [op_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO production_requirements (operation_id, category_key, type_name, required_quantity)
             VALUES (?1, 'minerals', 'Tritanium', 1000)",
            [op_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO requirement_calculation_snapshots (operation_id, inputs_json, result_json) VALUES (?1, '{}', '{}')",
            [op_id],
        )
        .unwrap();

        conn.execute_batch(
            "INSERT INTO inventory_locations (location_id, name, kind) VALUES (999, 'Test Station', 'station');
             INSERT INTO characters (character_id, name, is_demo) VALUES (-99, 'Tester', 1);
             INSERT INTO inventory_categories (key, label, sort_order) VALUES ('mat', 'Material', 1);
             INSERT INTO inventory_items (type_name, category_key, quantity, location_id, character_id, unit_value, source)
                VALUES ('Tritanium', 'mat', 100, 999, -99, 1, 'demo');",
        )
        .unwrap();
        let item_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO inventory_reservations (item_id, operation_id, quantity, reason) VALUES (?1, ?2, 10, 'test')",
            (item_id, op_id),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO reservation_events (reservation_id, event_type, quantity, from_operation_id, to_operation_id, reason)
             VALUES (1, 'created', 10, NULL, ?1, 'test')",
            [op_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO manual_inventory_entries (type_id, quantity, location_name) VALUES (100, 42, 'Home')",
            [],
        )
        .unwrap();

        (op_id, other_op_id)
    }

    fn count(conn: &Connection, sql: &str, params: impl rusqlite::Params) -> i64 {
        conn.query_row(sql, params, |row| row.get(0)).unwrap()
    }

    #[test]
    fn deleting_a_real_operation_removes_every_scoped_dependent_row() {
        let conn = test_db();
        let (op_id, _other_op_id) = seed_real_operation_with_full_dependents(&conn);
        let repo = OperationRepository::new(&conn);

        repo.delete(op_id).expect("delete should succeed");

        assert_eq!(count(&conn, "SELECT COUNT(*) FROM operations WHERE operation_id = ?1", [op_id]), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM operation_build_targets WHERE operation_id = ?1", [op_id]), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM operation_timeline WHERE operation_id = ?1", [op_id]), 0);
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM operation_dependencies WHERE operation_id = ?1 OR depends_on_operation_id = ?1", [op_id]),
            0
        );
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM operation_blueprint_requirements WHERE operation_id = ?1", [op_id]), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM production_requirement_groups WHERE operation_id = ?1", [op_id]), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM production_requirements WHERE operation_id = ?1", [op_id]), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM requirement_calculation_snapshots WHERE operation_id = ?1", [op_id]), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM inventory_reservations WHERE operation_id = ?1", [op_id]), 0);
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM reservation_events WHERE from_operation_id = ?1 OR to_operation_id = ?1", [op_id]),
            0
        );
    }

    #[test]
    fn deleting_a_real_operation_preserves_manual_inventory_and_static_data() {
        let conn = test_db();
        let (op_id, _other_op_id) = seed_real_operation_with_full_dependents(&conn);
        let repo = OperationRepository::new(&conn);

        repo.delete(op_id).expect("delete should succeed");

        assert_eq!(count(&conn, "SELECT COUNT(*) FROM manual_inventory_entries", []), 1, "manual inventory must survive operation deletion");
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM eve_types", []), 1, "imported CCP static data must survive operation deletion");
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM inventory_items", []),
            48, // migration 0003's 47-row demo seed + this test's own 1 inserted row
            "demo inventory (unrelated to this deletion) must survive too"
        );
    }

    #[test]
    fn deleting_a_real_operation_preserves_other_operations_and_their_dependency_link() {
        let conn = test_db();
        let (op_id, other_op_id) = seed_real_operation_with_full_dependents(&conn);
        let repo = OperationRepository::new(&conn);

        repo.delete(op_id).expect("delete should succeed");

        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM operations WHERE operation_id = ?1", [other_op_id]),
            1,
            "the other real operation must survive"
        );
    }

    #[test]
    fn demo_operations_cannot_be_deleted() {
        // Explicit fixture, not the migration chain's baked-in demo seed
        // — Sprint 010 retired demo data from normal runtime, so tests
        // exercising demo-protection logic should not lean on incidental
        // migration content that a fresh install no longer even has.
        let conn = test_db();
        conn.execute(
            "INSERT INTO operations (goal, priority, status, progress, notes, deadline, is_demo)
             VALUES ('Fixture Demo Op', 2, 'planned', 0, '', NULL, 1)",
            [],
        )
        .unwrap();
        let demo_op_id = conn.last_insert_rowid();

        let repo = OperationRepository::new(&conn);
        let before = count(&conn, "SELECT COUNT(*) FROM operations WHERE is_demo = 1", []);

        let result = repo.delete(demo_op_id);
        assert!(result.is_err(), "deleting a demo operation must be refused");
        assert!(result.unwrap_err().contains("demo"), "the error must clearly say why");

        let after = count(&conn, "SELECT COUNT(*) FROM operations WHERE is_demo = 1", []);
        assert_eq!(before, after, "no demo operation or its data may be touched, even partially");
    }

    #[test]
    fn failed_dependent_deletion_rolls_back_the_whole_transaction() {
        let conn = test_db();
        let (op_id, _other_op_id) = seed_real_operation_with_full_dependents(&conn);

        // Force a genuine failure partway through the deletion sequence.
        // operation_timeline, operation_dependencies, inventory_reservations,
        // and reservation_events all come before operation_blueprint_
        // requirements in the deletion order and will have already been
        // deleted successfully within this same transaction by the time
        // this DROP makes that step fail outright. (production_requirements
        // was tried first but SQLite correctly refuses to drop it — 
        // production_requirement_sources.requirement_id references it, so
        // dropping it would orphan that table; operation_blueprint_
        // requirements has no such dependent and drops cleanly.) This
        // proves the transaction rolls back completely, not just the one
        // failing statement — the earlier, already-"successful" deletes
        // must be undone too.
        conn.execute_batch("DROP TABLE operation_blueprint_requirements;").unwrap();

        let repo = OperationRepository::new(&conn);
        let result = repo.delete(op_id);
        assert!(result.is_err(), "delete must fail when a dependent table is unavailable");

        // The operation itself must still exist — nothing committed.
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM operations WHERE operation_id = ?1", [op_id]),
            1,
            "operation must remain intact after a failed delete"
        );
        // The timeline row (deleted successfully before the forced
        // failure, within the same transaction) must have been rolled
        // back too — proving this is a full rollback, not a partial commit.
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM operation_timeline WHERE operation_id = ?1", [op_id]),
            1,
            "rows deleted earlier in the same failed transaction must be restored by the rollback"
        );
    }
}
