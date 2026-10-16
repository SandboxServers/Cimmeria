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
                "args": c.args.iter().map(|a| json!({
                    "name": a.name,
                    "kind": a.kind,
                    "required": a.required,
                    "default": null,
                    "range": (a.min.is_some() || a.max.is_some()).then(|| json!({
                        "min": a.min,
                        "max": a.max,
                    })),
                    "choices": a.choices,
                    "help": a.help,
                    "sensitive": false,
                })).collect::<Vec<_>>(),
                "forms": c.forms,
                "preconditions": c.preconditions,
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
                "impact": (!c.impact_kind.is_empty()).then(|| json!({
                    "kind": c.impact_kind,
                    "summary": c.impact,
                    "preconditions": [c.preconditions],
                })),
                "verify": (!c.verify_tools.is_empty()).then(|| json!({
                    "read_tools": c.verify_tools,
                    "comparison_fields": c.comparison_fields,
                    "recipe": c.verify,
                })),
                "compensation": match c.compensation_kind {
                    "manual" => json!({"kind": "manual", "value": c.compensation}),
                    "none" => json!({"kind": "none"}),
                    _ => Value::Null,
                },
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
            assert_eq!(row["availability"], command.availability);
            assert_eq!(row["manual_pending"], command.manual_pending);
            if command.name == "despawn" {
                assert_eq!(row["target"]["argument_name"], "entityId");
            }
        }
    }

    #[test]
    fn reviewed_travel_and_spawn_contracts_reach_catalog() {
        let catalog = list_commands_for_actor(true).unwrap();
        let rows = catalog["commands"].as_array().unwrap();
        let family = [
            "gotoxyz",
            "goto",
            "summon",
            "gotolocation",
            "gotospace",
            "location",
            "rotation",
            "movehere",
            "spawn",
            "despawn",
            "savespawn",
            "delspawn",
            "autosavespawn",
            "respawnall",
            "spawnset",
            "spawnrandom",
        ];
        let runnable: Vec<_> = rows
            .iter()
            .filter(|row| {
                family.contains(&row["name"].as_str().unwrap()) && row["availability"] == "runnable"
            })
            .map(|row| row["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            runnable,
            [".gotolocation", ".spawn", ".despawn", ".delspawn"]
        );

        let xyz = rows.iter().find(|r| r["name"] == "gotoxyz").unwrap();
        assert_eq!(xyz["availability"], "unsupported");
        assert_eq!(xyz["manual_pending"], true);
        assert!(xyz["disposition_reason"]
            .as_str()
            .unwrap()
            .contains("execution-time expected-subject"));
        assert_eq!(xyz["target"]["kind"], "none");
        assert_eq!(xyz["target"]["selection_rule"], "none");
        assert_eq!(
            xyz["args"]
                .as_array()
                .unwrap()
                .iter()
                .map(|arg| json!([arg["name"], arg["kind"], arg["required"]]))
                .collect::<Vec<_>>(),
            vec![
                json!(["x", "float", true]),
                json!(["y", "float", true]),
                json!(["z", "float", true])
            ]
        );
        assert_eq!(xyz["impact"]["kind"], "persistent");
        assert!(xyz["impact"]["summary"]
            .as_str()
            .unwrap()
            .contains("BASEMSG_FORCED_POSITION"));
        assert_eq!(
            xyz["verify"]["read_tools"],
            json!(["server_entity_get", "server_log_tail", "server_db_query"])
        );
        assert!(xyz["verify"]["recipe"]
            .as_str()
            .unwrap()
            .contains("sgw_player"));

        let expected = [
            (
                "gotolocation",
                "none",
                "none",
                "persistent",
                json!([
                    ["worldName", "world", true],
                    ["x", "float", false],
                    ["y", "float", false],
                    ["z", "float", false]
                ]),
                json!([
                    "server_entity_get",
                    "server_sessions",
                    "server_log_tail",
                    "server_db_query"
                ]),
            ),
            (
                "spawn",
                "none",
                "none",
                "transient",
                json!([["templateId", "template", true]]),
                json!(["GM completion feedback", "server_entity_get"]),
            ),
            (
                "despawn",
                "mob_or_id",
                "selected_or_argument",
                "transient",
                json!([["entityId", "entity", false]]),
                json!(["server_entity_get"]),
            ),
            (
                "delspawn",
                "spawnable",
                "required",
                "authoring",
                json!([]),
                json!(["server_entity_get", "server_db_query", ".seedpending"]),
            ),
        ];
        for (name, target_kind, selection, impact, args, tools) in expected {
            let row = rows.iter().find(|row| row["name"] == name).unwrap();
            assert_eq!(row["availability"], "runnable", ".{name}");
            assert_eq!(row["target"]["kind"], target_kind, ".{name}");
            assert_eq!(row["target"]["selection_rule"], selection, ".{name}");
            assert_eq!(row["impact"]["kind"], impact, ".{name}");
            let fields: Vec<_> = row["args"]
                .as_array()
                .unwrap()
                .iter()
                .map(|arg| json!([arg["name"], arg["kind"], arg["required"]]))
                .collect();
            assert_eq!(Value::Array(fields), args, ".{name}");
            assert_eq!(row["verify"]["read_tools"], tools, ".{name}");
            assert!(
                !row["verify"]["recipe"].as_str().unwrap().is_empty(),
                ".{name}"
            );
        }
        let location = rows.iter().find(|r| r["name"] == "gotolocation").unwrap();
        assert!(location["preconditions"]
            .as_str()
            .unwrap()
            .contains("no navmesh or Z containment validation"));
        assert!(location["impact"]["summary"]
            .as_str()
            .unwrap()
            .contains("outside traversable or contained geometry"));
        let recipe = location["verify"]["recipe"].as_str().unwrap();
        assert!(recipe.contains("wire.out.forced_position send=sent"));
        assert!(recipe.contains("rows_affected_zero, persist_arrival_failed"));
        assert!(recipe.contains("evidence is unknown"));
        let despawn = rows.iter().find(|r| r["name"] == "despawn").unwrap();
        assert!(despawn["forms"].as_str().unwrap().contains("including pet"));
        assert!(despawn["impact"]["summary"]
            .as_str()
            .unwrap()
            .contains("without its normal dismiss path"));
        let unsafe_spawn = rows.iter().find(|r| r["name"] == "spawnrandom").unwrap();
        assert_eq!(unsafe_spawn["availability"], "unsupported");
        assert_eq!(unsafe_spawn["manual_pending"], true);
        assert!(unsafe_spawn["disposition_reason"]
            .as_str()
            .unwrap()
            .contains("Server"));
    }
}
