//! `InventoryRepository` — the only place that writes SQL against the
//! inventory tables. Engines and commands go through this, never through
//! raw `Connection` calls of their own, so the query surface stays in one
//! auditable place.

use super::models::{
    InventoryAllocation, InventoryCategory, InventoryItem, InventoryLocation, InventoryReservation,
    InventorySummary, ManualInventoryEntry, NewManualInventoryEntry, SyncedAsset,
};
use rusqlite::Connection;

pub struct InventoryRepository<'a> {
    conn: &'a Connection,
}

#[cfg(test)]
mod synced_asset_tests {
    use super::*;

    #[test]
    fn synchronized_assets_are_returned_with_owner_type_location_and_source() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE characters(character_id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE eve_types(type_id INTEGER PRIMARY KEY, name TEXT, volume_m3 REAL);
             CREATE TABLE character_assets(
                character_id INTEGER, item_id INTEGER, type_id INTEGER, quantity INTEGER,
                location_id INTEGER, location_type TEXT, location_flag TEXT,
                is_singleton INTEGER, synced_at TEXT);
             CREATE TABLE location_cache(location_id INTEGER PRIMARY KEY,location_kind TEXT,display_name TEXT,solar_system_name TEXT,constellation_name TEXT,region_name TEXT,resolution_status TEXT,resolution_source TEXT,resolved_at TEXT);
             INSERT INTO characters VALUES(1, 'Maxdelta');
             INSERT INTO eve_types VALUES(34, 'Tritanium', 0.01);
             INSERT INTO character_assets VALUES(1, 99, 34, 500, 60003760, 'station', 'Hangar', 0, '2026-07-12T18:35:01Z');"
        ).unwrap();

        let assets = InventoryRepository::new(&conn).list_synced_assets("personal").unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].owner_name, "Maxdelta");
        assert_eq!(assets[0].owner_type, "Personal");
        assert_eq!(assets[0].type_name, "Tritanium");
        assert_eq!(assets[0].location_id, 60003760);
        assert_eq!(assets[0].source, "ESI Character Assets");
        assert_eq!(assets[0].unit_volume_m3, Some(0.01));
        assert_eq!(assets[0].stack_volume_m3, Some(5.0));
    }

    fn ownership_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE characters(character_id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE corporations(corporation_id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE eve_types(type_id INTEGER PRIMARY KEY, name TEXT, volume_m3 REAL);
             CREATE TABLE character_assets(
                character_id INTEGER, item_id INTEGER, type_id INTEGER, quantity INTEGER,
                location_id INTEGER, location_type TEXT, location_flag TEXT,
                is_singleton INTEGER, synced_at TEXT);
             CREATE TABLE corporation_assets(
                corporation_id INTEGER, item_id INTEGER, type_id INTEGER, quantity INTEGER,
                location_id INTEGER, location_type TEXT, location_flag TEXT,
                is_singleton INTEGER, division_name TEXT, synced_at TEXT);
             CREATE TABLE location_cache(location_id INTEGER PRIMARY KEY,location_kind TEXT,display_name TEXT,solar_system_name TEXT,constellation_name TEXT,region_name TEXT,resolution_status TEXT,resolution_source TEXT,resolved_at TEXT);
             INSERT INTO characters VALUES(1, 'Pilot One');
             INSERT INTO corporations VALUES(10, 'Industry Corp');
             INSERT INTO eve_types VALUES(34, 'Tritanium', 0.01);
             INSERT INTO character_assets VALUES(1, 101, 34, 5, 60003760, 'station', 'Hangar', 0, '2026-07-17T12:00:00Z');
             INSERT INTO corporation_assets VALUES(10, 201, 34, 7, 60003760, 'station', 'CorpSAG2', 0, 'Minerals', '2026-07-17T12:01:00Z');"
        ).unwrap();
        conn
    }

    #[test]
    fn owner_scope_filters_keep_personal_and_corporation_rows_distinct() {
        let conn = ownership_db();
        let repository = InventoryRepository::new(&conn);
        let personal = repository.list_synced_assets("personal").unwrap();
        assert_eq!((personal.len(), personal[0].owner_type.as_str(), personal[0].quantity), (1, "Personal", 5));
        let corporation = repository.list_synced_assets("corporation").unwrap();
        assert_eq!((corporation.len(), corporation[0].owner_type.as_str(), corporation[0].quantity), (1, "Corporation", 7));
        assert_eq!(corporation[0].owner_name, "Industry Corp");
        assert_eq!(corporation[0].division.as_deref(), Some("Minerals"));
        let both = repository.list_synced_assets("both").unwrap();
        assert_eq!(both.len(), 2);
        assert!(both.iter().any(|asset| asset.item_id == 101));
        assert!(both.iter().any(|asset| asset.item_id == 201));
    }

    #[test]
    fn invalid_owner_scope_is_rejected() {
        let conn = ownership_db();
        let error = InventoryRepository::new(&conn).list_synced_assets("global").err().unwrap();
        assert!(error.contains("personal, corporation, or both"));
    }
}

