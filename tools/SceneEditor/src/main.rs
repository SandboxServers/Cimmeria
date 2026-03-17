#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use state::AppState;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("debug,tao=info,wry=info,tauri=info")
        .init();

    tracing::info!("Cimmeria Scene Editor starting...");

    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::scan_zones,
            commands::load_zone,
            commands::list_actors,
            commands::get_actor_details,
            commands::pick_actor,
            commands::load_mesh,
            commands::list_mesh_refs,
            commands::delete_actors,
            commands::duplicate_actors,
            commands::move_actors,
            commands::rotate_actors,
            commands::scale_actors,
            commands::undo,
            commands::redo,
            commands::export_sql,
            commands::save_zone,
            // Procedural placement commands
            commands::place_grid,
            commands::place_circle,
            commands::place_spiral,
            // Asset browsing commands
            commands::scan_packages,
            commands::list_packages,
            commands::list_package_exports,
            commands::search_assets,
            commands::get_texture_thumbnail,
            // Database commands
            commands::connect_db,
            commands::disconnect_db,
            commands::db_status,
            commands::list_worlds,
            commands::load_db_entities,
            commands::list_entity_templates,
            commands::create_spawn,
            commands::update_spawn_position,
            commands::delete_spawn,
            commands::create_region,
            commands::delete_region,
            commands::delete_db_entity,
            commands::export_db_entities_sql,
        ])
        .run(tauri::generate_context!())
        .expect("error while running scene editor");
}
