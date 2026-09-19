//! `server_content_reload` tool logic — trigger a content-engine hot reload.

use serde_json::{json, Value};

use cimmeria_services::cell::messages::BaseToCellMsg;

use crate::state::LabState;

/// Send `BaseToCellMsg::ReloadContentEngine` to the cell service, which
/// re-reads all content chains from the database. Fire-and-forget: the reload
/// runs on the cell loop; there is no completion reply on this message.
pub async fn reload_content(state: &LabState) -> Result<Value, String> {
    let Some(tx) = state.cell_tx().await else {
        return Err("cell service channel not available".to_string());
    };
    tx.send(BaseToCellMsg::ReloadContentEngine)
        .await
        .map_err(|e| format!("failed to send reload to cell service: {e}"))?;
    Ok(json!({ "status": "content reload triggered" }))
}
