//! `convert_action` — DB row → `Action` enum variant — and the
//! `parse_destination` helper used to read `"x,y,z"` action params.

use tracing::warn;

use crate::actions::Action;

use super::DbActionRow;

/// Convert a DB action row to an Action enum variant.
pub(super) fn convert_action(row: &DbActionRow) -> Option<Action> {
    let params = &row.params;
    match row.action_type.as_str() {
        "accept_mission" => Some(Action::AcceptMission {
            mission_id: row.target_id?,
        }),
        "complete_mission" => Some(Action::CompleteMission {
            mission_id: row.target_id?,
        }),
        "display_dialog" => Some(Action::DisplayDialog {
            dialog_id: row.target_id?,
        }),
        "add_dialog_set" => {
            let dialog_set_id = row.target_id?;
            let slot = params.get("slot").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let mission_id = params
                .get("mission_id")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);
            Some(Action::AddDialogSet {
                dialog_set_id,
                slot,
                mission_id,
            })
        }
        "remove_dialog_set" => {
            let dialog_set_id = row.target_id?;
            let slot = params.get("slot").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            Some(Action::RemoveDialogSet {
                dialog_set_id,
                slot,
            })
        }
        "add_item" => {
            let item_id = row.target_id?;
            let qty = params.get("qty").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            let container = params
                .get("container")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);
            Some(Action::GrantItem {
                item_id,
                count: qty,
                container_id: container,
            })
        }
        "remove_item" => {
            let item_id = row.target_id?;
            let qty = params.get("qty").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            Some(Action::RemoveItem {
                item_id,
                count: qty,
            })
        }
        "play_sequence" => Some(Action::PlaySequence {
            sequence_id: row.target_id?,
        }),
        "advance_step" => {
            let mission_id = row.target_id?;
            let step_id = row.target_key.as_deref()?.parse().ok()?;
            Some(Action::AdvanceStep {
                mission_id,
                step_id,
            })
        }
        "set_interaction_type" => {
            let entity_tag = row.target_key.as_deref()?.to_string();
            let operation = params
                .get("op")
                .and_then(|v| v.as_str())
                .unwrap_or("|")
                .to_string();
            // Accept either an integer literal (legacy form, e.g.,
            // `'mask': 256`) or a symbolic name from EInteractionNotification
            // Type (`'mask': 'INT_MinigameLivewire'`). The symbolic form is
            // preferred — see crates/entity/src/interaction_flags.rs.
            let mask = match params.get("mask") {
                Some(v) if v.is_i64() => v.as_i64().unwrap_or(0),
                Some(v) if v.is_string() => {
                    let name = v.as_str().unwrap_or("");
                    match cimmeria_entity::interaction_flags::mask_for_name(name) {
                        Some(m) => m,
                        None => {
                            tracing::warn!(
                                chain_id = row.chain_id, %name,
                                "set_interaction_type: unknown interaction-flag name; mask defaulted to 0"
                            );
                            0
                        }
                    }
                }
                _ => 0,
            };
            Some(Action::SetInteractionType {
                entity_tag,
                operation,
                mask,
            })
        }
        "start_minigame" => {
            let minigame_type = row.target_key.as_deref().unwrap_or("").to_string();
            let chains = params
                .get("on_victory_chains")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
                .unwrap_or_default();
            Some(Action::StartMinigame {
                minigame_type,
                on_victory_chains: chains,
            })
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
        "system_message" => Some(Action::SystemMessage {
            message_id: row.target_id?,
        }),
        "qr_combat_damage" => {
            let stat_id = params.get("stat_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let source_id = params
                .get("source_id")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let amount_nvp = params
                .get("amount_nvp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(Action::QrCombatDamage {
                stat_id,
                source_id,
                amount_nvp,
            })
        }
        "change_stat" => {
            let stat_id = params.get("stat_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let min = params.get("min").and_then(|v| v.as_i64()).map(|v| v as i32);
            let max = params.get("max").and_then(|v| v.as_i64()).map(|v| v as i32);
            let use_ammo_stat = params.get("use_ammo_stat").and_then(|v| v.as_bool());
            let set_to_max = params.get("set_to_max").and_then(|v| v.as_bool());
            // Reject out-of-i32-range `amount` values at this system
            // boundary rather than silently wrapping via `as i32`. The
            // delta is applied to the stat with `Stat::change`, so a
            // wrapped value would heal/damage by a wildly different
            // amount than the seed author intended. Drop the whole
            // action on the bad row — better to no-op than to apply
            // the wrong delta.
            let amount = match params.get("amount").and_then(|v| v.as_i64()) {
                None => None,
                Some(v) => match i32::try_from(v) {
                    Ok(v) => Some(v),
                    Err(_) => {
                        warn!(
                            chain_id = row.chain_id,
                            amount = v,
                            "change_stat.amount is out of i32 range; \
                             dropping action"
                        );
                        return None;
                    }
                },
            };
            Some(Action::ChangeStat {
                stat_id,
                min,
                max,
                use_ammo_stat,
                set_to_max,
                amount,
            })
        }
        "apply_effect" => {
            let effect_id = row.target_id?;
            Some(Action::ApplyEffect {
                effect_id,
                duration_secs: None,
            })
        }
        "remove_effect" => Some(Action::RemoveEffect {
            effect_id: row.target_id?,
        }),
        "abandon_mission" => Some(Action::AbandonMission {
            mission_id: row.target_id?,
        }),
        "fail_objective" => {
            let mission_id = row.target_id?;
            let objective_id = row.target_key.as_deref()?.parse().ok()?;
            Some(Action::FailObjective {
                mission_id,
                objective_id,
            })
        }
        "increment_counter" => {
            let counter_name = row.target_key.as_deref()?.to_string();
            let amount = params.get("amount").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            Some(Action::IncrementCounter {
                counter_name,
                amount,
            })
        }
        "reset_counter" => {
            let counter_name = row.target_key.as_deref()?.to_string();
            Some(Action::ResetCounter { counter_name })
        }
        "complete_objective" => {
            let mission_id = row.target_id?;
            let objective_id = row.target_key.as_deref()?.parse().ok()?;
            Some(Action::CompleteObjective {
                mission_id,
                objective_id,
            })
        }
        "set_visible" => {
            let entity_tag = row.target_key.as_deref()?.to_string();
            let visible = params
                .get("visible")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            Some(Action::SetVisible {
                entity_tag,
                visible,
            })
        }
        "move_entity" => {
            let entity_tag = row.target_key.as_deref().map(|s| s.to_string());
            let dest_str = params
                .get("destination")
                .and_then(|v| v.as_str())
                .unwrap_or("0,0,0");
            let destination = parse_destination(dest_str);
            let world = params
                .get("world")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let use_player = params.get("use_player").and_then(|v| v.as_bool());
            Some(Action::MoveEntity {
                entity_tag,
                destination,
                world,
                use_player,
            })
        }
        "move_waypoint" => {
            let entity_tag = row.target_key.as_deref()?.to_string();
            let dest_str = params
                .get("destination")
                .and_then(|v| v.as_str())
                .unwrap_or("0,0,0");
            let destination = parse_destination(dest_str);
            let speed = params.get("speed").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            Some(Action::MoveWaypoint {
                entity_tag,
                destination,
                speed,
            })
        }
        "set_active_slot" => {
            let bag_id = params.get("bag_id").and_then(|v| v.as_i64()).unwrap_or(3) as i32;
            let slot = params.get("slot").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            Some(Action::SetActiveSlot { bag_id, slot })
        }
        "launch_ability" => {
            let ability_id = row.target_id?;
            let entity_tag = row.target_key.as_deref().map(|s| s.to_string());
            Some(Action::LaunchAbility {
                ability_id,
                entity_tag,
            })
        }
        "add_dialog" => {
            let dialog_set_id = row.target_id?;
            let entity_template = params
                .get("entity_template")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);
            let mission_id = params
                .get("mission_id")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);
            Some(Action::AddDialog {
                dialog_set_id,
                entity_template,
                mission_id,
            })
        }
        "generate_threat" => {
            let entity_tag = row.target_key.as_deref().map(|s| s.to_string());
            let threat_level = params
                .get("threat_level")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            Some(Action::GenerateThreat {
                entity_tag,
                threat_level,
            })
        }
        _ => None,
    }
}

/// Parse a "x,y,z" destination string into [f32; 3].
pub(super) fn parse_destination(s: &str) -> [f32; 3] {
    let parts: Vec<f32> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    if parts.len() >= 3 {
        [parts[0], parts[1], parts[2]]
    } else {
        [0.0, 0.0, 0.0]
    }
}
