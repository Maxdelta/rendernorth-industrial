//! Tauri IPC commands. Thin layer: read from the data layer, shape DTOs.
//! No business math lives here once real engines exist. All queries are
//! generic over the selected build target — no ship is special-cased.

use crate::blueprint::{
    self,
    models::{
        BlueprintDetail, BlueprintReadiness, BlueprintRecord, BlueprintSummary,
        MissingBlueprintReport,
    },
};
use crate::character::{
    self,
    models::CharacterSummary,
};
use crate::db::Db;
use crate::inventory::{
    self,
    models::{
        InventoryCategory, InventoryItem, InventorySummary, ManualInventoryEntry,
        NewManualInventoryEntry,
    },
};
use crate::models::*;
use crate::operation::{
    self,
    models::{CreatedOperation, NewOperationInput, OperationDetail, OperationsDashboard},
};
use crate::production::{
    self,
    models::{
        CriticalBottleneck, OperationRequirementBreakdown, ProductionPlan, RequirementCategory,
        RequirementDetail, RequirementLine, RequirementShortage, RequirementSummary,
    },
};
use crate::reservation::{
    self,
    models::{
        InventoryCommitment, OperationReservations, ReservationConflict, ReservationDetail,
        ReservationRecord, ReservationSummary,
    },
};
use crate::staticdata::{self, models::{ImportSummary, TypeSearchResult}};
use rusqlite::Connection;
use tauri::State;

#[tauri::command]
pub fn health_check(db: State<'_, Db>) -> Result<DbHealth, String> {
    let version = db.schema_version()?;
    Ok(DbHealth {
        ok: version >= 8,
        schema_version: version,
        db_path: db.path.display().to_string(),
    })
}

#[tauri::command]
pub fn get_mission_control(db: State<'_, Db>) -> Result<MissionControl, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    mission_control(&conn)
}

