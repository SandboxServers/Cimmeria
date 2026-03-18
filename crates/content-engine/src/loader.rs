//! Chain loading from JSON and database row structs.
//!
//! Provides deserialization of chain definitions from JSON (for testing,
//! import/export, and the ServerEd chain editor) and conversion from
//! database row structs into typed `Chain` objects.

use tracing::warn;

use crate::actions::Action;
use crate::chain::Chain;
use crate::conditions::{ComparisonOp, Condition, MissionStatusValue, StepStatusValue};
use crate::triggers::Trigger;

/// Deserialize a list of chains from a JSON string.
pub fn load_chains_from_json(json: &str) -> Result<Vec<Chain>, serde_json::Error> {
    serde_json::from_str(json)
}

// ── Database row structs ──────────────────────────────────────────────────────

/// A row from `content_chains`.
#[derive(Debug)]
pub struct DbChainRow {
    pub chain_id: i32,
    pub description: Option<String>,
    pub scope_type: String,
    pub scope_id: Option<i32>,
    pub enabled: bool,
    pub priority: i32,
}

/// A row from `content_triggers`.
#[derive(Debug)]
pub struct DbTriggerRow {
    pub chain_id: i32,
    pub event_type: String,
    pub event_key: Option<String>,
    pub scope: String,
    pub once: bool,
    pub sort_order: i32,
}

/// A row from `content_conditions`.
#[derive(Debug)]
pub struct DbConditionRow {
    pub chain_id: i32,
    pub condition_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub operator: String,
    pub value: Option<String>,
    pub sort_order: i32,
}

/// A row from `content_actions`.
#[derive(Debug)]
pub struct DbActionRow {
    pub chain_id: i32,
    pub action_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub params: serde_json::Value,
    pub delay_ms: i32,
    pub sort_order: i32,
}

// ── Conversion ────────────────────────────────────────────────────────────────

/// Build a Vec<Chain> from separate vectors of DB rows.
///
/// Chains that have no trigger rows are included (triggerless chains can be
/// invoked directly via `on_victory_chains`).
pub fn build_chains_from_rows(
    chain_rows: Vec<DbChainRow>,
    trigger_rows: Vec<DbTriggerRow>,
    condition_rows: Vec<DbConditionRow>,
    action_rows: Vec<DbActionRow>,
) -> Vec<Chain> {
    use std::collections::HashMap;

    // Group triggers, conditions, actions by chain_id
    let mut triggers_by_chain: HashMap<i32, Vec<DbTriggerRow>> = HashMap::new();
    for row in trigger_rows {
        triggers_by_chain.entry(row.chain_id).or_default().push(row);
    }

    let mut conditions_by_chain: HashMap<i32, Vec<DbConditionRow>> = HashMap::new();
    for row in condition_rows {
        conditions_by_chain.entry(row.chain_id).or_default().push(row);
    }

    let mut actions_by_chain: HashMap<i32, Vec<DbActionRow>> = HashMap::new();
    for row in action_rows {
        actions_by_chain.entry(row.chain_id).or_default().push(row);
    }

    let mut chains = Vec::with_capacity(chain_rows.len());

    for row in chain_rows {
        let chain_id = row.chain_id;
        let name = row.description.unwrap_or_else(|| format!("chain_{}", chain_id));

        // Build trigger — take the first trigger row (chains have 0 or 1 triggers)
        let mut trigger_list = triggers_by_chain.remove(&chain_id).unwrap_or_default();
        trigger_list.sort_by_key(|t| t.sort_order);
        let trigger = if let Some(t_row) = trigger_list.into_iter().next() {
            match convert_trigger(&t_row) {
                Some(t) => t,
                None => {
                    warn!(chain_id, event_type = %t_row.event_type, "Unknown trigger event_type, skipping chain");
                    continue;
                }
            }
        } else {
            // Triggerless chain (invoked by on_victory_chains or TriggerChain)
            // Use a CustomEvent that never naturally fires
            Trigger::OnCustomEvent {
                event_name: format!("__direct_invoke_{}", chain_id),
            }
        };

        // Build conditions
        let mut cond_list = conditions_by_chain.remove(&chain_id).unwrap_or_default();
        cond_list.sort_by_key(|c| c.sort_order);
        let conditions: Vec<Condition> = cond_list.iter().filter_map(|c_row| {
            let result = convert_condition(c_row);
            if result.is_none() {
                warn!(chain_id, condition_type = %c_row.condition_type, "Unknown condition_type, skipping");
            }
            result
        }).collect();

        // Build actions
        let mut act_list = actions_by_chain.remove(&chain_id).unwrap_or_default();
        act_list.sort_by_key(|a| a.sort_order);
        let actions: Vec<Action> = act_list.iter().filter_map(|a_row| {
            let result = convert_action(a_row);
            if result.is_none() {
                warn!(chain_id, action_type = %a_row.action_type, "Unknown action_type, skipping");
            }
            result
        }).collect();

        chains.push(Chain {
            id: chain_id as i64,
            name,
            enabled: row.enabled,
            trigger,
            conditions,
            actions,
            priority: row.priority,
        });
    }

    chains
}

