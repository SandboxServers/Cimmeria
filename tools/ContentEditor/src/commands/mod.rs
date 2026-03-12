mod chains;
mod convert;
mod data;
mod hot_reload;
mod scripts;
mod seed_export;

pub use chains::*;
pub use convert::*;
pub use data::*;
pub use hot_reload::*;
pub use scripts::*;
pub use seed_export::*;

use crate::state::AppState;
use serde::Serialize;

#[derive(Serialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub database: Option<String>,
}

#[tauri::command]
pub async fn connect_db(
    state: tauri::State<'_, AppState>,
    connection_string: String,
    server_url: Option<String>,
) -> Result<ConnectionStatus, String> {
    tracing::info!("connect_db called");
    if let Some(url) = server_url {
        tracing::debug!("Setting server URL to {url}");
        state.set_server_url(url).await;
    }

    state.connect(&connection_string).await?;
    tracing::info!("connect_db succeeded");

    Ok(ConnectionStatus {
        connected: true,
        database: Some(connection_string),
    })
}

#[tauri::command]
pub async fn get_connection_status(
    state: tauri::State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    let connected = state.pool().is_ok();
    Ok(ConnectionStatus {
        connected,
        database: None,
    })
}