#[tauri::command]
pub fn list_build_targets(db: State<'_, Db>) -> Result<Vec<BuildTargetSummary>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    let selected = selected_project_id(&conn)?;

    let mut stmt = conn
        .prepare(
            "SELECT project_id, name, target_type_name, status, overall_progress
             FROM build_projects ORDER BY project_id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let project_id: i64 = row.get(0)?;
            let target_type_name: String = row.get(2)?;
            Ok(BuildTargetSummary {
                project_id,
                name: row.get(1)?,
                class_name: class_from_type_name(&target_type_name),
                status: row.get(3)?,
                overall_progress: row.get(4)?,
                is_selected: project_id == selected,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// Point Mission Control at a different build target, then return the fresh
/// Mission Control state in one round trip.
#[tauri::command]
pub fn select_build_target(db: State<'_, Db>, project_id: i64) -> Result<MissionControl, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;

    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM build_projects WHERE project_id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists == 0 {
        return Err(format!("unknown build target: {project_id}"));
    }

    conn.execute(
        "INSERT INTO app_meta (key, value) VALUES ('selected_project_id', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [project_id.to_string()],
    )
    .map_err(|e| e.to_string())?;

    mission_control(&conn)
}

// ---------------------------------------------------------------------------

fn selected_project_id(conn: &Connection) -> Result<i64, String> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM app_meta WHERE key = 'selected_project_id'",
            [],
            |row| row.get(0),
        )
        .ok();
    if let Some(id) = stored.and_then(|v| v.parse::<i64>().ok()) {
        return Ok(id);
    }
    conn.query_row(
        "SELECT project_id FROM build_projects ORDER BY project_id LIMIT 1",
        [],
        |row| row.get(0),
    )
    .map_err(|e| format!("no build projects exist: {e}"))
}

/// Seed rows label targets as "Name (Class description)". Derive the class
/// generically; no per-hull logic.
fn class_from_type_name(target_type_name: &str) -> String {
    target_type_name
        .split_once('(')
        .map(|(_, rest)| rest.trim_end_matches(')').trim().to_string())
        .unwrap_or_else(|| target_type_name.to_string())
}

/// Deterministic health status derivation — documented in UI_CONSTITUTION.md.
fn derive_status(health: f64, blocked_jobs: i64) -> &'static str {
    if blocked_jobs > 0 {
        "Blocked"
    } else if health < 0.6 {
        "Degraded"
    } else {
        "Operational"
    }
}

fn mission_control(conn: &Connection) -> Result<MissionControl, String> {
    let project_id = selected_project_id(conn)?;

    let running_jobs = metric(conn, "running_jobs")? as i64;
    let idle_characters = metric(conn, "idle_characters")? as i64;
    let idle_bpos = metric(conn, "idle_bpos")? as i64;
    let wallet_isk = metric(conn, "wallet_isk")?;

    let (name, target_type_name): (String, String) = conn
        .query_row(
            "SELECT name, target_type_name FROM build_projects WHERE project_id = ?1",
            [project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("selected build target missing: {e}"))?;

    // Operations do not own inventory or a hardcoded percentage — Mission
    // Control asks the Inventory Engine how covered the selected target is,
    // every time. See InventoryRepository::coverage_for_operation for the
    // current (tier-average) formula and its Sprint 004 successor.
    let overall_progress = inventory::engine_for(conn).coverage_for_operation(project_id)?;

    let mut tiers = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT label, coverage FROM build_requirement_groups
                 WHERE project_id = ?1 ORDER BY sort_order",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([project_id], |row| {
                Ok(TierCoverage {
                    label: row.get(0)?,
                    coverage: row.get(1)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            tiers.push(row.map_err(|e| e.to_string())?);
        }
    }

    let mut missing_materials = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT type_name, quantity, category FROM missing_materials
                 WHERE project_id = ?1 ORDER BY quantity DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([project_id], |row| {
                Ok(MissingMaterial {
                    name: row.get(0)?,
                    quantity: row.get(1)?,
                    category: row.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            missing_materials.push(row.map_err(|e| e.to_string())?);
        }
    }

    // Top active recommendation for the selected target. `reason` is stored
    // as JSON per the Determinism Doctrine: rule_id, summary, inputs.
    let (title, reason_json): (String, String) = conn
        .query_row(
            "SELECT title, reason FROM recommendations
             WHERE project_id = ?1 AND dismissed_at IS NULL
             ORDER BY priority LIMIT 1",
            [project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("no recommendation for target {project_id}: {e}"))?;

    let parsed: serde_json::Value =
        serde_json::from_str(&reason_json).unwrap_or(serde_json::Value::Null);
    let recommendation = Recommendation {
        title,
        reason: parsed
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or(&reason_json)
            .to_string(),
        rule_id: parsed
            .get("rule_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unspecified")
            .to_string(),
        inputs: parsed
            .get("inputs")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    };

    let health = metric(conn, "factory_health")?;
    let blocked_jobs = metric(conn, "blocked_jobs")? as i64;
    let factory_health = FactoryHealth {
        health,
        status: derive_status(health, blocked_jobs).to_string(),
        idle_slots: metric(conn, "idle_slots")? as i64,
        blocked_jobs,
        missing_inputs: missing_materials.len() as i64,
        isk_locked_in_jobs: metric(conn, "isk_locked_in_jobs")?,
        projected_finish_days: metric(conn, "projected_finish_days")? as i64,
    };

    Ok(MissionControl {
        source: "sqlite".into(),
        as_of: chrono::Utc::now().to_rfc3339(),
        running_jobs,
        idle_characters,
        idle_bpos,
        wallet_isk,
        factory_health,
        selected_target: SelectedTarget {
            project_id,
            name,
            class_name: class_from_type_name(&target_type_name),
            overall_progress,
            tiers,
        },
        missing_materials,
        recommendation,
    })
}

fn metric(conn: &Connection, key: &str) -> Result<f64, String> {
    conn.query_row(
        "SELECT value FROM factory_snapshot WHERE metric = ?1",
        [key],
        |row| row.get(0),
    )
    .map_err(|e| format!("missing factory metric '{key}': {e}"))
}

// ---------------------------------------------------------------------------
// Inventory Engine commands. Thin: every question is answered by
// `inventory::engine_for(conn)`, never by ad hoc SQL in this file.

#[tauri::command]
pub fn get_inventory_summary(db: State<'_, Db>) -> Result<InventorySummary, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    inventory::engine_for(&conn).summary()
}

#[tauri::command]
pub fn list_inventory_categories(db: State<'_, Db>) -> Result<Vec<InventoryCategory>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    inventory::engine_for(&conn).categories()
}

#[tauri::command]
pub fn list_inventory_items(
    db: State<'_, Db>,
    category_key: Option<String>,
) -> Result<Vec<InventoryItem>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    inventory::engine_for(&conn).items(category_key.as_deref())
}

// ---------------------------------------------------------------------------
// Operation Engine commands. Thin: every question is answered by
// `operation::engine_for(conn)`, never by ad hoc SQL in this file. Mission
// Control's Operations Dashboard and the Operations Workspace page both
// read exclusively through these.

#[tauri::command]
pub fn get_operations_dashboard(db: State<'_, Db>) -> Result<OperationsDashboard, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    let engine = operation::engine_for(&conn);

    Ok(OperationsDashboard {
        health: engine.health()?,
        current_operations: engine.list_summaries()?,
        priority_queue: engine.priority_queue()?,
        blocked: engine.blocked()?,
        upcoming_completions: engine.upcoming_completions(5)?,
    })
}

#[tauri::command]
pub fn get_operation_detail(
    db: State<'_, Db>,
    operation_id: i64,
) -> Result<OperationDetail, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    operation::engine_for(&conn).get_detail(operation_id)
}

// ---------------------------------------------------------------------------
// Reservation Engine commands. Thin: every question is answered by
// `reservation::engine_for(conn)`, never by ad hoc SQL in this file.
// Mission Control's Inventory Commitment panel and the Operations
// Workspace's Reservations section both read exclusively through these.

#[tauri::command]
pub fn get_inventory_commitment(db: State<'_, Db>) -> Result<InventoryCommitment, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    reservation::engine_for(&conn).inventory_commitment()
}

#[tauri::command]
pub fn get_reservation_summary(db: State<'_, Db>) -> Result<ReservationSummary, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    reservation::engine_for(&conn).summary()
}

#[tauri::command]
pub fn list_active_reservations(db: State<'_, Db>) -> Result<Vec<ReservationRecord>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    reservation::engine_for(&conn).list_active()
}

#[tauri::command]
pub fn get_reservation_detail(
    db: State<'_, Db>,
    reservation_id: i64,
) -> Result<ReservationDetail, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    reservation::engine_for(&conn).get_detail(reservation_id)
}

#[tauri::command]
pub fn get_reservation_conflicts(db: State<'_, Db>) -> Result<Vec<ReservationConflict>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    reservation::engine_for(&conn).conflicts()
}

#[tauri::command]
pub fn get_operation_reservations(
    db: State<'_, Db>,
    operation_id: i64,
) -> Result<OperationReservations, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    reservation::engine_for(&conn).operation_reservations(operation_id)
}

// ---------------------------------------------------------------------------
// Blueprint Engine commands. Thin: every question is answered by
// `blueprint::engine_for(conn)`, never by ad hoc SQL in this file. The
// Blueprints page, Mission Control's Blueprint Readiness panel, and the
// Operations Workspace's Blueprints section all read exclusively through
// these.

#[tauri::command]
pub fn get_blueprint_summary(db: State<'_, Db>) -> Result<BlueprintSummary, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    blueprint::engine_for(&conn).summary()
}

#[tauri::command]
pub fn list_blueprints(db: State<'_, Db>) -> Result<Vec<BlueprintRecord>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    blueprint::engine_for(&conn).list()
}

#[tauri::command]
pub fn get_blueprint_detail(
    db: State<'_, Db>,
    blueprint_id: i64,
) -> Result<BlueprintDetail, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    blueprint::engine_for(&conn).get_detail(blueprint_id)
}