/// Convert a DB trigger row to a Trigger enum variant.
fn convert_trigger(row: &DbTriggerRow) -> Option<Trigger> {
    let key = row.event_key.as_deref();
    match row.event_type.as_str() {
        "player_loaded" => Some(Trigger::OnPlayerLoaded { world_name: key.map(|s| s.to_string()) }),
        "dialog_open" => Some(Trigger::OnDialogOpen {
            dialog_id: key?.parse().ok()?,
        }),
        "dialog_choice" => Some(Trigger::OnDialogChoice {
            dialog_id: key?.parse().ok()?,
        }),
        "enter_region" => Some(Trigger::OnRegionEnter {
            region_key: key?.to_string(),
        }),
        "exit_region" => Some(Trigger::OnRegionExit {
            region_key: key?.to_string(),
        }),
        "interact_tag" => Some(Trigger::OnInteractTag {
            entity_tag: key?.to_string(),
        }),
        "interact_template" => Some(Trigger::OnInteractTemplate {
            template_name: key?.to_string(),
        }),
        "entity_dead_tag" => Some(Trigger::OnEntityDeath {
            entity_type: None,
            entity_tag: Some(key?.to_string()),
        }),
        "item_use" => Some(Trigger::OnItemUse {
            item_id: key?.parse().ok()?,
        }),
        "teleport_in" => Some(Trigger::OnTeleportIn {
            region_id: key?.parse().ok()?,
        }),
        "effect_init" => Some(Trigger::OnEffectInit),
        "effect_pulse_begin" => Some(Trigger::OnEffectPulseBegin),
        "effect_pulse_end" => Some(Trigger::OnEffectPulseEnd),
        "effect_removed" => Some(Trigger::OnEffectRemoved),
        "mission_completed" => Some(Trigger::OnMissionCompleted {
            mission_id: key?.parse().ok()?,
        }),
        "dialog_set_open" => Some(Trigger::OnDialogSetOpen {
            dialog_set_name: key?.to_string(),
        }),
        _ => None,
    }
}

/// Convert a DB condition row to a Condition enum variant.
fn convert_condition(row: &DbConditionRow) -> Option<Condition> {
    let op = parse_comparison_op(&row.operator)?;
    match row.condition_type.as_str() {
        "mission_status" => {
            let mission_id = row.target_id?;
            let status = parse_mission_status(row.value.as_deref()?)?;
            Some(Condition::MissionStatus { mission_id, operator: op, expected_status: status })
        }
        "step_status" => {
            let mission_id = row.target_id?;
            let step_id = row.target_key.as_deref()?.parse().ok()?;
            let status = parse_step_status(row.value.as_deref()?)?;
            Some(Condition::StepStatus { mission_id, step_id, operator: op, expected_status: status })
        }
        "archetype" => {
            let archetype_id = row.value.as_deref()?.parse().ok()?;
            Some(Condition::Archetype { operator: op, archetype_id })
        }
        "objective_status" => {
            let mission_id = row.target_id?;
            let objective_id = row.target_key.as_deref()?.parse().ok()?;
            let expected = row.value.as_deref()?.to_string();
            Some(Condition::ObjectiveStatus { mission_id, objective_id, operator: op, expected_status: expected })
        }
        "counter" => {
            let counter_name = row.target_key.as_deref()?.to_string();
            let value = row.value.as_deref()?.parse().ok()?;
            Some(Condition::Counter { counter_name, operator: op, value })
        }
        _ => None,
    }
}

