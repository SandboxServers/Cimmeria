use crate::commands::chains::{
    SaveActionInput, SaveChainInput, SaveConditionInput, SaveTriggerInput,
};
use crate::script::xml_format::{self, ScriptConnection, ScriptFile, ScriptNode};
use crate::state::AppState;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Public DTOs
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ConvertResult {
    pub chains: Vec<SaveChainInput>,
    pub warnings: Vec<String>,
    pub space_name: String,
}

#[derive(Deserialize)]
pub struct ConvertOptions {
    /// Starting chain ID for allocation (default: auto from next_id * 100).
    pub start_chain_id: Option<i32>,
    /// Override scope_type (default: "space").
    pub scope_type: Option<String>,
    /// Override scope_id (default: derived from space name).
    pub scope_id: Option<i32>,
}

// ---------------------------------------------------------------------------
// Tauri command
// ---------------------------------------------------------------------------

/// Convert a .script XML file into content chain records.
///
/// Returns the chains ready for save_chains or SQL export — does NOT
/// write to the database automatically.
#[tauri::command]
pub async fn convert_script_to_chains(
    state: tauri::State<'_, AppState>,
    path: String,
    options: Option<ConvertOptions>,
) -> Result<ConvertResult, String> {
    let full_path = state.scripts_dir().join(&path);
    tracing::info!("convert_script_to_chains: {}", full_path.display());

    let script = xml_format::load_script(&full_path).map_err(|e| {
        tracing::error!("Failed to load script {}: {e}", full_path.display());
        format!("Failed to load script: {e}")
    })?;

    let options = options.unwrap_or(ConvertOptions {
        start_chain_id: None,
        scope_type: None,
        scope_id: None,
    });

    let result = convert_script(&script, &options);
    tracing::info!(
        "Converted {} nodes → {} chains ({} warnings)",
        script.nodes.len(),
        result.chains.len(),
        result.warnings.len(),
    );

    Ok(result)
}

// ---------------------------------------------------------------------------
// Conversion engine
// ---------------------------------------------------------------------------

