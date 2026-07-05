// RenderNorth Industrial — Tauri core process.
// Intelligence, not automation: this process reads local data and official
// APIs only. It never touches the EVE client.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod decision;
mod inventory;
mod models;
mod operation;

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
            commands::list_inventory_items
        ])
        .run(tauri::generate_context!())
        .expect("error while running RenderNorth Industrial");
}