/// Convert a DB action row to an Action enum variant.
fn convert_action(row: &DbActionRow) -> Option<Action> {
    let params = &row.params;
    match row.action_type.as_str() {
        "accept_mission" => Some(Action::AcceptMission { mission_id: row.target_id? }),
        "complete_mission" => Some(Action::CompleteMission { mission_id: row.target_id? }),
        "display_dialog" => Some(Action::DisplayDialog { dialog_id: row.target_id? }),
        "add_dialog_set" => {
            let dialog_set_id = row.target_id?;
            let slot = params.get("slot").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let mission_id = params.get("mission_id").and_then(|v| v.as_i64()).map(|v| v as i32);
            Some(Action::AddDialogSet { dialog_set_id, slot, mission_id })
        }
        "remove_dialog_set" => {
            let dialog_set_id = row.target_id?;
            let slot = params.get("slot").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            Some(Action::RemoveDialogSet { dialog_set_id, slot })
        }
        "add_item" => {
            let item_id = row.target_id?;
            let qty = params.get("qty").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            let container = params.get("container").and_then(|v| v.as_i64()).map(|v| v as i32);
            Some(Action::GrantItem { item_id, count: qty, container_id: container })
        }
        "remove_item" => {
            let item_id = row.target_id?;
            let qty = params.get("qty").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            Some(Action::RemoveItem { item_id, count: qty })
        }
        "play_sequence" => Some(Action::PlaySequence { sequence_id: row.target_id? }),
        "advance_step" => {
            let mission_id = row.target_id?;
            let step_id = row.target_key.as_deref()?.parse().ok()?;
            Some(Action::AdvanceStep { mission_id, step_id })
        }
        "set_interaction_type" => {
            let entity_tag = row.target_key.as_deref()?.to_string();
            let operation = params.get("op").and_then(|v| v.as_str()).unwrap_or("|").to_string();
            let mask = params.get("mask").and_then(|v| v.as_i64()).unwrap_or(0);
            Some(Action::SetInteractionType { entity_tag, operation, mask })
        }
        "start_minigame" => {
            let minigame_type = row.target_key.as_deref().unwrap_or("").to_string();
            let chains = params.get("on_victory_chains")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
                .unwrap_or_default();
            Some(Action::StartMinigame { minigame_type, on_victory_chains: chains })
        }
        "set_aggression" => {
            let entity_tag = row.target_key.as_deref()?.to_string();
            let level = params.get("level").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            Some(Action::SetAggression { entity_tag, level })
        }
        "destroy_entity" => {
            let entity_tag = row.target_key.as_deref()?.to_string();
            Some(Action::DestroyTaggedEntity { entity_tag })
        }
        "trigger_transporter" => {
            let region_id = params.get("regionId").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            Some(Action::TriggerTransporter { region_id })
        }
        "system_message" => Some(Action::SystemMessage { message_id: row.target_id? }),
        "qr_combat_damage" => {
            let stat_id = params.get("stat_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let source_id = params.get("source_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let amount_nvp = params.get("amount_nvp").and_then(|v| v.as_str()).unwrap_or("").to_string();
            Some(Action::QrCombatDamage { stat_id, source_id, amount_nvp })
        }
        "change_stat" => {
            let stat_id = params.get("stat_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let min = params.get("min").and_then(|v| v.as_i64()).map(|v| v as i32);
            let max = params.get("max").and_then(|v| v.as_i64()).map(|v| v as i32);
            let use_ammo_stat = params.get("use_ammo_stat").and_then(|v| v.as_bool());
            let set_to_max = params.get("set_to_max").and_then(|v| v.as_bool());
            Some(Action::ChangeStat { stat_id, min, max, use_ammo_stat, set_to_max })
        }
        "apply_effect" => {
            let effect_id = row.target_id?;
            Some(Action::ApplyEffect { effect_id, duration_secs: None })
        }
        "remove_effect" => Some(Action::RemoveEffect { effect_id: row.target_id? }),
        "abandon_mission" => Some(Action::AbandonMission { mission_id: row.target_id? }),
        "fail_objective" => {
            let mission_id = row.target_id?;
            let objective_id = row.target_key.as_deref()?.parse().ok()?;
            Some(Action::FailObjective { mission_id, objective_id })
        }
        "increment_counter" => {
            let counter_name = row.target_key.as_deref()?.to_string();
            let amount = params.get("amount").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            Some(Action::IncrementCounter { counter_name, amount })
        }
        "reset_counter" => {
            let counter_name = row.target_key.as_deref()?.to_string();
            Some(Action::ResetCounter { counter_name })
        }
        "complete_objective" => {
            let mission_id = row.target_id?;
            let objective_id = row.target_key.as_deref()?.parse().ok()?;
            Some(Action::CompleteObjective { mission_id, objective_id })
        }
        "set_visible" => {
            let entity_tag = row.target_key.as_deref()?.to_string();
            let visible = params.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
            Some(Action::SetVisible { entity_tag, visible })
        }
        "move_entity" => {
            let entity_tag = row.target_key.as_deref().map(|s| s.to_string());
            let dest_str = params.get("destination").and_then(|v| v.as_str()).unwrap_or("0,0,0");
            let destination = parse_destination(dest_str);
            let world = params.get("world").and_then(|v| v.as_str()).map(|s| s.to_string());
            let use_player = params.get("use_player").and_then(|v| v.as_bool());
            Some(Action::MoveEntity { entity_tag, destination, world, use_player })
        }
        _ => None,
    }
}

/// Parse a "x,y,z" destination string into [f32; 3].
fn parse_destination(s: &str) -> [f32; 3] {
    let parts: Vec<f32> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    if parts.len() >= 3 {
        [parts[0], parts[1], parts[2]]
    } else {
        [0.0, 0.0, 0.0]
    }
}

fn parse_comparison_op(s: &str) -> Option<ComparisonOp> {
    match s {
        "eq" => Some(ComparisonOp::Eq),
        "neq" => Some(ComparisonOp::Neq),
        "gte" => Some(ComparisonOp::Gte),
        "lte" => Some(ComparisonOp::Lte),
        "gt" => Some(ComparisonOp::Gt),
        "lt" => Some(ComparisonOp::Lt),
        _ => None,
    }
}

fn parse_mission_status(s: &str) -> Option<MissionStatusValue> {
    match s {
        "not_active" => Some(MissionStatusValue::NotActive),
        "active" => Some(MissionStatusValue::Active),
        "completed" => Some(MissionStatusValue::Completed),
        _ => None,
    }
}

fn parse_step_status(s: &str) -> Option<StepStatusValue> {
    match s {
        "not_active" => Some(StepStatusValue::NotActive),
        "active" => Some(StepStatusValue::Active),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_empty_array() {
        let chains = load_chains_from_json("[]").unwrap();
        assert!(chains.is_empty());
    }

    #[test]
    fn load_single_chain() {
        let json = r#"[{
            "id": 1,
            "name": "Grant XP on mob kill",
            "enabled": true,
            "trigger": {
                "OnEntityDeath": { "entity_type": "SGWMob" }
            },
            "conditions": [],
            "actions": [
                { "GrantXP": { "amount": 100 } }
            ],
            "priority": 10
        }]"#;

        let chains = load_chains_from_json(json).unwrap();
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].id, 1);
        assert_eq!(chains[0].name, "Grant XP on mob kill");
    }

    #[test]
    fn load_invalid_json_returns_error() {
        let result = load_chains_from_json("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn build_chains_from_db_rows() {
        let chain_rows = vec![DbChainRow {
            chain_id: 1001,
            description: Some("622 - Zone load: accept mission".to_string()),
            scope_type: "mission".to_string(),
            scope_id: Some(622),
            enabled: true,
            priority: 0,
        }];
        let trigger_rows = vec![DbTriggerRow {
            chain_id: 1001,
            event_type: "player_loaded".to_string(),
            event_key: None,
            scope: "player".to_string(),
            once: false,
            sort_order: 0,
        }];
        let condition_rows = vec![DbConditionRow {
            chain_id: 1001,
            condition_type: "mission_status".to_string(),
            target_id: Some(622),
            target_key: None,
            operator: "eq".to_string(),
            value: Some("not_active".to_string()),
            sort_order: 0,
        }];
        let action_rows = vec![
            DbActionRow {
                chain_id: 1001,
                action_type: "accept_mission".to_string(),
                target_id: Some(622),
                target_key: None,
                params: serde_json::json!({}),
                delay_ms: 0,
                sort_order: 0,
            },
            DbActionRow {
                chain_id: 1001,
                action_type: "display_dialog".to_string(),
                target_id: Some(2982),
                target_key: None,
                params: serde_json::json!({}),
                delay_ms: 0,
                sort_order: 1,
            },
        ];

        let chains = build_chains_from_rows(chain_rows, trigger_rows, condition_rows, action_rows);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].id, 1001);
        assert_eq!(chains[0].conditions.len(), 1);
        assert_eq!(chains[0].actions.len(), 2);

        match &chains[0].actions[0] {
            Action::AcceptMission { mission_id } => assert_eq!(*mission_id, 622),
            other => panic!("Expected AcceptMission, got {:?}", other),
        }
    }

    #[test]
    fn triggerless_chain_gets_custom_event() {
        let chain_rows = vec![DbChainRow {
            chain_id: 1017,
            description: Some("Victory callback".to_string()),
            scope_type: "mission".to_string(),
            scope_id: Some(638),
            enabled: true,
            priority: 0,
        }];
        let action_rows = vec![DbActionRow {
            chain_id: 1017,
            action_type: "advance_step".to_string(),
            target_id: Some(638),
            target_key: Some("2116".to_string()),
            params: serde_json::json!({}),
            delay_ms: 0,
            sort_order: 0,
        }];

        let chains = build_chains_from_rows(chain_rows, vec![], vec![], action_rows);
        assert_eq!(chains.len(), 1);
        match &chains[0].trigger {
            Trigger::OnCustomEvent { event_name } => {
                assert!(event_name.contains("1017"));
            }
            other => panic!("Expected OnCustomEvent, got {:?}", other),
        }
    }

    #[test]
    fn parse_destination_valid() {
        assert_eq!(parse_destination("-123.625,1.311,-246.858"), [-123.625, 1.311, -246.858]);
    }

    #[test]
    fn parse_destination_invalid() {
        assert_eq!(parse_destination("bad"), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn convert_interact_tag_trigger() {
        let row = DbTriggerRow {
            chain_id: 1,
            event_type: "interact_tag".to_string(),
            event_key: Some("ArmYourself_FrostBody".to_string()),
            scope: "player".to_string(),
            once: false,
            sort_order: 0,
        };
        let trigger = convert_trigger(&row).unwrap();
        match trigger {
            Trigger::OnInteractTag { entity_tag } => assert_eq!(entity_tag, "ArmYourself_FrostBody"),
            other => panic!("Expected OnInteractTag, got {:?}", other),
        }
    }

    #[test]
    fn convert_counter_condition() {
        let row = DbConditionRow {
            chain_id: 1,
            condition_type: "counter".to_string(),
            target_id: None,
            target_key: Some("hallway01_kills".to_string()),
            operator: "gte".to_string(),
            value: Some("3".to_string()),
            sort_order: 0,
        };
        let condition = convert_condition(&row).unwrap();
        match condition {
            Condition::Counter { counter_name, operator, value } => {
                assert_eq!(counter_name, "hallway01_kills");
                assert_eq!(operator, ComparisonOp::Gte);
                assert_eq!(value, 3);
            }
            other => panic!("Expected Counter, got {:?}", other),
        }
    }
}
