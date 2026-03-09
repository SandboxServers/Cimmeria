#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod script;
mod state;

use state::AppState;

fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    tracing::info!("Cimmeria Content Editor starting...");

    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::connect_db,
            commands::get_connection_status,
            // Content chains
            commands::list_spaces,
            commands::load_chains,
            commands::save_chains,
            commands::delete_chain,
            // Seed export & hot reload
            commands::export_to_seed_file,
            commands::hot_reload,
            // Visual scripts
            commands::list_scripts,
            commands::load_script,
            commands::save_script,
            commands::compile_script,
            commands::load_node_templates,
            commands::load_enumerations,
            // Entity templates
            commands::list_entity_templates,
            commands::get_entity_template,
            commands::save_entity_template,
            commands::delete_entity_template,
            // Items
            commands::list_items,
            commands::get_item,
            commands::save_item,
            commands::delete_item,
            // Missions
            commands::list_missions,
            commands::get_mission,
            commands::save_mission,
            commands::delete_mission,
            // Loot tables
            commands::list_loot_tables,
            commands::get_loot_table,
            commands::save_loot_table,
            commands::delete_loot_table,
            // Abilities
            commands::list_abilities,
            commands::get_ability,
            commands::save_ability,
            commands::delete_ability,
        ])
        .run(tauri::generate_context!())
        .expect("error while running content editor");
}
