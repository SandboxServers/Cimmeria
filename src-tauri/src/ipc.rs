use crate::state::AppState;

#[tauri::command]
pub async fn get_server_status(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok("running".to_string())
}

#[tauri::command]
pub async fn get_player_count(state: tauri::State<'_, AppState>) -> Result<u32, String> {
    Ok(0)
}

#[tauri::command]
pub async fn get_uptime(state: tauri::State<'_, AppState>) -> Result<u64, String> {
    let uptime = state.orchestrator.uptime().await;
    Ok(uptime.as_secs())
}
