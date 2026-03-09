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
            commands::list_spaces,
            commands::load_chains,
            commands::save_chains,
            commands::delete_chain,
            commands::export_to_seed_file,
            commands::hot_reload,
            commands::list_scripts,
            commands::load_script,
            commands::save_script,
            commands::compile_script,
            commands::load_node_templates,
            commands::load_enumerations,
        ])
        .run(tauri::generate_context!())
        .expect("error while running content editor");
}