#[tauri::command]
pub fn get_missing_blueprint_report(
    db: State<'_, Db>,
) -> Result<Vec<MissingBlueprintReport>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    blueprint::engine_for(&conn).missing_report()
}

#[tauri::command]
pub fn get_blueprint_readiness_all(db: State<'_, Db>) -> Result<Vec<BlueprintReadiness>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    blueprint::engine_for(&conn).readiness_all()
}

#[tauri::command]
pub fn get_blueprint_readiness_for_operation(
    db: State<'_, Db>,
    operation_id: i64,
) -> Result<BlueprintReadiness, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    blueprint::engine_for(&conn).readiness_for_operation(operation_id)
}

// ---------------------------------------------------------------------------
// Production Requirement Engine commands. Thin: every question is
// answered by `production::engine_for(conn)`, never by ad hoc SQL in this
// file. Mission Control's Production Readiness panel, the Operations
// Workspace's Production Requirements section, and the Production page
// all read exclusively through these.

#[tauri::command]
pub fn get_requirement_summary(db: State<'_, Db>) -> Result<RequirementSummary, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    production::engine_for(&conn).summary()
}

#[tauri::command]
pub fn list_requirement_lines(
    db: State<'_, Db>,
    operation_id: Option<i64>,
    category_key: Option<String>,
) -> Result<Vec<RequirementLine>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    production::engine_for(&conn).lines(operation_id, category_key.as_deref())
}

