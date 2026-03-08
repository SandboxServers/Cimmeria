#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod content;
mod drafts;
mod ipc;
mod state;

use cimmeria_common::config::ServerConfig;
use cimmeria_services::orchestrator::Orchestrator;
use std::sync::Arc;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    tracing::info!("Cimmeria Server starting...");

    let config = ServerConfig::default();
    let db_connection_string = config.db_connection_string.clone();
    let orchestrator = Arc::new(Orchestrator::new(config));

    tauri::Builder::default()
        .manage(state::AppState::new(orchestrator, db_connection_string))
        .invoke_handler(tauri::generate_handler![
            ipc::get_server_status,
            ipc::get_player_count,
            ipc::get_uptime,
            content::load_chain_editor_content,
            content::save_chain_editor_content,
            content::validate_chain_editor_content,
            drafts::load_chain_editor_draft,
            drafts::save_chain_editor_draft,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
