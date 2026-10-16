//! `server_console_list` + `server_console_exec` tool logic.
//!
//! Kept free of the rmcp macro surface so it can be reasoned about (and, for
//! the exec path, tested at the cell-loop level in `cimmeria-services`)
//! independently of the transport. The rmcp adapter in [`super`] is a thin
//! wrapper over these.

use serde_json::{json, Value};
use tokio::sync::oneshot;

use cimmeria_services::cell::console::{command_catalog, is_gm};
use cimmeria_services::cell::messages::{BaseToCellMsg, LabQuery, LabQueryReply};

use crate::state::LabState;

/// Enumerate the dot registry only after a fresh cell-side actor snapshot.
/// Missing, disconnected and non-GM entities fail closed.
pub async fn list_commands(state: &LabState, actor_entity_id: u32) -> Result<Value, String> {
    let reply = state
        .lab_query(LabQuery::EntityGet {
            entity_id: actor_entity_id,
        })
        .await?;
    let allowed = match reply {
        LabQueryReply::Entity {
            entity: Some(actor),
        } => actor.entity_id == actor_entity_id && actor.is_player && is_gm(actor.access_level),
        LabQueryReply::Entity { entity: None } => false,
        other => return Err(format!("unexpected LabQuery reply variant: {other:?}")),
    };
    list_commands_for_actor(allowed).map_err(str::to_string)
}

/// Catalog entry point for a GM-scoped workbench API. The caller supplies a
/// freshly checked server authority result, never a browser-provided flag.
fn list_commands_for_actor(server_is_gm: bool) -> Result<Value, &'static str> {
    if !server_is_gm {
        return Err("not_gm");
    }
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
                "id": format!(".{}", c.name),
                "channel": c.channel,
                "category": c.category,
                "aliases": [],
                "summary": c.help,
                "example": null,
                "availability": c.availability,
                "manual_pending": c.manual_pending,
                "disposition_reason": c.disposition_reason,
                "args": [],
                "target_policy": c.target_policy,
                "target": {
                    "kind": c.target_policy,
                    "selection_rule": match c.target_policy {
                        "none" => "none",
                        "mob_or_id" => "selected_or_argument",
                        _ => "required",
                    },
                    "argument_name": c.target_argument_name,
                },
                "impact": null,
                "verify": null,
                "compensation": null,
                "metadata_version": c.metadata_version,
            })
        })
        .collect();
    Ok(
        json!({ "count": commands.len(), "commands": commands, "slash_catalog_status": "pending_source_review" }),
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workbench_catalog_refuses_non_gm() {
        assert_eq!(list_commands_for_actor(false).unwrap_err(), "not_gm");
    }

    #[test]
    fn dot_catalog_matches_registry_and_exposes_pending_metadata() {
        let expected = command_catalog();
        let catalog = list_commands_for_actor(true).unwrap();
        let rows = catalog["commands"].as_array().unwrap();
        assert_eq!(rows.len(), expected.len());
        for (row, command) in rows.iter().zip(expected) {
            assert_eq!(row["name"], command.name);
            assert_eq!(row["id"], format!(".{}", command.name));
            assert_eq!(row["channel"], "dot");
            assert_eq!(row["availability"], "unsupported");
            assert_eq!(row["manual_pending"], true);
            if command.name == "despawn" {
                assert_eq!(row["target"]["argument_name"], "entityId");
            }
        }
    }
}
