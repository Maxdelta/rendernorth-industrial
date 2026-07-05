//! `ReservationRepository` — the only place that writes SQL against the
//! reservation tables. Mirrors `inventory::repository::InventoryRepository`
//! and `operation::repository::OperationRepository`.

use super::models::{
    InventoryCommitment, OperationReservations, ReservationConflict, ReservationDetail,
    ReservationEvent, ReservationRecord, ReservationSummary,
};
use rusqlite::Connection;

pub struct ReservationRepository<'a> {
    conn: &'a Connection,
}

impl<'a> ReservationRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn map_record(row: &rusqlite::Row) -> rusqlite::Result<ReservationRecord> {
        let released_at: Option<String> = row.get(7)?;
        Ok(ReservationRecord {
            reservation_id: row.get(0)?,
            item_id: row.get(1)?,
            item_name: row.get(2)?,
            operation_id: row.get(3)?,
            operation_goal: row.get(4)?,
            quantity: row.get(5)?,
            reason: row.get(6)?,
            is_active: released_at.is_none(),
            released_at,
            created_at: row.get(8)?,
        })
    }

    const RECORD_SELECT: &'static str = "
        SELECT r.id, r.item_id, i.type_name, r.operation_id, o.goal,
               r.quantity, r.reason, r.released_at, r.created_at
        FROM inventory_reservations r
        JOIN inventory_items i ON i.item_id = r.item_id
        LEFT JOIN operations o ON o.operation_id = r.operation_id
    ";

    pub fn list_active(&self) -> Result<Vec<ReservationRecord>, String> {
        let sql = format!("{} WHERE r.released_at IS NULL ORDER BY r.id", Self::RECORD_SELECT);
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], Self::map_record)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_detail(&self, reservation_id: i64) -> Result<ReservationDetail, String> {
        let sql = format!("{} WHERE r.id = ?1", Self::RECORD_SELECT);
        let record = self
            .conn
            .query_row(&sql, [reservation_id], Self::map_record)
            .map_err(|e| format!("reservation {reservation_id} not found: {e}"))?;

        let mut history = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, reservation_id, event_type, quantity,
                            from_operation_id, to_operation_id, reason, created_at
                     FROM reservation_events WHERE reservation_id = ?1 ORDER BY created_at",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([reservation_id], |row| {
                    Ok(ReservationEvent {
                        id: row.get(0)?,
                        reservation_id: row.get(1)?,
                        event_type: row.get(2)?,
                        quantity: row.get(3)?,
                        from_operation_id: row.get(4)?,
                        to_operation_id: row.get(5)?,
                        reason: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                history.push(row.map_err(|e| e.to_string())?);
            }
        }

        Ok(ReservationDetail { record, history })
    }

    pub fn summary(&self) -> Result<ReservationSummary, String> {
        let (active_reservation_count, total_reserved_quantity, items_with_reservations): (
            i64,
            i64,
            i64,
        ) = self
            .conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(quantity), 0), COUNT(DISTINCT item_id)
                 FROM inventory_reservations WHERE released_at IS NULL",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| format!("reservation summary query failed: {e}"))?;

        let conflict_count = self.conflicts()?.len() as i64;

        Ok(ReservationSummary {
            active_reservation_count,
            total_reserved_quantity,
            items_with_reservations,
            conflict_count,
        })
    }

    /// Detect only — never resolves anything. Three independent checks,
    /// each computed fresh from current reservation + inventory state:
    /// multiple operations holding the same item, a reservation total that
    /// exceeds on-hand quantity, and a reservation against an item with
    /// zero on-hand stock.
    pub fn conflicts(&self) -> Result<Vec<ReservationConflict>, String> {
        let mut conflicts = Vec::new();

        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT r.item_id, i.type_name, COUNT(DISTINCT r.operation_id) AS op_count,
                            SUM(r.quantity) AS total_reserved, i.quantity AS on_hand
                     FROM inventory_reservations r
                     JOIN inventory_items i ON i.item_id = r.item_id
                     WHERE r.released_at IS NULL
                     GROUP BY r.item_id
                     HAVING op_count > 1",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    let item_id: i64 = row.get(0)?;
                    let item_name: String = row.get(1)?;
                    let op_count: i64 = row.get(2)?;
                    let total_reserved: i64 = row.get(3)?;
                    let on_hand: i64 = row.get(4)?;
                    Ok(ReservationConflict {
                        item_id,
                        item_name: item_name.clone(),
                        conflict_type: "overlapping_operations".into(),
                        description: format!(
                            "{item_name} held by {op_count} operations — {total_reserved} of {on_hand} on hand"
                        ),
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                conflicts.push(row.map_err(|e| e.to_string())?);
            }
        }

        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT r.item_id, i.type_name, SUM(r.quantity) AS total_reserved, i.quantity AS on_hand
                     FROM inventory_reservations r
                     JOIN inventory_items i ON i.item_id = r.item_id
                     WHERE r.released_at IS NULL
                     GROUP BY r.item_id
                     HAVING total_reserved > on_hand",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    let item_id: i64 = row.get(0)?;
                    let item_name: String = row.get(1)?;
                    let total_reserved: i64 = row.get(2)?;
                    let on_hand: i64 = row.get(3)?;
                    Ok(ReservationConflict {
                        item_id,
                        item_name: item_name.clone(),
                        conflict_type: "exceeds_stock".into(),
                        description: format!(
                            "{item_name} reserved {total_reserved} against {on_hand} on hand — {} units over-committed",
                            total_reserved - on_hand
                        ),
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                conflicts.push(row.map_err(|e| e.to_string())?);
            }
        }

        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT r.item_id, i.type_name
                     FROM inventory_reservations r
                     JOIN inventory_items i ON i.item_id = r.item_id
                     WHERE r.released_at IS NULL AND i.quantity = 0",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    let item_id: i64 = row.get(0)?;
                    let item_name: String = row.get(1)?;
                    Ok(ReservationConflict {
                        item_id,
                        item_name: item_name.clone(),
                        conflict_type: "missing_inventory".into(),
                        description: format!("{item_name} has an active reservation but zero on-hand stock"),
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                conflicts.push(row.map_err(|e| e.to_string())?);
            }
        }

        Ok(conflicts)
    }

    pub fn inventory_commitment(&self) -> Result<InventoryCommitment, String> {
        let total_inventory: i64 = self
            .conn
            .query_row("SELECT COALESCE(SUM(quantity), 0) FROM inventory_items", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("total inventory query failed: {e}"))?;

        let reserved: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(quantity), 0) FROM inventory_reservations WHERE released_at IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("reserved query failed: {e}"))?;

        // Per-item reserved vs on-hand, to compute available/blocked without
        // letting one over-committed item mask another's genuine
        // availability (or vice versa) — see InventoryCommitment's doc
        // comment for the exact formulas.
        let (available, blocked): (i64, i64) = self
            .conn
            .query_row(
                "SELECT
                    COALESCE(SUM(MAX(i.quantity - COALESCE(res.reserved_qty, 0), 0)), 0),
                    COALESCE(SUM(MAX(COALESCE(res.reserved_qty, 0) - i.quantity, 0)), 0)
                 FROM inventory_items i
                 LEFT JOIN (
                     SELECT item_id, SUM(quantity) AS reserved_qty
                     FROM inventory_reservations WHERE released_at IS NULL
                     GROUP BY item_id
                 ) res ON res.item_id = i.item_id",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| format!("available/blocked query failed: {e}"))?;

        let unallocated: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(i.quantity), 0)
                 FROM inventory_items i
                 WHERE NOT EXISTS (
                     SELECT 1 FROM inventory_reservations r
                     WHERE r.item_id = i.item_id AND r.released_at IS NULL
                 )
                 AND NOT EXISTS (
                     SELECT 1 FROM inventory_allocations a WHERE a.item_id = i.item_id
                 )",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("unallocated query failed: {e}"))?;

        Ok(InventoryCommitment {
            total_inventory,
            reserved,
            available,
            blocked,
            unallocated,
        })
    }

    /// Scoped to one operation. Mineral/component/PI buckets come straight
    /// from that operation's active reservations, joined to
    /// `inventory_categories`. `missing_reservations` compares against
    /// `missing_materials` (migration 0002) by matching type name — it is
    /// only meaningful for operations sharing id space with a
    /// `build_projects` row (see migration 0004's coexistence note); a
    /// standalone operation with no such row simply has none to compare
    /// against and returns 0.
    pub fn operation_reservations(&self, operation_id: i64) -> Result<OperationReservations, String> {
        let bucket = |categories: &[&str]| -> Result<i64, String> {
            let placeholders = categories.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!(
                "SELECT COALESCE(SUM(r.quantity), 0)
                 FROM inventory_reservations r
                 JOIN inventory_items i ON i.item_id = r.item_id
                 WHERE r.released_at IS NULL AND r.operation_id = ? AND i.category_key IN ({placeholders})"
            );
            let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut params: Vec<&dyn rusqlite::ToSql> = vec![&operation_id];
            for c in categories {
                params.push(c);
            }
            stmt.query_row(params.as_slice(), |row| row.get(0))
                .map_err(|e| e.to_string())
        };

        let reserved_minerals = bucket(&["minerals"])?;
        let reserved_components = bucket(&["components", "capital_components", "advanced_components"])?;
        let reserved_pi = bucket(&["pi"])?;

        let missing_reservations: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*)
                 FROM missing_materials m
                 WHERE m.project_id = ?1
                 AND NOT EXISTS (
                     SELECT 1 FROM inventory_reservations r
                     JOIN inventory_items i ON i.item_id = r.item_id
                     WHERE r.operation_id = ?1 AND r.released_at IS NULL AND i.type_name = m.type_name
                 )",
                [operation_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("missing reservations query failed: {e}"))?;

        Ok(OperationReservations {
            reserved_minerals,
            reserved_components,
            reserved_pi,
            missing_reservations,
        })
    }
}