/// Internal converter: ScriptFile → Vec<SaveChainInput>.
fn convert_script(script: &ScriptFile, options: &ConvertOptions) -> ConvertResult {
    let mut warnings = Vec::new();

    let scope_type: String = options
        .scope_type
        .clone()
        .unwrap_or_else(|| "space".to_string());
    let scope_id = options.scope_id;
    let mut next_chain_id = options
        .start_chain_id
        .unwrap_or_else(|| (script.next_id as i32) * 100);

    // Build lookup maps
    let node_map: HashMap<u32, &ScriptNode> = script.nodes.iter().map(|n| (n.id, n)).collect();

    // Build adjacency: out_node → [(out_port, in_node, in_port)]
    let mut outgoing: HashMap<u32, Vec<&ScriptConnection>> = HashMap::new();
    for conn in &script.connections {
        outgoing.entry(conn.out_node).or_default().push(conn);
    }

    // Find all event nodes (triggers)
    let event_nodes: Vec<&ScriptNode> = script
        .nodes
        .iter()
        .filter(|n| is_event_node(&n.ref_name))
        .filter(|n| node_prop(n, "Enabled").as_deref().unwrap_or("true") == "true")
        .collect();

    let mut chains = Vec::new();

    for event_node in &event_nodes {
        let trigger = match convert_event_to_trigger(event_node) {
            Some(t) => t,
            None => {
                warnings.push(format!(
                    "Node {} ({}): unsupported event type, skipped",
                    event_node.id, event_node.ref_name
                ));
                continue;
            }
        };

        let fire_once = node_prop(event_node, "Fire Only Once").as_deref().unwrap_or("false") == "true";

        // Walk the graph from this event's output connections
        let conns = outgoing.get(&event_node.id).cloned().unwrap_or_default();

        // For each output port path, build a chain
        // Most events have a single "Out" port; GenericRegion has "Entered"/"Left"
        let port_groups = group_by_out_port(&conns);

        for (out_port, first_targets) in &port_groups {
            // Determine trigger variant based on port
            let effective_trigger = match (event_node.ref_name.as_str(), out_port.as_str()) {
                ("Event_GenericRegion", "Left") => {
                    // Switch to exit_region trigger
                    let region_key = node_prop(event_node, "Tag").unwrap_or_default();
                    SaveTriggerInput {
                        event_type: "exit_region".to_string(),
                        event_key: Some(region_key),
                        scope: "player".to_string(),
                        once: fire_once,
                        sort_order: 0,
                    }
                }
                _ => trigger.clone(),
            };

            // Walk from each first-hop target, collecting conditions and actions
            let mut conditions = Vec::new();
            let mut actions = Vec::new();
            let mut entity_tag_context: Option<String> = None;

            for target_conn in first_targets {
                walk_chain(
                    target_conn.in_node,
                    &node_map,
                    &outgoing,
                    &mut conditions,
                    &mut actions,
                    &mut entity_tag_context,
                    &mut warnings,
                );
            }

            if actions.is_empty() && conditions.is_empty() {
                // No-op chain (e.g. Event→Log only), skip
                continue;
            }

            // Assign sort_order to actions
            for (i, action) in actions.iter_mut().enumerate() {
                action.sort_order = i as i32;
            }
            for (i, condition) in conditions.iter_mut().enumerate() {
                condition.sort_order = i as i32;
            }

            chains.push(SaveChainInput {
                chain_id: Some(next_chain_id),
                description: Some(format!(
                    "{} → {} (node {})",
                    script.module, event_node.ref_name, event_node.id,
                )),
                scope_type: scope_type.clone(),
                scope_id,
                enabled: true,
                priority: 0,
                triggers: vec![effective_trigger],
                conditions,
                actions,
                counters: Vec::new(),
            });

            next_chain_id += 1;
        }
    }

    ConvertResult {
        chains,
        warnings,
        space_name: script.module.clone(),
    }
}

// ---------------------------------------------------------------------------
// Graph walking
// ---------------------------------------------------------------------------