#[tauri::command]
pub fn get_requirement_detail(
    db: State<'_, Db>,
    requirement_id: i64,
) -> Result<RequirementDetail, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    production::engine_for(&conn).get_detail(requirement_id)
}

#[tauri::command]
pub fn list_requirement_categories(db: State<'_, Db>) -> Result<Vec<RequirementCategory>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    production::engine_for(&conn).categories()
}

#[tauri::command]
pub fn get_requirement_shortages(db: State<'_, Db>) -> Result<Vec<RequirementShortage>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    production::engine_for(&conn).shortages()
}

#[tauri::command]
pub fn get_critical_bottlenecks(db: State<'_, Db>) -> Result<Vec<CriticalBottleneck>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    production::engine_for(&conn).critical_bottlenecks()
}

#[tauri::command]
pub fn get_operation_requirement_breakdown(
    db: State<'_, Db>,
    operation_id: i64,
) -> Result<OperationRequirementBreakdown, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    production::engine_for(&conn).breakdown_for_operation(operation_id)
}

#[tauri::command]
pub fn get_build_target_requirement_breakdown(
    db: State<'_, Db>,
    project_id: i64,
) -> Result<OperationRequirementBreakdown, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    production::engine_for(&conn).requirements_for_build_target(project_id)
}

// ---------------------------------------------------------------------------
// Static Data Import commands (Sprint 008). Owns official reference-data
// ingestion only — never touches operations, inventory, reservations, or
// settings. No network access; the user supplies a local directory path.

#[tauri::command]
pub fn import_static_data(db: State<'_, Db>, dir_path: String) -> Result<ImportSummary, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    staticdata::import_from_directory(&conn, &dir_path)
}

/// Official CCP JSON Lines SDE import (Sprint 008.2) — a separate entry
/// point from `import_static_data`; the sample CSV fixture path above is
/// untouched. See `staticdata::jsonl` for the field-mapping disclosure.
#[tauri::command]
pub fn import_official_sde(db: State<'_, Db>, dir_path: String) -> Result<ImportSummary, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    staticdata::import_official_sde(&conn, &dir_path)
}

/// Returns `None` rather than an error when no import has ever run, so the
/// frontend can show a clean "no static data yet" state instead of an
/// alarming failure.
#[tauri::command]
pub fn get_latest_import(db: State<'_, Db>) -> Result<Option<ImportSummary>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    match staticdata::latest_import(&conn) {
        Ok(summary) => Ok(Some(summary)),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub fn search_eve_types(
    db: State<'_, Db>,
    query: String,
    limit: i64,
) -> Result<Vec<TypeSearchResult>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    staticdata::search_types(&conn, &query, limit)
}

// ---------------------------------------------------------------------------
// Real operation creation (Sprint 008). The one Operation Engine mutation
// upgraded from an architecture-only stub to a genuine implementation —
// every other mutation on OperationEngine remains a stub.

#[tauri::command]
pub fn create_real_operation(
    db: State<'_, Db>,
    input: NewOperationInput,
) -> Result<CreatedOperation, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    let operation_id = operation::engine_for(&conn).create_operation(&input)?;
    Ok(CreatedOperation { operation_id })
}

/// Real operation deletion (Sprint 009.1). Refuses demo operations before
/// any transaction begins; deletes the operation and every row it owns,
/// transactionally, in one call.
#[tauri::command]
pub fn delete_real_operation(db: State<'_, Db>, operation_id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    operation::engine_for(&conn).delete_operation(operation_id)
}

// ---------------------------------------------------------------------------
// Real production plan calculation (Sprint 008). Recursive, deterministic,
// derived live from imported static data + owned/assumed blueprint state +
// inventory — never a stored flag.

#[tauri::command]
pub fn calculate_production_plan(
    db: State<'_, Db>,
    operation_id: i64,
) -> Result<ProductionPlan, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    production::engine_for(&conn).calculate_plan(operation_id)
}