impl<'a> InventoryRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn list_synced_assets(&self, owner_scope:&str) -> Result<Vec<SyncedAsset>, String> {
        if !["personal","corporation","both"].contains(&owner_scope){return Err("owner scope must be personal, corporation, or both".into())}
        let mut output=Vec::new();
        if owner_scope!="corporation" { output.extend(self.list_personal_assets()?); }
        if owner_scope!="personal" { output.extend(self.list_corporation_assets()?); }
        output.sort_by(|a,b|(&a.owner_type,&a.owner_name,a.location_id,&a.type_name,a.item_id).cmp(&(&b.owner_type,&b.owner_name,b.location_id,&b.type_name,b.item_id)));
        Ok(output)
    }

    fn list_personal_assets(&self) -> Result<Vec<SyncedAsset>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT a.character_id, c.name, a.type_id,
                    COALESCE(t.name, 'Unknown Type ' || a.type_id), a.quantity, a.item_id,
                    a.location_id, a.location_type, a.location_flag, a.is_singleton, a.synced_at,
                    t.volume_m3
             FROM character_assets a
             JOIN characters c ON c.character_id = a.character_id
             LEFT JOIN eve_types t ON t.type_id = a.type_id
             ORDER BY c.name, a.location_id,
                      COALESCE(t.name, 'Unknown Type ' || a.type_id), a.item_id"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| { let character_id=row.get(0)?; let location_id=row.get(6)?; Ok(SyncedAsset {
            owner_type:"Personal".into(),owner_id:character_id,owner_name:row.get(1)?,character_id:Some(character_id),corporation_id:None,division:None,type_id: row.get(2)?,
            type_name: row.get(3)?, quantity: row.get(4)?, item_id: row.get(5)?,
            unit_volume_m3: row.get(11)?,
            stack_volume_m3: crate::volume::volume_for_quantity(row.get(11)?, row.get(4)?),
            location_id, location_type: row.get(7)?, location_flag: row.get(8)?,
            singleton: row.get::<_, i64>(9)? != 0, source: "ESI Character Assets".into(),
            last_synced: row.get(10)?,
            resolved_location: crate::location::resolve(self.conn,character_id,location_id).map_err(|e|rusqlite::Error::ToSqlConversionFailure(e.into()))?,
        })}).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    fn list_corporation_assets(&self)->Result<Vec<SyncedAsset>,String>{
        let mut stmt=self.conn.prepare("SELECT a.corporation_id,c.name,a.type_id,COALESCE(t.name,'Unknown Type '||a.type_id),a.quantity,a.item_id,a.location_id,a.location_type,a.location_flag,a.is_singleton,a.synced_at,t.volume_m3,a.division_name FROM corporation_assets a JOIN corporations c ON c.corporation_id=a.corporation_id LEFT JOIN eve_types t ON t.type_id=a.type_id ORDER BY c.name,a.location_id,COALESCE(t.name,'Unknown Type '||a.type_id),a.item_id").map_err(|e|e.to_string())?;
        let rows=stmt.query_map([],|row|{let corporation_id=row.get(0)?;let location_id=row.get(6)?;let quantity=row.get(4)?;let unit=row.get(11)?;Ok(SyncedAsset{owner_type:"Corporation".into(),owner_id:corporation_id,owner_name:row.get(1)?,character_id:None,corporation_id:Some(corporation_id),division:row.get(12)?,type_id:row.get(2)?,type_name:row.get(3)?,quantity,item_id:row.get(5)?,unit_volume_m3:unit,stack_volume_m3:crate::volume::volume_for_quantity(unit,quantity),location_id,location_type:row.get(7)?,location_flag:row.get(8)?,singleton:row.get::<_,i64>(9)?!=0,source:"ESI Corporation Assets".into(),last_synced:row.get(10)?,resolved_location:crate::location::resolve_corporation(self.conn,corporation_id,location_id).map_err(|e|rusqlite::Error::ToSqlConversionFailure(e.into()))?})}).map_err(|e|e.to_string())?;
        rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
    }

    /// Normal runtime: excludes demo-seeded rows (`source = 'demo'`).
    /// Real inventory lives in `manual_inventory_entries`, a separate
    /// table with its own commands — this function and the demo
    /// `inventory_items` table it reads are retained for developer/test
    /// fixture use only; no normal UI calls this after Sprint 010's demo
    /// retirement. See docs/architecture/DATA_OWNERSHIP.md.
    pub fn summary(&self) -> Result<InventorySummary, String> {
        let (total_assets, estimated_value, unique_item_types): (i64, f64, i64) = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(quantity), 0),
                        COALESCE(SUM(quantity * unit_value), 0),
                        COUNT(DISTINCT type_name)
                 FROM inventory_items WHERE source != 'demo'",
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
                 WHERE r.released_at IS NULL AND i.source != 'demo'",
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
                        (SELECT COALESCE(SUM(quantity), 0) FROM inventory_items i WHERE i.category_key = c.key AND i.source != 'demo')
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
                res.project_name AS reserved_operation,
                COALESCE(alloc.allocated_qty, 0) AS allocated_qty
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
                SELECT a.item_id, SUM(a.quantity) AS allocated_qty, MAX(bp.name) AS project_name
                FROM inventory_allocations a
                JOIN build_projects bp ON bp.project_id = a.project_id
                GROUP BY a.item_id
            ) alloc ON alloc.item_id = i.item_id
            WHERE (?1 IS NULL OR i.category_key = ?1) AND i.source != 'demo'
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
                let allocated_qty: i64 = row.get(13)?;

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
                    // First increment beyond reserved/available (Sprint 003):
                    // Free also nets out soft allocations, so it answers
                    // "untouched by any operation at all," not just "not
                    // hard-reserved." Does not yet deduplicate an item that
                    // carries both an allocation and a reservation from the
                    // same operation (see reservation::models::InventoryCommitment
                    // doc comment for the equivalent caveat at the aggregate
                    // level) — a refinement for when the Reservation Engine's
                    // mutations are real.
                    free_quantity: (quantity - reserved_qty - allocated_qty).max(0),
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

    // ------------------------------------------------------------------
    // Manual inventory (Sprint 008) — real, user-entered stock, always
    // tied to an imported `eve_types` row. Entirely separate storage from
    // `inventory_items` above; nothing here touches that table.
    // ------------------------------------------------------------------

    pub fn list_manual_entries(&self) -> Result<Vec<ManualInventoryEntry>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT m.id, m.type_id, t.name, m.quantity, m.location_name, m.created_at, m.updated_at, t.volume_m3
                 FROM manual_inventory_entries m
                 JOIN eve_types t ON t.type_id = m.type_id
                 ORDER BY m.updated_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ManualInventoryEntry {
                    id: row.get(0)?,
                    type_id: row.get(1)?,
                    type_name: row.get(2)?,
                    quantity: row.get(3)?,
                    unit_volume_m3: row.get(7)?,
                    stack_volume_m3: crate::volume::volume_for_quantity(row.get(7)?, row.get(3)?),
                    location_name: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn add_manual_entry(&self, input: &NewManualInventoryEntry) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO manual_inventory_entries (type_id, quantity, location_name)
                 VALUES (?1, ?2, ?3)",
                (
                    input.type_id,
                    input.quantity,
                    input.location_name.as_deref().unwrap_or("Unspecified"),
                ),
            )
            .map_err(|e| format!("failed to add manual inventory entry: {e}"))?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Bulk insert for Paste Inventory (Sprint 008.2) — one transaction,
    /// so a large pasted list either all lands or none of it does.
    /// Callers merge duplicate type names before calling this; each entry
    /// here becomes its own row (matching the existing single-entry
    /// behavior — merging is a UI-preview concern, not a storage rule).
    pub fn add_manual_entries_bulk(&self, entries: &[NewManualInventoryEntry]) -> Result<i64, String> {
        self.conn
            .execute_batch("BEGIN;")
            .map_err(|e| format!("failed to begin bulk insert transaction: {e}"))?;

        let result: Result<i64, String> = (|| {
            let mut inserted = 0i64;
            for input in entries {
                self.conn
                    .execute(
                        "INSERT INTO manual_inventory_entries (type_id, quantity, location_name)
                         VALUES (?1, ?2, ?3)",
                        (
                            input.type_id,
                            input.quantity,
                            input.location_name.as_deref().unwrap_or("Unspecified"),
                        ),
                    )
                    .map_err(|e| format!("failed to insert entry for type {}: {e}", input.type_id))?;
                inserted += 1;
            }
            Ok(inserted)
        })();

        match result {
            Ok(count) => {
                self.conn
                    .execute_batch("COMMIT;")
                    .map_err(|e| format!("failed to commit bulk insert: {e}"))?;
                Ok(count)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK;");
                Err(e)
            }
        }
    }

    pub fn update_manual_entry_quantity(&self, id: i64, quantity: i64) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "UPDATE manual_inventory_entries
                 SET quantity = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                 WHERE id = ?2",
                (quantity, id),
            )
            .map_err(|e| format!("failed to update manual inventory entry {id}: {e}"))?;
        if changed == 0 {
            return Err(format!("manual inventory entry {id} not found"));
        }
        Ok(())
    }

    pub fn remove_manual_entry(&self, id: i64) -> Result<(), String> {
        let changed = self
            .conn
            .execute("DELETE FROM manual_inventory_entries WHERE id = ?1", [id])
            .map_err(|e| format!("failed to remove manual inventory entry {id}: {e}"))?;
        if changed == 0 {
            return Err(format!("manual inventory entry {id} not found"));
        }
        Ok(())
    }
}
