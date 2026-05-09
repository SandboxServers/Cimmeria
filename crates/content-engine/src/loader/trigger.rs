//! `convert_trigger` — DB row → `Trigger` enum variant.

use crate::triggers::Trigger;

use super::DbTriggerRow;

/// Convert a DB trigger row to a Trigger enum variant.
pub(super) fn convert_trigger(row: &DbTriggerRow) -> Option<Trigger> {
    let key = row.event_key.as_deref();
    match row.event_type.as_str() {
        "player_loaded" => Some(Trigger::OnPlayerLoaded {
            world_name: key.map(|s| s.to_string()),
        }),
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
        // `item_equipped` accepts a wildcard (`event_key = NULL`) that matches
        // any equipped item, OR a specific design id (`event_key = "55"`).
        // A non-empty `event_key` that fails to parse must reject the chain
        // entirely — silently collapsing `Some("bad")` into `None` would turn
        // a typo'd integer into a wildcard that fires for every equip.
        "item_equipped" => match key {
            None => Some(Trigger::OnItemEquipped { item_id: None }),
            Some(s) => Some(Trigger::OnItemEquipped {
                item_id: Some(s.parse().ok()?),
            }),
        },
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
        "mission_accepted" => Some(Trigger::OnMissionAccepted {
            mission_id: key?.parse().ok()?,
        }),
        _ => None,
    }
}
