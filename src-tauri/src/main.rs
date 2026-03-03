#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc;
mod state;

use cimmeria_common::config::ServerConfig;
use cimmeria_services::orchestrator::Orchestrator;
use std::sync::Arc;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tracing::info!("Cimmeria Server starting...");

    let config = ServerConfig::default();
    let orchestrator = Arc::new(Orchestrator::new(config));

    tauri::Builder::default()
        .manage(state::AppState::new(orchestrator))
        .invoke_handler(tauri::generate_handler![
            ipc::get_server_status,
            ipc::get_player_count,
            ipc::get_uptime,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
