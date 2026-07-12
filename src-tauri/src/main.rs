// RenderNorth Industrial — Tauri core process.
// Intelligence, not automation: this process reads local data and official
// APIs only. It never touches the EVE client.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod blueprint;
mod character;
mod commands;
mod db;
mod decision;
mod esi;
mod inventory;
mod models;
mod operation;
mod production;
mod reservation;
mod staticdata;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("could not resolve app data directory");
            let db = db::Db::open(data_dir).map_err(|e| {
                eprintln!("database initialization failed: {e}");
                e
            })?;
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health_check,
            commands::get_mission_control,
            commands::list_build_targets,
            commands::select_build_target,
            commands::get_inventory_summary,
            commands::list_inventory_categories,
            commands::list_inventory_items,
            commands::get_operations_dashboard,
            commands::get_operation_detail,
            commands::get_inventory_commitment,
            commands::get_reservation_summary,
            commands::list_active_reservations,
            commands::get_reservation_detail,
            commands::get_reservation_conflicts,
            commands::get_operation_reservations,
            commands::get_blueprint_summary,
            commands::list_blueprints,
            commands::get_blueprint_detail,
            commands::get_missing_blueprint_report,
            commands::get_blueprint_readiness_all,
            commands::get_blueprint_readiness_for_operation,
            commands::get_requirement_summary,
            commands::list_requirement_lines,
            commands::get_requirement_detail,
            commands::list_requirement_categories,
            commands::get_requirement_shortages,
            commands::get_critical_bottlenecks,
            commands::get_operation_requirement_breakdown,
            commands::get_build_target_requirement_breakdown,
            commands::import_static_data,
            commands::import_official_sde,
            commands::get_latest_import,
            commands::search_eve_types,
            commands::create_real_operation,
            commands::delete_real_operation,
            commands::calculate_production_plan,
            commands::list_manual_inventory,
            commands::add_manual_inventory_entry,
            commands::add_manual_inventory_bulk,
            commands::update_manual_inventory_quantity,
            commands::remove_manual_inventory_entry,
            commands::get_app_setting,
            commands::set_app_setting,
            commands::add_character,
            commands::remove_character,
            commands::set_character_enabled,
            commands::list_characters
        ])
        .run(tauri::generate_context!())
        .expect("error while running RenderNorth Industrial");
}
