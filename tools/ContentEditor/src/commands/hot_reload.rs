use crate::state::AppState;

#[tauri::command]
pub async fn hot_reload(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let url = state.server_url().await;
    let reload_url = format!("{url}/api/content/reload");

    let client = reqwest::Client::new();
    let response = client
        .post(&reload_url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Failed to reach server at {reload_url}: {e}"))?;

    if response.status().is_success() {
        tracing::info!("Hot reload triggered successfully");
        Ok("Content engine reloaded".to_string())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Hot reload failed ({status}): {body}"))
    }
}
