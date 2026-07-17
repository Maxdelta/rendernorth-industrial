// RenderNorth Industrial — Tauri core process.
// Intelligence, not automation: this process reads local data and official
// APIs only. It never touches the EVE client.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod blueprint;
mod character;
mod commands;
mod corporation;
mod db;
mod decision;
mod esi;
mod inventory;
mod location;
mod market;
mod onboarding;
mod volume;
mod models;
mod operation;
mod production;
mod procurement;
mod quartermaster;
mod reservation;
mod release;
mod staticdata;
mod update;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            commands::list_owned_blueprint_candidates,
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
            commands::list_characters,
            commands::sync_character_assets,
            commands::sync_all_character_assets,
            commands::sync_corporation_assets,
            commands::sync_all_corporation_assets,
            commands::list_synced_assets,
            commands::refresh_asset_locations,
            commands::get_market_profile,
            commands::refresh_market_prices,
            commands::search_inventory_market,
            commands::get_market_quote,
            commands::get_operation_cost_assumptions,
            commands::save_operation_cost_assumptions,
            commands::reset_operation_cost_assumptions,
            commands::get_operation_economics,
            commands::get_operation_shopping_list,
            commands::update_procurement_line,
            commands::reset_procurement_state,
            commands::list_doctrines,
            commands::create_doctrine,
            commands::update_doctrine,
            commands::list_doctrine_fits,
            commands::import_doctrine_eft_text,
            commands::import_doctrine_eft_file,
            commands::import_doctrine_eft_folder,
            commands::set_doctrine_fit_quantity,
            commands::delete_doctrine_fit,
            commands::analyze_doctrine,
            commands::sync_character_blueprints,
            commands::sync_all_character_blueprints
            ,commands::inspect_sde_directory
            ,commands::pick_sde_directory
            ,commands::pick_sde_archive
            ,commands::open_selected_folder
            ,commands::get_about_info
            ,commands::get_authentication_info
            ,commands::save_custom_authentication
            ,commands::restore_official_authentication
            ,commands::export_diagnostics
            ,commands::open_external_url
            ,commands::set_application_section
            ,commands::get_release_view_state
            ,commands::mark_current_release_viewed
            ,commands::get_update_state
            ,commands::get_update_preferences
            ,commands::save_update_preferences
            ,commands::check_for_updates
            ,commands::remind_update_later
            ,commands::skip_update_version
            ,commands::clear_skipped_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running RenderNorth Industrial");
}