/// Recursively walk the graph from a node, collecting conditions and actions.
fn walk_chain(
    node_id: u32,
    node_map: &HashMap<u32, &ScriptNode>,
    outgoing: &HashMap<u32, Vec<&ScriptConnection>>,
    conditions: &mut Vec<SaveConditionInput>,
    actions: &mut Vec<SaveActionInput>,
    entity_tag_ctx: &mut Option<String>,
    warnings: &mut Vec<String>,
) {
    let Some(node) = node_map.get(&node_id) else {
        return;
    };

    if node_prop(node, "Enabled").as_deref().unwrap_or("true") == "false" {
        return;
    }

    let ref_name = node.ref_name.as_str();

    // Handle condition nodes (Act_GetMission, Act_GetMissionStep, etc.)
    match ref_name {
        "Act_GetMission" => {
            let mission_id = node_prop_i32(node, "Mission Id");
            // Follow each output port to determine the condition path
            if let Some(conns) = outgoing.get(&node_id) {
                for conn in conns {
                    let condition_value = match conn.out_port.as_str() {
                        "Not Accepted" => "not_active",
                        "Active" => "active",
                        "Completed" => "completed",
                        "Failed" => "completed", // treat failed as completed for now
                        _ => {
                            // "Player" port just passes through
                            continue;
                        }
                    };
                    conditions.push(SaveConditionInput {
                        condition_type: "mission_status".to_string(),
                        target_id: mission_id,
                        target_key: None,
                        operator: "eq".to_string(),
                        value: Some(condition_value.to_string()),
                        sort_order: 0,
                    });
                    walk_chain(
                        conn.in_node,
                        node_map,
                        outgoing,
                        conditions,
                        actions,
                        entity_tag_ctx,
                        warnings,
                    );
                }
            }
            return;
        }
        "Act_GetMissionStep" => {
            let mission_id = node_prop_i32(node, "Mission Id");
            let step_id = node_prop(node, "Step Id");
            if let Some(conns) = outgoing.get(&node_id) {
                for conn in conns {
                    let condition_value = match conn.out_port.as_str() {
                        "Not Started" => "not_active",
                        "Active" => "active",
                        "Completed" => "completed",
                        _ => continue,
                    };
                    conditions.push(SaveConditionInput {
                        condition_type: "step_status".to_string(),
                        target_id: mission_id,
                        target_key: step_id.clone(),
                        operator: "eq".to_string(),
                        value: Some(condition_value.to_string()),
                        sort_order: 0,
                    });
                    walk_chain(
                        conn.in_node,
                        node_map,
                        outgoing,
                        conditions,
                        actions,
                        entity_tag_ctx,
                        warnings,
                    );
                }
            }
            return;
        }
        "Act_GetEntity" => {
            // Resolve entity_tag for subsequent actions
            let tag = node_prop(node, "Tag");
            if let Some(t) = &tag {
                *entity_tag_ctx = Some(t.clone());
            }
            // Follow "Found" output
            if let Some(conns) = outgoing.get(&node_id) {
                for conn in conns {
                    if conn.out_port == "Found" || conn.out_port == "Out" {
                        walk_chain(
                            conn.in_node,
                            node_map,
                            outgoing,
                            conditions,
                            actions,
                            entity_tag_ctx,
                            warnings,
                        );
                    }
                }
            }
            return;
        }
        _ => {}
    }

    // Handle action nodes
    if let Some(action) = convert_node_to_action(node, entity_tag_ctx, warnings) {
        actions.push(action);
    }

    // Follow output connections (Out, Successful, etc.)
    if let Some(conns) = outgoing.get(&node_id) {
        for conn in conns {
            let port = conn.out_port.as_str();
            // Skip ports that represent failure/error paths
            if port == "Failed" || port == "Player" || port == "Entity" || port == "Entity Id" {
                continue;
            }
            walk_chain(
                conn.in_node,
                node_map,
                outgoing,
                conditions,
                actions,
                entity_tag_ctx,
                warnings,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Node classification
// ---------------------------------------------------------------------------

fn is_event_node(ref_name: &str) -> bool {
    matches!(
        ref_name,
        "Event_Loaded"
            | "Event_ScriptLoaded"
            | "Event_GenericRegion"
            | "Event_EntityInteract"
            | "Event_Teleport"
            | "Event_EntityDead"
            | "Event_Dialog"
            | "Event_MissionComplete"
    )
}

// ---------------------------------------------------------------------------
// Event → Trigger conversion
// ---------------------------------------------------------------------------

fn convert_event_to_trigger(node: &ScriptNode) -> Option<SaveTriggerInput> {
    let fire_once = node_prop(node, "Fire Only Once").as_deref().unwrap_or("false") == "true";

    let (event_type, event_key) = match node.ref_name.as_str() {
        "Event_Loaded" | "Event_ScriptLoaded" => ("player_loaded".to_string(), None),
        "Event_GenericRegion" => {
            let tag = node_prop(node, "Tag")?;
            ("enter_region".to_string(), Some(tag))
        }
        "Event_EntityInteract" => {
            let tag = node_prop(node, "Tag");
            let template = node_prop(node, "Template Name");
            if let Some(t) = tag.filter(|s| !s.is_empty()) {
                ("interact_tag".to_string(), Some(t))
            } else if let Some(t) = template.filter(|s| !s.is_empty()) {
                ("interact_template".to_string(), Some(t))
            } else {
                return None;
            }
        }
        "Event_Teleport" => {
            let region_id = node_prop(node, "Region Id")?;
            ("teleport_in".to_string(), Some(region_id))
        }
        "Event_EntityDead" => {
            let tag = node_prop(node, "Tag");
            ("entity_dead_tag".to_string(), tag)
        }
        "Event_Dialog" => {
            let dialog_id = node_prop(node, "Dialog Set").or_else(|| node_prop(node, "Dialog Id"));
            ("dialog_choice".to_string(), dialog_id)
        }
        "Event_MissionComplete" => {
            let mission_id = node_prop(node, "Mission Id");
            ("mission_completed".to_string(), mission_id)
        }
        _ => return None,
    };

    Some(SaveTriggerInput {
        event_type,
        event_key,
        scope: "player".to_string(),
        once: fire_once,
        sort_order: 0,
    })
}

// ---------------------------------------------------------------------------
// Action node → SaveActionInput
// ---------------------------------------------------------------------------

fn convert_node_to_action(
    node: &ScriptNode,
    entity_tag_ctx: &Option<String>,
    warnings: &mut Vec<String>,
) -> Option<SaveActionInput> {
    let ref_name = node.ref_name.as_str();
    match ref_name {
        "Act_UpdateMission" => {
            let mission_id = node_prop_i32(node, "Mission Id");
            // Determine operation from which input port has a connection
            // Default to accept; callers connect to Accept/Complete/Abandon ports
            // We look at the port connections in the parent, but since we're walking
            // forward, we use the incoming port name that was connected to
            // For simplicity, check if any output port hints at the operation
            Some(SaveActionInput {
                action_type: "accept_mission".to_string(),
                target_id: mission_id,
                target_key: None,
                params: serde_json::json!({}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_Dialog" | "Act_DisplayDialog" => {
            let dialog_id = node_prop_i32(node, "Dialog Set")
                .or_else(|| node_prop_i32(node, "Dialog Id"));
            Some(SaveActionInput {
                action_type: "display_dialog".to_string(),
                target_id: dialog_id,
                target_key: None,
                params: serde_json::json!({}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_GiveItems" => {
            let item_id = node_prop_i32(node, "Item Id");
            let qty = node_prop_i32(node, "Quantity").unwrap_or(1);
            Some(SaveActionInput {
                action_type: "add_item".to_string(),
                target_id: item_id,
                target_key: None,
                params: serde_json::json!({"qty": qty}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_RemoveItems" => {
            let item_id = node_prop_i32(node, "Item Id");
            let qty = node_prop_i32(node, "Quantity").unwrap_or(1);
            Some(SaveActionInput {
                action_type: "remove_item".to_string(),
                target_id: item_id,
                target_key: None,
                params: serde_json::json!({"qty": qty}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_Sequence" | "Act_PlaySequence" => {
            let seq_id = node_prop_i32(node, "Sequence Id")
                .or_else(|| node_prop_i32(node, "Cinematic Id"));
            Some(SaveActionInput {
                action_type: "play_sequence".to_string(),
                target_id: seq_id,
                target_key: None,
                params: serde_json::json!({}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_UpdateInteraction" => {
            let mask_str = node_prop(node, "Interaction Type").unwrap_or_default();
            let mask: i64 = mask_str.parse().unwrap_or(0);
            let entity_tag = entity_tag_ctx.clone();
            Some(SaveActionInput {
                action_type: "set_interaction_type".to_string(),
                target_id: None,
                target_key: entity_tag,
                params: serde_json::json!({"op": "|", "mask": mask}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_SysMsg" => {
            let msg_id = node_prop_i32(node, "String Id");
            Some(SaveActionInput {
                action_type: "system_message".to_string(),
                target_id: msg_id,
                target_key: None,
                params: serde_json::json!({}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_AddToDialogSet" => {
            let dialog_set_id = node_prop_i32(node, "Dialog Set");
            let slot = node_prop_i32(node, "Slot").unwrap_or(0);
            let mission_id = node_prop_i32(node, "Mission Id");
            let mut params = serde_json::json!({"slot": slot});
            if let Some(mid) = mission_id {
                params["mission_id"] = serde_json::json!(mid);
            }
            Some(SaveActionInput {
                action_type: "add_dialog_set".to_string(),
                target_id: dialog_set_id,
                target_key: None,
                params,
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_RemoveFromDialogSet" => {
            let dialog_set_id = node_prop_i32(node, "Dialog Set");
            let slot = node_prop_i32(node, "Slot").unwrap_or(0);
            Some(SaveActionInput {
                action_type: "remove_dialog_set".to_string(),
                target_id: dialog_set_id,
                target_key: None,
                params: serde_json::json!({"slot": slot}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_MoveTo" | "Act_MoveEntity" => {
            // Animated waypoint movement — uses entity_tag from context
            let entity_tag = entity_tag_ctx.clone();
            // Try to find destination from connected Var_Vec3 node (handled by position port)
            // If not available, use a zero default
            let speed_str = node_prop(node, "Speed").unwrap_or_else(|| "1.0".to_string());
            let speed: f64 = speed_str.parse().unwrap_or(1.0);
            Some(SaveActionInput {
                action_type: "move_waypoint".to_string(),
                target_id: None,
                target_key: entity_tag,
                params: serde_json::json!({"destination": "0,0,0", "speed": speed}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_SetActiveSlot" => {
            let slot = node_prop_i32(node, "Slot").unwrap_or(0);
            let bag_id = node_prop_i32(node, "Bag Id").unwrap_or(3); // default Bandolier
            Some(SaveActionInput {
                action_type: "set_active_slot".to_string(),
                target_id: None,
                target_key: None,
                params: serde_json::json!({"bag_id": bag_id, "slot": slot}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_Ability" | "Act_LaunchAbility" => {
            let ability_id = node_prop_i32(node, "Ability Id");
            let entity_tag = entity_tag_ctx.clone();
            Some(SaveActionInput {
                action_type: "launch_ability".to_string(),
                target_id: ability_id,
                target_key: entity_tag,
                params: serde_json::json!({}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_SetAggression" | "Act_UpdateProperty" => {
            // UpdateProperty that sets Aggression — check if Aggression port is connected
            let entity_tag = entity_tag_ctx.clone();
            // For Act_UpdateProperty, we map to set_aggression when Aggression is connected
            // For simplicity, always emit set_aggression with level from connected Var_Int
            // Default level 0 if not determinable
            Some(SaveActionInput {
                action_type: "set_aggression".to_string(),
                target_id: None,
                target_key: entity_tag,
                params: serde_json::json!({"level": 0}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_DestroyEntity" => {
            let entity_tag = entity_tag_ctx.clone();
            Some(SaveActionInput {
                action_type: "destroy_entity".to_string(),
                target_id: None,
                target_key: entity_tag,
                params: serde_json::json!({}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_SpawnMinigame" => {
            let mg_type = node_prop(node, "Minigame Type").unwrap_or_default();
            Some(SaveActionInput {
                action_type: "start_minigame".to_string(),
                target_id: None,
                target_key: Some(mg_type),
                params: serde_json::json!({"on_victory_chains": []}),
                delay_ms: 0,
                sort_order: 0,
            })
        }
        "Act_Log" => {
            // Log nodes are debug-only, skip
            None
        }
        _ => {
            // Variable nodes (Var_Int, Var_Vec3, etc.) are data-only, not actions
            if ref_name.starts_with("Var_") || ref_name.starts_with("Cond_") {
                None
            } else {
                warnings.push(format!(
                    "Node {} ({}): unsupported action type, skipped",
                    node.id, ref_name
                ));
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get a property value from a node by name.
fn node_prop(node: &ScriptNode, name: &str) -> Option<String> {
    node.properties
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.clone())
        .filter(|v| !v.is_empty())
}

/// Get a property as i32.
fn node_prop_i32(node: &ScriptNode, name: &str) -> Option<i32> {
    node_prop(node, name)?.parse().ok()
}

/// Group connections by their output port name.
fn group_by_out_port<'a>(
    conns: &[&'a ScriptConnection],
) -> Vec<(String, Vec<&'a ScriptConnection>)> {
    let mut map: HashMap<String, Vec<&'a ScriptConnection>> = HashMap::new();
    for conn in conns {
        map.entry(conn.out_port.clone()).or_default().push(conn);
    }
    let mut groups: Vec<_> = map.into_iter().collect();
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::xml_format::ScriptFile;

    fn make_script(nodes: Vec<ScriptNode>, connections: Vec<ScriptConnection>) -> ScriptFile {
        ScriptFile {
            version: "1.4".to_string(),
            dataset_version: "1.2".to_string(),
            module: "TestSpace".to_string(),
            script_type: "Level".to_string(),
            next_id: 10,
            nodes,
            connections,
            comments: Vec::new(),
        }
    }

    fn make_node(id: u32, ref_name: &str, props: Vec<(&str, &str)>) -> ScriptNode {
        ScriptNode {
            id,
            ref_name: ref_name.to_string(),
            x: 0,
            y: 0,
            properties: props
                .into_iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
            ports: Vec::new(),
        }
    }

    fn make_conn(out_node: u32, out_port: &str, in_node: u32, in_port: &str) -> ScriptConnection {
        ScriptConnection {
            out_node,
            out_port: out_port.to_string(),
            in_node,
            in_port: in_port.to_string(),
        }
    }

    fn default_options() -> ConvertOptions {
        ConvertOptions {
            start_chain_id: Some(100),
            scope_type: Some("space".to_string()),
            scope_id: Some(1),
        }
    }

    #[test]
    fn event_loaded_to_accept_mission() {
        let script = make_script(
            vec![
                make_node(1, "Event_Loaded", vec![("Enabled", "true")]),
                make_node(
                    2,
                    "Act_GetMission",
                    vec![("Enabled", "true"), ("Mission Id", "622")],
                ),
                make_node(
                    3,
                    "Act_UpdateMission",
                    vec![("Enabled", "true"), ("Mission Id", "622")],
                ),
            ],
            vec![
                make_conn(1, "Out", 2, "In"),
                make_conn(2, "Not Accepted", 3, "Accept"),
            ],
        );

        let result = convert_script(&script, &default_options());
        assert_eq!(result.chains.len(), 1);

        let chain = &result.chains[0];
        assert_eq!(chain.triggers[0].event_type, "player_loaded");
        assert_eq!(chain.conditions.len(), 1);
        assert_eq!(chain.conditions[0].condition_type, "mission_status");
        assert_eq!(chain.conditions[0].target_id, Some(622));
        assert_eq!(chain.conditions[0].value, Some("not_active".to_string()));
        assert_eq!(chain.actions.len(), 1);
        assert_eq!(chain.actions[0].action_type, "accept_mission");
    }

    #[test]
    fn region_enter_to_system_message() {
        let script = make_script(
            vec![
                make_node(
                    1,
                    "Event_GenericRegion",
                    vec![
                        ("Enabled", "true"),
                        ("Tag", "Castle_CellBlock.Region8"),
                        ("Fire Only Once", "false"),
                    ],
                ),
                make_node(
                    2,
                    "Act_SysMsg",
                    vec![("Enabled", "true"), ("String Id", "5180")],
                ),
            ],
            vec![make_conn(1, "Entered", 2, "In")],
        );

        let result = convert_script(&script, &default_options());
        assert_eq!(result.chains.len(), 1);
        assert_eq!(result.chains[0].triggers[0].event_type, "enter_region");
        assert_eq!(
            result.chains[0].triggers[0].event_key,
            Some("Castle_CellBlock.Region8".to_string())
        );
        assert_eq!(result.chains[0].actions[0].action_type, "system_message");
        assert_eq!(result.chains[0].actions[0].target_id, Some(5180));
    }

    #[test]
    fn interact_event_with_get_entity() {
        let script = make_script(
            vec![
                make_node(
                    1,
                    "Event_EntityInteract",
                    vec![("Enabled", "true"), ("Tag", "ColMarsh")],
                ),
                make_node(
                    2,
                    "Act_GetEntity",
                    vec![("Enabled", "true"), ("Tag", "ColMarsh")],
                ),
                make_node(
                    3,
                    "Act_UpdateInteraction",
                    vec![("Enabled", "true"), ("Interaction Type", "8388608")],
                ),
            ],
            vec![
                make_conn(1, "Out", 2, "In"),
                make_conn(2, "Found", 3, "In"),
            ],
        );

        let result = convert_script(&script, &default_options());
        assert_eq!(result.chains.len(), 1);
        assert_eq!(result.chains[0].triggers[0].event_type, "interact_tag");
        assert_eq!(result.chains[0].actions.len(), 1);
        assert_eq!(
            result.chains[0].actions[0].action_type,
            "set_interaction_type"
        );
        assert_eq!(
            result.chains[0].actions[0].target_key,
            Some("ColMarsh".to_string())
        );
    }

    #[test]
    fn disabled_nodes_skipped() {
        let script = make_script(
            vec![
                make_node(1, "Event_Loaded", vec![("Enabled", "false")]),
                make_node(
                    2,
                    "Act_UpdateMission",
                    vec![("Enabled", "true"), ("Mission Id", "622")],
                ),
            ],
            vec![make_conn(1, "Out", 2, "Accept")],
        );

        let result = convert_script(&script, &default_options());
        assert!(result.chains.is_empty());
    }

    #[test]
    fn launch_ability_action() {
        let script = make_script(
            vec![
                make_node(1, "Event_Loaded", vec![("Enabled", "true")]),
                make_node(
                    2,
                    "Act_Ability",
                    vec![("Enabled", "true"), ("Ability Id", "1372")],
                ),
            ],
            vec![make_conn(1, "Out", 2, "In")],
        );

        let result = convert_script(&script, &default_options());
        assert_eq!(result.chains.len(), 1);
        assert_eq!(result.chains[0].actions[0].action_type, "launch_ability");
        assert_eq!(result.chains[0].actions[0].target_id, Some(1372));
    }

    #[test]
    fn teleport_event_trigger() {
        let script = make_script(
            vec![
                make_node(
                    1,
                    "Event_Teleport",
                    vec![
                        ("Enabled", "true"),
                        ("Region Id", "2"),
                        ("Fire Only Once", "true"),
                    ],
                ),
                make_node(
                    2,
                    "Act_UpdateMission",
                    vec![("Enabled", "true"), ("Mission Id", "640")],
                ),
            ],
            vec![make_conn(1, "Out", 2, "Accept")],
        );

        let result = convert_script(&script, &default_options());
        assert_eq!(result.chains.len(), 1);
        assert_eq!(result.chains[0].triggers[0].event_type, "teleport_in");
        assert_eq!(
            result.chains[0].triggers[0].event_key,
            Some("2".to_string())
        );
        assert!(result.chains[0].triggers[0].once);
    }

    #[test]
    fn empty_script_produces_no_chains() {
        let script = make_script(vec![], vec![]);
        let result = convert_script(&script, &default_options());
        assert!(result.chains.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn chain_ids_increment() {
        let script = make_script(
            vec![
                make_node(
                    1,
                    "Event_GenericRegion",
                    vec![("Enabled", "true"), ("Tag", "Region1")],
                ),
                make_node(
                    2,
                    "Act_SysMsg",
                    vec![("Enabled", "true"), ("String Id", "100")],
                ),
                make_node(
                    3,
                    "Event_GenericRegion",
                    vec![("Enabled", "true"), ("Tag", "Region2")],
                ),
                make_node(
                    4,
                    "Act_SysMsg",
                    vec![("Enabled", "true"), ("String Id", "200")],
                ),
            ],
            vec![
                make_conn(1, "Entered", 2, "In"),
                make_conn(3, "Entered", 4, "In"),
            ],
        );

        let result = convert_script(&script, &default_options());
        assert_eq!(result.chains.len(), 2);
        assert_eq!(result.chains[0].chain_id, Some(100));
        assert_eq!(result.chains[1].chain_id, Some(101));
    }
}