// ---------------------------------------------------------------------------
// Manual inventory commands (Sprint 008). Real, user-entered stock, kept
// entirely separate from the demo-seeded inventory_items table.

#[tauri::command]
pub fn list_manual_inventory(db: State<'_, Db>) -> Result<Vec<ManualInventoryEntry>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    inventory::engine_for(&conn).list_manual_entries()
}

#[tauri::command]
pub fn add_manual_inventory_entry(
    db: State<'_, Db>,
    input: NewManualInventoryEntry,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    inventory::engine_for(&conn).add_manual_entry(&input)
}

/// Paste Inventory bulk add (Sprint 008.2). The frontend has already
/// parsed, validated, matched, and merged duplicate rows by the time this
/// is called — this command only persists the confirmed result, in one
/// transaction.
#[tauri::command]
pub fn add_manual_inventory_bulk(
    db: State<'_, Db>,
    entries: Vec<NewManualInventoryEntry>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    inventory::engine_for(&conn).add_manual_entries_bulk(&entries)
}

#[tauri::command]
pub fn update_manual_inventory_quantity(
    db: State<'_, Db>,
    id: i64,
    quantity: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    inventory::engine_for(&conn).update_manual_entry_quantity(id, quantity)
}

#[tauri::command]
pub fn remove_manual_inventory_entry(db: State<'_, Db>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    inventory::engine_for(&conn).remove_manual_entry(id)
}

// ---------------------------------------------------------------------------
// Generic app settings (Sprint 008.1). Reuses the existing `app_meta`
// key/value table (migration 0001, already used for `selected_project_id`
// since Sprint 002) rather than adding a new table or engine. Intentionally
// generic — any future UI preference can reuse these two commands.

#[tauri::command]
pub fn get_app_setting(db: State<'_, Db>, key: String) -> Result<Option<String>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    conn.query_row("SELECT value FROM app_meta WHERE key = ?1", [key], |row| row.get(0))
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(format!("failed to read app setting: {other}")),
        })
}

#[tauri::command]
pub fn set_app_setting(db: State<'_, Db>, key: String, value: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    conn.execute(
        "INSERT INTO app_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )
    .map_err(|e| format!("failed to write app setting: {e}"))?;
    Ok(())
}

// ============================================================
// Sprint 011A — EVE SSO Character Authentication Foundation.
// Authentication only: Add/Remove/Enable/List a connected character.
// No asset sync, no universe map, no inventory scope, no production
// integration — those are explicitly out of scope for this sprint.
// See docs/ESI_INTEGRATION.md for the design and
// docs/SPRINT_011A_SETUP.md for exact EVE Developer app configuration.
// ============================================================

/// Blocking (opens the system browser, waits on the loopback listener).
/// Deliberately a *non-async* command — Tauri dispatches non-async
/// command handlers off its main thread automatically, which is exactly
/// what a multi-minute wait on user login needs, without requiring `Db`
/// to be `Arc`-wrapped/cloned the way an explicit `spawn_blocking` would.
#[tauri::command]
pub fn add_character(db: State<'_, Db>, client_id: String) -> Result<CharacterSummary, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    character::engine::add_character(&conn, &client_id)
}

#[tauri::command]
pub fn remove_character(db: State<'_, Db>, character_id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    character::engine::remove_character(&conn, character_id)
}

#[tauri::command]
pub fn set_character_enabled(db: State<'_, Db>, character_id: i64, enabled: bool) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    character::engine::set_enabled(&conn, character_id, enabled)
}

#[tauri::command]
pub fn list_characters(db: State<'_, Db>) -> Result<Vec<CharacterSummary>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    character::engine::list_characters(&conn)
}

#[tauri::command]
pub fn sync_character_assets(db: State<'_, Db>, client_id: String, character_id: i64) -> Result<character::assets::SyncResult, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    character::assets::sync_one(&conn, &client_id, character_id)
}

#[tauri::command]
pub fn sync_all_character_assets(db: State<'_, Db>, client_id: String) -> Result<Vec<character::assets::SyncResult>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    character::assets::sync_all(&conn, &client_id)
}

#[tauri::command]
pub fn list_synced_assets(db: State<'_, Db>) -> Result<Vec<inventory::models::SyncedAsset>, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    inventory::repository::InventoryRepository::new(&conn).list_synced_assets()
}

