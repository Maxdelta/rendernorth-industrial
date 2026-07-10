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

    pub fn list_summaries(&self) -> Result<Vec<OperationSummary>, String> {
        let sql = Self::summary_sql("", "ORDER BY o.priority ASC, o.operation_id ASC");
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], Self::map_summary)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn priority_queue(&self) -> Result<Vec<OperationSummary>, String> {
        let sql = Self::summary_sql(
            "WHERE o.status != 'completed'",
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
            &format!("WHERE {IS_BLOCKED_EXPR}"),
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
            "WHERE o.deadline IS NOT NULL AND o.status != 'completed'",
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
                     FROM operations o"
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
                     FROM operations o WHERE o.operation_id = ?1"
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
                             owned_blueprint_id, assumed_is_bpc, assumed_me, assumed_te, assumed_runs)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
}
