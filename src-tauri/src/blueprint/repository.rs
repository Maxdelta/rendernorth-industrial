//! `BlueprintRepository` — the only place that writes SQL against the
//! blueprint tables. Mirrors `inventory::repository::InventoryRepository`,
//! `operation::repository::OperationRepository`, and
//! `reservation::repository::ReservationRepository`.

use super::models::{
    BlueprintDetail, BlueprintReadiness, BlueprintRecord, BlueprintRequirement, BlueprintSummary,
    MissingBlueprintReport,
};
use rusqlite::Connection;

pub struct BlueprintRepository<'a> {
    conn: &'a Connection,
}

impl<'a> BlueprintRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    const RECORD_SELECT: &'static str = "
        SELECT b.blueprint_id, b.type_name, b.is_copy, b.me_level, b.te_level,
               b.runs_remaining, COALESCE(ch.name, 'Unassigned') AS owner_name,
               COALESCE(loc.name, 'Unknown location') AS location_name, b.status,
               op.goal AS linked_operation
        FROM blueprints b
        LEFT JOIN characters ch ON ch.character_id = b.character_id
        LEFT JOIN inventory_locations loc ON loc.location_id = b.location_id
        LEFT JOIN (
            SELECT r.type_name, MAX(o.goal) AS goal
            FROM operation_blueprint_requirements r
            JOIN operations o ON o.operation_id = r.operation_id
            GROUP BY r.type_name
        ) op ON op.type_name = b.type_name
    ";

    fn map_record(row: &rusqlite::Row) -> rusqlite::Result<BlueprintRecord> {
        let is_copy: i64 = row.get(2)?;
        Ok(BlueprintRecord {
            blueprint_id: row.get(0)?,
            type_name: row.get(1)?,
            is_copy: is_copy != 0,
            me_level: row.get(3)?,
            te_level: row.get(4)?,
            runs_remaining: row.get(5)?,
            owner_name: row.get(6)?,
            location_name: row.get(7)?,
            status: row.get(8)?,
            linked_operation: row.get(9)?,
        })
    }

    pub fn list(&self) -> Result<Vec<BlueprintRecord>, String> {
        let sql = format!("{} ORDER BY b.blueprint_id", Self::RECORD_SELECT);
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], Self::map_record)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_detail(&self, blueprint_id: i64) -> Result<BlueprintDetail, String> {
        let sql = format!("{} WHERE b.blueprint_id = ?1", Self::RECORD_SELECT);
        let record = self
            .conn
            .query_row(&sql, [blueprint_id], Self::map_record)
            .map_err(|e| format!("blueprint {blueprint_id} not found: {e}"))?;

        let mut required_by = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT DISTINCT o.goal
                     FROM operation_blueprint_requirements r
                     JOIN operations o ON o.operation_id = r.operation_id
                     WHERE r.type_name = ?1",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([&record.type_name], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            for row in rows {
                required_by.push(row.map_err(|e| e.to_string())?);
            }
        }

        Ok(BlueprintDetail { record, required_by })
    }

    pub fn summary(&self) -> Result<BlueprintSummary, String> {
        let (total_blueprints, bpo_count, bpc_count, research_complete, copies): (
            i64,
            i64,
            i64,
            i64,
            i64,
        ) = self
            .conn
            .query_row(
                "SELECT
                    COUNT(*),
                    SUM(CASE WHEN is_copy = 0 THEN 1 ELSE 0 END),
                    SUM(CASE WHEN is_copy = 1 THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status != 'researching' THEN 1 ELSE 0 END),
                    COALESCE(SUM(CASE WHEN is_copy = 1 THEN runs_remaining ELSE 0 END), 0)
                 FROM blueprints",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                        row.get(4)?,
                    ))
                },
            )
            .map_err(|e| format!("blueprint summary query failed: {e}"))?;

        let missing_for_operations: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT r.type_name)
                 FROM operation_blueprint_requirements r
                 WHERE NOT EXISTS (SELECT 1 FROM blueprints b WHERE b.type_name = r.type_name)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("missing-for-operations query failed: {e}"))?;

        Ok(BlueprintSummary {
            total_blueprints,
            bpo_count,
            bpc_count,
            research_complete,
            copies,
            missing_for_operations,
        })
    }

    /// Global report: every required blueprint type with no owned
    /// blueprint anywhere, and which operations need it. Detect only —
    /// nothing here acquires or researches a blueprint.
    pub fn missing_report(&self) -> Result<Vec<MissingBlueprintReport>, String> {
        let mut report = Vec::new();
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT r.type_name
                 FROM operation_blueprint_requirements r
                 WHERE NOT EXISTS (SELECT 1 FROM blueprints b WHERE b.type_name = r.type_name)",
            )
            .map_err(|e| e.to_string())?;
        let type_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        for type_name in type_names {
            let mut op_stmt = self
                .conn
                .prepare(
                    "SELECT o.goal FROM operation_blueprint_requirements r
                     JOIN operations o ON o.operation_id = r.operation_id
                     WHERE r.type_name = ?1",
                )
                .map_err(|e| e.to_string())?;
            let goals = op_stmt
                .query_map([&type_name], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            report.push(MissingBlueprintReport {
                type_name,
                required_by_operations: goals,
            });
        }

        Ok(report)
    }

    /// Blueprint readiness for one operation: every requirement, whether
    /// it's owned, and whether an owned copy is currently researching or
    /// copying (a warning, distinct from missing). Always computed live —
    /// same precedent as Operation Engine's `is_blocked`.
    pub fn readiness_for_operation(&self, operation_id: i64) -> Result<BlueprintReadiness, String> {
        let operation_goal: String = self
            .conn
            .query_row(
                "SELECT goal FROM operations WHERE operation_id = ?1",
                [operation_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("operation {operation_id} not found: {e}"))?;

        let mut required = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT r.type_name, r.reason,
                            EXISTS(SELECT 1 FROM blueprints b WHERE b.type_name = r.type_name) AS is_owned,
                            (SELECT b.status FROM blueprints b
                             WHERE b.type_name = r.type_name AND b.status IN ('researching', 'copying')
                             LIMIT 1) AS warning_status
                     FROM operation_blueprint_requirements r
                     WHERE r.operation_id = ?1
                     ORDER BY r.id",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([operation_id], |row| {
                    let is_owned: i64 = row.get(2)?;
                    Ok(BlueprintRequirement {
                        type_name: row.get(0)?,
                        reason: row.get(1)?,
                        is_owned: is_owned != 0,
                        warning_status: row.get(3)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                required.push(row.map_err(|e| e.to_string())?);
            }
        }

        let owned_count = required.iter().filter(|r| r.is_owned).count() as i64;
        let missing_count = required.iter().filter(|r| !r.is_owned).count() as i64;
        let warning_count = required.iter().filter(|r| r.warning_status.is_some()).count() as i64;

        Ok(BlueprintReadiness {
            operation_id,
            operation_goal,
            required,
            owned_count,
            missing_count,
            warning_count,
        })
    }

    /// Readiness for every operation that has at least one blueprint
    /// requirement — backs Mission Control's Blueprint Readiness panel.
    pub fn readiness_all(&self) -> Result<Vec<BlueprintReadiness>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT operation_id FROM operation_blueprint_requirements ORDER BY operation_id")
            .map_err(|e| e.to_string())?;
        let operation_ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut out = Vec::new();
        for id in operation_ids {
            out.push(self.readiness_for_operation(id)?);
        }
        Ok(out)
    }
}
