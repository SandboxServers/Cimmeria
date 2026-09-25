//! `server_console_list` + `server_console_exec` tool logic.
//!
//! Kept free of the rmcp macro surface so it can be reasoned about (and, for
//! the exec path, tested at the cell-loop level in `cimmeria-services`)
//! independently of the transport. The rmcp adapter in [`super`] is a thin
//! wrapper over these.

use serde_json::{json, Value};
use tokio::sync::oneshot;

use cimmeria_services::cell::console::command_catalog;
use cimmeria_services::cell::messages::BaseToCellMsg;

use crate::state::LabState;

/// Enumerate the `.`-console command registry.
pub fn list_commands() -> Value {
    let cmds = command_catalog();
    let commands: Vec<Value> = cmds
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "min_args": c.min_args,
                "max_args": c.max_args,
                "target": c.target,
                "help": c.help,
            })
        })
        .collect();
    json!({ "count": commands.len(), "commands": commands })
}

/// Run a `.`-console line as `entity_id` via `BaseToCellMsg::LabConsoleExec`
/// and return the captured feedback output. The GM access-level gate is
/// enforced cell-side against the acting entity.
pub async fn exec_console(state: &LabState, entity_id: u32, line: String) -> Result<Value, String> {
    let Some(tx) = state.cell_tx().await else {
        return Err("cell service channel not available".to_string());
    };
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(BaseToCellMsg::LabConsoleExec {
        entity_id,
        line,
        reply_tx,
    })
    .await
    .map_err(|e| format!("failed to send to cell service: {e}"))?;

    match reply_rx.await {
        Ok(Ok(output)) => Ok(json!({ "entity_id": entity_id, "output": output })),
        // Cell-side denial (non-GM / unknown entity) — surface as an error so
        // the audit outcome is `error`.
        Ok(Err(reason)) => Err(reason),
        Err(_) => Err("cell service dropped the reply (shutting down?)".to_string()),
    }
}
