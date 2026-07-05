//! `InventoryRepository` — the only place that writes SQL against the
//! inventory tables. Engines and commands go through this, never through
//! raw `Connection` calls of their own, so the query surface stays in one
//! auditable place.

use super::models::{
    InventoryAllocation, InventoryCategory, InventoryItem, InventoryLocation, InventoryReservation,
    InventorySummary,
};
use rusqlite::Connection;

pub struct InventoryRepository<'a> {
    conn: &'a Connection,
}

impl<'a> InventoryRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn summary(&self) -> Result<InventorySummary, String> {
        let (total_assets, estimated_value, unique_item_types): (i64, f64, i64) = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(quantity), 0),
                        COALESCE(SUM(quantity * unit_value), 0),
                        COUNT(DISTINCT type_name)
                 FROM inventory_items",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| format!("inventory summary query failed: {e}"))?;

        let locations: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM inventory_locations", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("inventory locations count failed: {e}"))?;

        let reserved_value: f64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(i.unit_value * r.quantity), 0)
                 FROM inventory_reservations r
                 JOIN inventory_items i ON i.item_id = r.item_id
                 WHERE r.released_at IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("inventory reserved-value query failed: {e}"))?;

        Ok(InventorySummary {
            total_assets,
            estimated_value,
            unique_item_types,
            locations,
            reserved_value,
            available_value: estimated_value - reserved_value,
        })
    }

    pub fn categories(&self) -> Result<Vec<InventoryCategory>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT c.key, c.label, c.sort_order,
                        (SELECT COALESCE(SUM(quantity), 0) FROM inventory_items i WHERE i.category_key = c.key)
                 FROM inventory_categories c
                 ORDER BY c.sort_order",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(InventoryCategory {
                    key: row.get(0)?,
                    label: row.get(1)?,
                    sort_order: row.get(2)?,
                    item_count: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// `category_key = None` returns every item ("Everything" view).
    pub fn items(&self, category_key: Option<&str>) -> Result<Vec<InventoryItem>, String> {
        let sql = "
            SELECT
                i.item_id, i.type_name, i.category_key, c.label AS category_label,
                i.quantity,
                COALESCE(loc.name, 'Unknown location') AS location_name,
                COALESCE(ch.name, 'Unassigned') AS owner_name,
                i.unit_value,
                i.state,
                st.label AS state_label,
                COALESCE(res.reserved_qty, 0) AS reserved_qty,
                alloc.project_name AS allocated_operation,
                res.project_name AS reserved_operation
            FROM inventory_items i
            JOIN inventory_categories c ON c.key = i.category_key
            JOIN inventory_states st ON st.key = i.state
            LEFT JOIN inventory_locations loc ON loc.location_id = i.location_id
            LEFT JOIN characters ch ON ch.character_id = i.character_id
            LEFT JOIN (
                SELECT r.item_id, SUM(r.quantity) AS reserved_qty, MAX(bp.name) AS project_name
                FROM inventory_reservations r
                LEFT JOIN build_projects bp ON bp.project_id = r.project_id
                WHERE r.released_at IS NULL
                GROUP BY r.item_id
            ) res ON res.item_id = i.item_id
            LEFT JOIN (
                SELECT a.item_id, MAX(bp.name) AS project_name
                FROM inventory_allocations a
                JOIN build_projects bp ON bp.project_id = a.project_id
                GROUP BY a.item_id
            ) alloc ON alloc.item_id = i.item_id
            WHERE (?1 IS NULL OR i.category_key = ?1)
            ORDER BY (i.quantity * i.unit_value) DESC
        ";

        let mut stmt = self.conn.prepare(sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([category_key], |row| {
                let quantity: i64 = row.get(4)?;
                let reserved_qty: i64 = row.get(10)?;
                let state: String = row.get(8)?;
                let state_label: String = row.get(9)?;
                let allocated_operation: Option<String> = row.get(11)?;
                let reserved_operation: Option<String> = row.get(12)?;
                let unit_value: f64 = row.get(7)?;

                let status = if state != "available" {
                    state_label.clone()
                } else if reserved_qty >= quantity && quantity > 0 {
                    "Reserved".to_string()
                } else if reserved_qty > 0 {
                    "Partially Reserved".to_string()
                } else if allocated_operation.is_some() {
                    "Allocated".to_string()
                } else {
                    "Available".to_string()
                };

                Ok(InventoryItem {
                    item_id: row.get(0)?,
                    type_name: row.get(1)?,
                    category_key: row.get(2)?,
                    category_label: row.get(3)?,
                    quantity,
                    location_name: row.get(5)?,
                    owner_name: row.get(6)?,
                    unit_value,
                    total_value: unit_value * quantity as f64,
                    reserved_quantity: reserved_qty,
                    available_quantity: (quantity - reserved_qty).max(0),
                    allocated_operation,
                    reserved_operation,
                    state,
                    state_label,
                    status,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Read path for a future Locations breakdown view; no command
    /// surfaces it yet, hence the allow — this is a reminder, not a mistake.
    #[allow(dead_code)]
    pub fn locations(&self) -> Result<Vec<InventoryLocation>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT location_id, name, kind, system_name, region_name
                 FROM inventory_locations ORDER BY name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(InventoryLocation {
                    location_id: row.get(0)?,
                    name: row.get(1)?,
                    kind: row.get(2)?,
                    system_name: row.get(3)?,
                    region_name: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Average tier coverage for a build target, read from the tiers
    /// recorded against it (see migration 0002). This is a placeholder
    /// formula: full item-level requirement matching arrives with the
    /// Production Engine (Sprint 004). It is deliberately *not* a stored
    /// literal on `build_projects` — Mission Control asks the Inventory
    /// Engine for this number every time.
    pub fn coverage_for_operation(&self, project_id: i64) -> Result<f64, String> {
        self.conn
            .query_row(
                "SELECT COALESCE(AVG(coverage), 0) FROM build_requirement_groups WHERE project_id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("coverage query failed for project {project_id}: {e}"))
    }

    #[allow(dead_code)] // read path for a future Reservations view; not surfaced this sprint
    pub fn allocations_for(&self, project_id: i64) -> Result<Vec<InventoryAllocation>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, item_id, project_id, quantity FROM inventory_allocations
                 WHERE project_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([project_id], |row| {
                Ok(InventoryAllocation {
                    id: row.get(0)?,
                    item_id: row.get(1)?,
                    project_id: row.get(2)?,
                    quantity: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    #[allow(dead_code)] // read path for a future Reservations view; not surfaced this sprint
    pub fn reservations_for(&self, project_id: i64) -> Result<Vec<InventoryReservation>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, item_id, project_id, quantity, reason, released_at
                 FROM inventory_reservations WHERE project_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([project_id], |row| {
                Ok(InventoryReservation {
                    id: row.get(0)?,
                    item_id: row.get(1)?,
                    project_id: row.get(2)?,
                    quantity: row.get(3)?,
                    reason: row.get(4)?,
                    released_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }
}
