//! Operation shopping lists are derived, never stored. Only the user's
//! operation-specific procurement status and notes are mutable here.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

pub const STATUSES: &[&str] = &[
    "Needed",
    "Planned",
    "Purchased",
    "Skipped",
    "Fulfilled Manually",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingLine {
    pub type_id: i64,
    pub item_name: String,
    pub required_quantity: i64,
    pub owned_quantity: i64,
    pub shortage_quantity: i64,
    pub unit_volume_m3: Option<f64>,
    pub total_volume_m3: Option<f64>,
    pub acquisition_unit_price: Option<f64>,
    pub total_acquisition_cost: Option<f64>,
    pub available_market_volume: i64,
    pub market_status: String,
    pub price_timestamp: Option<String>,
    pub procurement_status: String,
    pub notes: String,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingSummary {
    pub missing_item_types: i64,
    pub total_missing_units: i64,
    pub total_purchase_cost: Option<f64>,
    pub total_purchase_volume_m3: Option<f64>,
    pub priced_lines: i64,
    pub unpriced_lines: i64,
    pub insufficient_volume_lines: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingExports {
    pub eve_multi_buy: String,
    pub discord_report: String,
    pub csv: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationShoppingList {
    pub operation_id: i64,
    pub operation_name: String,
    pub market_profile: String,
    pub lines: Vec<ShoppingLine>,
    pub summary: ShoppingSummary,
    pub exports: ShoppingExports,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcurementState {
    pub operation_id: i64,
    pub type_id: i64,
    pub status: String,
    pub notes: String,
    pub updated_at: String,
}

fn validate_status(status: &str) -> Result<(), String> {
    if STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(format!("invalid procurement status: {status}"))
    }
}

fn load_state(
    conn: &Connection,
    operation_id: i64,
    type_id: i64,
) -> Result<Option<ProcurementState>, String> {
    conn.query_row(
        "SELECT status,notes,updated_at FROM operation_procurement_lines WHERE operation_id=?1 AND type_id=?2",
        (operation_id, type_id),
        |row| {
            Ok(ProcurementState {
                operation_id,
                type_id,
                status: row.get(0)?,
                notes: row.get(1)?,
                updated_at: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(|error| format!("failed to load procurement state: {error}"))
}

pub fn save_state(
    conn: &Connection,
    operation_id: i64,
    type_id: i64,
    status: &str,
    notes: &str,
) -> Result<ProcurementState, String> {
    validate_status(status)?;
    let operation_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM operations WHERE operation_id=?1 AND is_demo=0)",
            [operation_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("failed to validate procurement operation: {error}"))?;
    if !operation_exists {
        return Err(format!("real operation {operation_id} not found"));
    }
    let type_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM eve_types WHERE type_id=?1)",
            [type_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("failed to validate procurement type: {error}"))?;
    if !type_exists {
        return Err(format!("type {type_id} not found in CCP static data"));
    }
    let previous = load_state(conn, operation_id, type_id)?;
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|error| format!("failed to begin procurement update: {error}"))?;
    let result = (|| -> Result<(), String> {
        conn.execute(
            "INSERT INTO operation_procurement_lines(operation_id,type_id,status,notes) VALUES(?1,?2,?3,?4) ON CONFLICT(operation_id,type_id) DO UPDATE SET status=excluded.status,notes=excluded.notes,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')",
            (operation_id, type_id, status, notes),
        )
        .map_err(|error| format!("failed to save procurement state: {error}"))?;
        conn.execute(
            "INSERT INTO operation_procurement_events(operation_id,type_id,previous_status,new_status,notes,event_type) VALUES(?1,?2,?3,?4,?5,'updated')",
            (operation_id, type_id, previous.as_ref().map(|state| state.status.as_str()), status, notes),
        )
        .map_err(|error| format!("failed to record procurement event: {error}"))?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = conn.execute_batch("ROLLBACK");
        return Err(error);
    }
    conn.execute_batch("COMMIT")
        .map_err(|error| format!("failed to commit procurement update: {error}"))?;
    load_state(conn, operation_id, type_id)?
        .ok_or_else(|| "procurement state was not available after a successful save".to_string())
}

pub fn reset_state(conn: &Connection, operation_id: i64) -> Result<i64, String> {
    let mut stmt = conn
        .prepare(
            "SELECT type_id,status,notes FROM operation_procurement_lines WHERE operation_id=?1 ORDER BY type_id",
        )
        .map_err(|error| format!("failed to load procurement state for reset: {error}"))?;
    let rows = stmt
        .query_map([operation_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| format!("failed to read procurement state for reset: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to collect procurement state for reset: {error}"))?;
    drop(stmt);
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|error| format!("failed to begin procurement reset: {error}"))?;
    let result = (|| -> Result<(), String> {
        for (type_id, previous_status, notes) in &rows {
            conn.execute(
                "INSERT INTO operation_procurement_events(operation_id,type_id,previous_status,new_status,notes,event_type) VALUES(?1,?2,?3,'Needed',?4,'reset')",
                (operation_id, type_id, previous_status, notes),
            )
            .map_err(|error| format!("failed to record procurement reset event: {error}"))?;
        }
        conn.execute(
            "DELETE FROM operation_procurement_lines WHERE operation_id=?1",
            [operation_id],
        )
        .map_err(|error| format!("failed to reset procurement state: {error}"))?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = conn.execute_batch("ROLLBACK");
        return Err(error);
    }
    conn.execute_batch("COMMIT")
        .map_err(|error| format!("failed to commit procurement reset: {error}"))?;
    Ok(rows.len() as i64)
}

fn market_status(quote: &crate::market::MarketQuote) -> &'static str {
    if quote.acquisition.available_volume == 0 {
        "No market orders"
    } else if !quote.acquisition.sufficient {
        "Insufficient market volume"
    } else if quote.stale {
        "Stale price"
    } else {
        "Current"
    }
}

pub fn shopping_list(
    conn: &Connection,
    operation_id: i64,
) -> Result<OperationShoppingList, String> {
    let plan = crate::production::repository::ProductionRepository::new(conn)
        .calculate_plan(operation_id)
        .map_err(|error| format!("failed to derive operation shopping list: {error}"))?;
    let mut shortages = plan
        .leaf_totals
        .iter()
        .filter(|line| line.missing_quantity > 0)
        .collect::<Vec<_>>();
    shortages.sort_by(|left, right| {
        left.type_name
            .cmp(&right.type_name)
            .then(left.type_id.cmp(&right.type_id))
    });
    let requests = shortages
        .iter()
        .map(|line| (line.type_id, line.missing_quantity))
        .collect::<Vec<_>>();
    let quotes = crate::market::bulk_quote(conn, &requests)
        .map_err(|error| format!("failed to price operation shopping list: {error}"))?;
    let market_profile = quotes
        .first()
        .map(|quote| quote.market_profile.clone())
        .unwrap_or_else(|| {
            crate::market::selected_profile(conn)
                .map(|profile| profile.display_name)
                .unwrap_or_else(|_| "Selected market unavailable".into())
        });
    let mut lines = Vec::with_capacity(shortages.len());
    for (leaf, quote) in shortages.into_iter().zip(quotes) {
        let state = load_state(conn, operation_id, leaf.type_id)?;
        let complete = quote.acquisition.sufficient;
        lines.push(ShoppingLine {
            type_id: leaf.type_id,
            item_name: leaf.type_name.clone(),
            required_quantity: leaf.required_quantity,
            owned_quantity: leaf.available_quantity.min(leaf.required_quantity),
            shortage_quantity: leaf.missing_quantity,
            unit_volume_m3: leaf.unit_volume_m3,
            total_volume_m3: leaf.missing_volume_m3,
            acquisition_unit_price: complete.then_some(quote.acquisition.unit_price).flatten(),
            total_acquisition_cost: complete.then_some(quote.acquisition.total_price).flatten(),
            available_market_volume: quote.acquisition.available_volume,
            market_status: market_status(&quote).into(),
            price_timestamp: quote.fetched_at,
            procurement_status: state
                .as_ref()
                .map(|value| value.status.clone())
                .unwrap_or_else(|| "Needed".into()),
            notes: state
                .as_ref()
                .map(|value| value.notes.clone())
                .unwrap_or_default(),
            last_updated: state.map(|value| value.updated_at),
        });
    }
    let priced_lines = lines
        .iter()
        .filter(|line| line.total_acquisition_cost.is_some())
        .count() as i64;
    let unpriced_lines = lines
        .iter()
        .filter(|line| line.market_status == "No market orders")
        .count() as i64;
    let insufficient_volume_lines = lines
        .iter()
        .filter(|line| line.market_status == "Insufficient market volume")
        .count() as i64;
    let priced_total = lines
        .iter()
        .filter_map(|line| line.total_acquisition_cost)
        .sum::<f64>();
    let summary = ShoppingSummary {
        missing_item_types: lines.len() as i64,
        total_missing_units: lines.iter().map(|line| line.shortage_quantity).sum(),
        total_purchase_cost: (priced_lines > 0).then_some(priced_total),
        total_purchase_volume_m3: crate::volume::aggregate_volume(
            lines.iter().map(|line| line.total_volume_m3),
        ),
        priced_lines,
        unpriced_lines,
        insufficient_volume_lines,
    };
    let exports = ShoppingExports {
        eve_multi_buy: eve_multi_buy(&lines),
        discord_report: discord_report(&plan.operation_goal, &market_profile, &summary, &lines),
        csv: csv_export(&plan.operation_goal, &lines),
    };
    Ok(OperationShoppingList {
        operation_id,
        operation_name: plan.operation_goal,
        market_profile,
        lines,
        summary,
        exports,
    })
}

fn eve_multi_buy(lines: &[ShoppingLine]) -> String {
    lines
        .iter()
        .map(|line| format!("{} {}", line.item_name, line.shortage_quantity))
        .collect::<Vec<_>>()
        .join("\n")
}

fn number(value: f64) -> String {
    let text = format!("{value:.6}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn isk(value: Option<f64>) -> String {
    value
        .map(|amount| format!("{amount:.2} ISK"))
        .unwrap_or_else(|| "Unpriced".into())
}

fn volume(value: Option<f64>) -> String {
    value
        .map(|amount| format!("{} m³", number(amount)))
        .unwrap_or_else(|| "Unknown m³".into())
}

fn discord_report(
    operation_name: &str,
    market_profile: &str,
    summary: &ShoppingSummary,
    lines: &[ShoppingLine],
) -> String {
    let timestamp = lines
        .iter()
        .filter_map(|line| line.price_timestamp.as_deref())
        .max()
        .unwrap_or("No cached price timestamp");
    let mut out = vec![
        operation_name.to_string(),
        format!("Missing item types: {}", summary.missing_item_types),
        format!(
            "Estimated {market_profile} cost (priced lines): {}",
            isk(summary.total_purchase_cost)
        ),
        format!("Total volume: {}", volume(summary.total_purchase_volume_m3)),
        format!("Pricing timestamp: {timestamp}"),
        String::new(),
    ];
    out.extend(lines.iter().map(|line| {
        format!(
            "• {} × {} — {} — {}",
            line.item_name,
            line.shortage_quantity,
            isk(line.total_acquisition_cost),
            volume(line.total_volume_m3)
        )
    }));
    if summary.unpriced_lines > 0 {
        out.push(format!(
            "WARNING: {} item type(s) have no market orders and are excluded from estimated cost.",
            summary.unpriced_lines
        ));
    }
    if summary.insufficient_volume_lines > 0 {
        out.push(format!(
            "WARNING: {} item type(s) have insufficient market volume and are excluded from estimated cost.",
            summary.insufficient_volume_lines
        ));
    }
    out.join("\n")
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn csv_export(operation_name: &str, lines: &[ShoppingLine]) -> String {
    let mut out = vec!["operation,type_id,item_name,required,owned,shortage,unit_volume_m3,total_volume_m3,unit_price,total_cost,market_status,procurement_status,notes".to_string()];
    out.extend(lines.iter().map(|line| {
        [
            csv_escape(operation_name),
            line.type_id.to_string(),
            csv_escape(&line.item_name),
            line.required_quantity.to_string(),
            line.owned_quantity.to_string(),
            line.shortage_quantity.to_string(),
            line.unit_volume_m3.map(number).unwrap_or_default(),
            line.total_volume_m3.map(number).unwrap_or_default(),
            line.acquisition_unit_price.map(number).unwrap_or_default(),
            line.total_acquisition_cost.map(number).unwrap_or_default(),
            csv_escape(&line.market_status),
            csv_escape(&line.procurement_status),
            csv_escape(&line.notes),
        ]
        .join(",")
    }));
    out.join("\n")
}

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
        include_str!("../../migrations/0012_character_asset_sync.sql"),
        include_str!("../../migrations/0013_character_blueprint_sync.sql"),
        include_str!("../../migrations/0014_location_resolution.sql"),
        include_str!("../../migrations/0015_market_valuation.sql"),
        include_str!("../../migrations/0016_type_volume.sql"),
        include_str!("../../migrations/0017_operation_procurement.sql"),
    ];

    fn fixture() -> (Connection, i64) {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for migration in MIGRATIONS {
            conn.execute_batch(migration).unwrap();
        }
        conn.execute_batch(
            "INSERT INTO eve_categories(category_id,name) VALUES(90,'Test Materials');
             INSERT INTO eve_groups(group_id,category_id,name) VALUES(90,90,'Test Group');
             INSERT INTO eve_types(type_id,name,group_id,is_manufacturable,volume_m3) VALUES
                (100,'Test Widget',90,1,10.0),
                (10,'Alpha Material',90,0,2.0),
                (20,'Beta Material',90,0,3.0),
                (30,'Gamma Material',90,0,4.0),
                (40,'Delta Material',90,0,5.0);
             INSERT INTO blueprint_products(blueprint_type_id,product_type_id,quantity) VALUES(100,100,1);
             INSERT INTO blueprint_materials(blueprint_type_id,material_type_id,quantity) VALUES
                (100,10,10),(100,20,5),(100,30,2),(100,40,3);
             INSERT INTO operations(goal,priority,status,progress,notes,deadline,is_demo)
                VALUES('Build Test Widget',2,'planned',0,'',NULL,0);",
        )
        .unwrap();
        let operation_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO operation_build_targets(operation_id,type_id,quantity_requested,blueprint_mode,assumed_me,assumed_te,assumed_is_bpc) VALUES(?1,100,1,'assumed',0,0,0)",
            [operation_id],
        )
        .unwrap();
        conn.execute_batch(
            "INSERT INTO manual_inventory_entries(type_id,quantity,location_name) VALUES
                (10,4,'Home'),(20,5,'Home');
             INSERT INTO market_refresh_state(profile_id,status,fetched_at,expires_at,order_count,page_count,last_error)
                VALUES(1,'success','2026-07-14T12:00:00Z','2999-01-01T00:00:00Z',3,1,NULL);
             INSERT INTO market_order_cache(profile_id,order_id,type_id,is_buy_order,price,volume_remain,min_volume,order_range,location_id,system_id,fetched_at,expires_at,source) VALUES
                (1,1,10,0,10.0,2,1,'station',60003760,30000142,'2026-07-14T12:00:00Z','2999-01-01T00:00:00Z','test'),
                (1,2,10,0,12.0,10,1,'station',60003760,30000142,'2026-07-14T12:00:00Z','2999-01-01T00:00:00Z','test'),
                (1,3,40,0,7.0,1,1,'station',60003760,30000142,'2026-07-14T12:00:00Z','2999-01-01T00:00:00Z','test');",
        )
        .unwrap();
        (conn, operation_id)
    }

    fn list() -> OperationShoppingList {
        let (conn, operation_id) = fixture();
        shopping_list(&conn, operation_id).unwrap()
    }

    #[test]
    fn shopping_list_contains_shortages_only() {
        let list = list();
        assert_eq!(list.lines.len(), 3);
        assert!(list.lines.iter().all(|line| line.shortage_quantity > 0));
    }

    #[test]
    fn owned_quantity_is_subtracted_by_production_plan() {
        let list = list();
        let alpha = list.lines.iter().find(|line| line.type_id == 10).unwrap();
        assert_eq!(alpha.required_quantity, 10);
        assert_eq!(alpha.owned_quantity, 4);
        assert_eq!(alpha.shortage_quantity, 6);
    }

    #[test]
    fn zero_shortage_is_excluded() {
        assert!(!list().lines.iter().any(|line| line.type_id == 20));
    }

    #[test]
    fn total_purchase_cost_sums_only_complete_quotes() {
        let list = list();
        assert_eq!(list.summary.total_purchase_cost, Some(68.0));
        assert_eq!(list.summary.priced_lines, 1);
    }

    #[test]
    fn total_purchase_volume_uses_ccp_static_volume() {
        let list = list();
        assert_eq!(list.summary.total_purchase_volume_m3, Some(35.0));
        assert_eq!(
            list.lines
                .iter()
                .find(|line| line.type_id == 10)
                .unwrap()
                .total_volume_m3,
            Some(12.0)
        );
    }

    #[test]
    fn no_orders_is_unpriced_not_zero() {
        let gamma = list()
            .lines
            .into_iter()
            .find(|line| line.type_id == 30)
            .unwrap();
        assert_eq!(gamma.market_status, "No market orders");
        assert_eq!(gamma.acquisition_unit_price, None);
        assert_eq!(gamma.total_acquisition_cost, None);
    }

    #[test]
    fn insufficient_market_volume_is_explicit_and_unpriced() {
        let delta = list()
            .lines
            .into_iter()
            .find(|line| line.type_id == 40)
            .unwrap();
        assert_eq!(delta.market_status, "Insufficient market volume");
        assert_eq!(delta.available_market_volume, 1);
        assert_eq!(delta.total_acquisition_cost, None);
    }

    #[test]
    fn stale_complete_price_is_labeled_without_becoming_unpriced() {
        let (conn, operation_id) = fixture();
        conn.execute(
            "UPDATE market_refresh_state SET expires_at='2000-01-01T00:00:00Z' WHERE profile_id=1",
            [],
        )
        .unwrap();
        let alpha = shopping_list(&conn, operation_id)
            .unwrap()
            .lines
            .into_iter()
            .find(|line| line.type_id == 10)
            .unwrap();
        assert_eq!(alpha.market_status, "Stale price");
        assert_eq!(alpha.total_acquisition_cost, Some(68.0));
    }

    #[test]
    fn eve_multi_buy_is_valid_shortage_only_format() {
        assert_eq!(
            list().exports.eve_multi_buy,
            "Alpha Material 6\nDelta Material 3\nGamma Material 2"
        );
    }

    #[test]
    fn discord_report_contains_summary_lines_and_warnings() {
        let report = list().exports.discord_report;
        assert!(report.contains("Build Test Widget"));
        assert!(report.contains("68.00 ISK"));
        assert!(report.contains("35 m³"));
        assert!(report.contains("no market orders"));
        assert!(report.contains("insufficient market volume"));
    }

    #[test]
    fn csv_contains_required_columns_and_escapes_notes() {
        let mut list = list();
        list.lines[0].notes = "buy, after downtime".into();
        let csv = csv_export(&list.operation_name, &list.lines);
        assert!(csv.starts_with("operation,type_id,item_name,required,owned,shortage,unit_volume_m3,total_volume_m3,unit_price,total_cost,market_status,procurement_status,notes"));
        assert!(csv.contains("\"buy, after downtime\""));
    }

    #[test]
    fn ordering_is_stable_by_item_name_then_type_id() {
        let names = list()
            .lines
            .into_iter()
            .map(|line| line.item_name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["Alpha Material", "Delta Material", "Gamma Material"]
        );
    }

    #[test]
    fn procurement_state_saves_and_reloads_with_note() {
        let (conn, operation_id) = fixture();
        save_state(&conn, operation_id, 10, "Planned", "Buy tonight").unwrap();
        let list = shopping_list(&conn, operation_id).unwrap();
        let alpha = list.lines.iter().find(|line| line.type_id == 10).unwrap();
        assert_eq!(alpha.procurement_status, "Planned");
        assert_eq!(alpha.notes, "Buy tonight");
        assert!(alpha.last_updated.is_some());
    }

    #[test]
    fn reset_returns_lines_to_needed_and_preserves_event_history() {
        let (conn, operation_id) = fixture();
        save_state(&conn, operation_id, 10, "Purchased", "Contract 12").unwrap();
        assert_eq!(reset_state(&conn, operation_id).unwrap(), 1);
        let alpha = shopping_list(&conn, operation_id)
            .unwrap()
            .lines
            .into_iter()
            .find(|line| line.type_id == 10)
            .unwrap();
        assert_eq!(alpha.procurement_status, "Needed");
        assert_eq!(alpha.notes, "");
        let events: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM operation_procurement_events WHERE operation_id=?1 AND notes='Contract 12'",
                [operation_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(events, 2);
    }

    #[test]
    fn procurement_state_does_not_change_inventory() {
        let (conn, operation_id) = fixture();
        let before: i64 = conn
            .query_row(
                "SELECT quantity FROM manual_inventory_entries WHERE type_id=10",
                [],
                |row| row.get(0),
            )
            .unwrap();
        save_state(&conn, operation_id, 10, "Purchased", "In transit").unwrap();
        let after: i64 = conn
            .query_row(
                "SELECT quantity FROM manual_inventory_entries WHERE type_id=10",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn later_inventory_increase_naturally_reduces_or_removes_shortage() {
        let (conn, operation_id) = fixture();
        save_state(&conn, operation_id, 10, "Planned", "Existing plan").unwrap();
        conn.execute(
            "UPDATE manual_inventory_entries SET quantity=10 WHERE type_id=10",
            [],
        )
        .unwrap();
        assert!(!shopping_list(&conn, operation_id)
            .unwrap()
            .lines
            .iter()
            .any(|line| line.type_id == 10));
        assert!(load_state(&conn, operation_id, 10).unwrap().is_some());
    }

    #[test]
    fn demo_inventory_is_excluded_from_shortage_math() {
        let (conn, operation_id) = fixture();
        conn.execute_batch(
            "INSERT OR IGNORE INTO inventory_categories(key,label,sort_order) VALUES('test','Test',999);
             INSERT INTO inventory_locations(location_id,name,kind) VALUES(9999,'Demo','station');
             INSERT INTO characters(character_id,name,is_demo) VALUES(-9999,'Demo Shopper',1);
             INSERT INTO inventory_items(type_name,category_key,quantity,location_id,character_id,unit_value,source) VALUES('Alpha Material','test',999999,9999,-9999,1,'demo');",
        )
        .unwrap();
        let alpha = shopping_list(&conn, operation_id)
            .unwrap()
            .lines
            .into_iter()
            .find(|line| line.type_id == 10)
            .unwrap();
        assert_eq!(alpha.shortage_quantity, 6);
    }

    #[test]
    fn backend_derivation_and_state_errors_are_returned() {
        let (conn, _) = fixture();
        assert!(shopping_list(&conn, 999999)
            .unwrap_err()
            .contains("failed to derive operation shopping list"));
        assert!(save_state(&conn, 999999, 10, "Planned", "")
            .unwrap_err()
            .contains("not found"));
        assert!(save_state(&conn, 1, 10, "Invalid", "")
            .unwrap_err()
            .contains("invalid procurement status"));
    }
}