#[tauri::command]
pub async fn refresh_asset_locations(
    db: State<'_, Db>,
    client_id: String,
) -> Result<crate::location::RefreshResult, String> {
    // Location resolution may perform several sequential CCP requests. Run it
    // away from Tauri's event loop and use a separate SQLite connection so
    // Inventory/Blueprint reads are not blocked behind the shared Db mutex.
    let path = db.path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&path)
            .map_err(|e| format!("failed to open location worker database: {e}"))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| format!("failed to configure location worker database: {e}"))?;
        crate::location::refresh(&conn, &client_id)
    })
    .await
    .map_err(|e| format!("location worker failed: {e}"))?
}

#[tauri::command]
pub fn get_market_profile(db: State<'_, Db>) -> Result<crate::market::MarketProfile, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    crate::market::selected_profile(&conn)
}

#[tauri::command]
pub async fn refresh_market_prices(
    db: State<'_, Db>,
) -> Result<crate::market::RefreshResult, String> {
    let path = db.path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
        crate::market::refresh(&conn)
    })
    .await
    .map_err(|e| format!("market refresh worker failed: {e}"))?
}

#[tauri::command]
pub fn search_inventory_market(
    db: State<'_, Db>,
    input: crate::market::InventorySearchInput,
) -> Result<crate::market::InventorySearchResult, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    crate::market::search_inventory(&conn, &input)
}

#[tauri::command]
pub fn get_market_quote(db: State<'_, Db>, type_id: i64, quantity: i64) -> Result<crate::market::MarketQuote, String> {
    if quantity < 1 {
        return Err("requested market quantity must be at least 1".to_string());
    }
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    crate::market::quote(&conn, type_id, quantity)
}

#[tauri::command]
pub fn get_operation_cost_assumptions(
    db: State<'_, Db>,
    operation_id: i64,
) -> Result<crate::market::CostAssumptionState, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    crate::market::load_cost_assumptions(&conn, operation_id)
}

#[tauri::command]
pub fn save_operation_cost_assumptions(
    db: State<'_, Db>,
    operation_id: i64,
    assumptions: crate::market::CostAssumptions,
) -> Result<crate::market::CostAssumptionState, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    crate::market::save_cost_assumptions(&conn, operation_id, &assumptions)
}

#[tauri::command]
pub fn reset_operation_cost_assumptions(
    db: State<'_, Db>,
    operation_id: i64,
) -> Result<crate::market::CostAssumptionState, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    crate::market::reset_cost_assumptions(&conn, operation_id)
}

#[tauri::command]
pub fn get_operation_economics(
    db: State<'_, Db>,
    operation_id: i64,
    revenue_basis: Option<String>,
) -> Result<crate::market::OperationEconomics, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    crate::market::operation_economics(&conn, operation_id, revenue_basis.as_deref())
}

#[tauri::command]
pub fn get_operation_shopping_list(
    db: State<'_, Db>,
    operation_id: i64,
) -> Result<crate::procurement::OperationShoppingList, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    crate::procurement::shopping_list(&conn, operation_id)
}

#[tauri::command]
pub fn update_procurement_line(
    db: State<'_, Db>,
    operation_id: i64,
    type_id: i64,
    status: String,
    notes: String,
) -> Result<crate::procurement::ProcurementState, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    crate::procurement::save_state(&conn, operation_id, type_id, &status, &notes)
}

#[tauri::command]
pub fn reset_procurement_state(db: State<'_, Db>, operation_id: i64) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|_| "db lock poisoned".to_string())?;
    crate::procurement::reset_state(&conn, operation_id)
}

#[tauri::command]
pub fn sync_character_blueprints(db: State<'_, Db>, client_id: String, character_id: i64) -> Result<blueprint::sync::BlueprintSyncResult, String> {
    let conn=db.conn.lock().map_err(|_|"db lock poisoned".to_string())?;
    blueprint::sync::sync_one(&conn,&client_id,character_id)
}

#[tauri::command]
pub fn sync_all_character_blueprints(db: State<'_, Db>, client_id: String) -> Result<Vec<blueprint::sync::BlueprintSyncResult>, String> {
    let conn=db.conn.lock().map_err(|_|"db lock poisoned".to_string())?;
    blueprint::sync::sync_all(&conn,&client_id)
}
