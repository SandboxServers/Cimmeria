//! `convert_trigger` — DB row → `Trigger` enum variant.

use std::ops::RangeInclusive;

use crate::triggers::Trigger;

use super::DbTriggerRow;

/// Thresholds an `entity_health_below` trigger can actually fire on.
///
/// Bounded by the matcher's strict downward-crossing test, not by the
/// percentage domain — see the arm in [`convert_trigger`]. Public so the
/// matcher's own documentation and the loader tests read the same source.
pub const HEALTH_PCT_RANGE: RangeInclusive<i32> = 1..=99;

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
        // Stargate events (CA10). `event_key` is the destination world
        // name from `resources.worlds.world` (e.g. `"Harset"`); NULL is
        // the wildcard "any destination". Unlike the integer-keyed
        // triggers there is nothing to reject on a typo — a world name
        // that doesn't exist simply never matches.
        "stargate_dialed" => Some(Trigger::OnStargateDialed {
            destination_world: key.map(|s| s.to_string()),
        }),
        "stargate_crossed" => Some(Trigger::OnStargateCrossed {
            destination_world: key.map(|s| s.to_string()),
        }),
        "effect_init" => Some(Trigger::OnEffectInit),
        "effect_pulse_begin" => Some(Trigger::OnEffectPulseBegin),
        "effect_pulse_end" => Some(Trigger::OnEffectPulseEnd),
        "effect_removed" => Some(Trigger::OnEffectRemoved),
        "mission_completed" => Some(Trigger::OnMissionCompleted {
            mission_id: key?.parse().ok()?,
        }),
        // `event_key` is the mission id, same shape as `mission_completed`
        // and `mission_accepted`. No wildcard: a chain that reacted to *any*
        // abandon would have nothing to repaint.
        "mission_abandoned" => Some(Trigger::OnMissionAbandoned {
            mission_id: key?.parse().ok()?,
        }),
        "dialog_set_open" => Some(Trigger::OnDialogSetOpen {
            dialog_set_name: key?.to_string(),
        }),
        "mission_accepted" => Some(Trigger::OnMissionAccepted {
            mission_id: key?.parse().ok()?,
        }),
        // Cover-system triggers. `event_key` is the
        // optional `cover_set_id` filter — `NULL` means "any cover
        // set". Same wildcard-or-id pattern as `item_equipped` so a
        // typo'd integer rejects the chain rather than silently
        // collapsing to wildcard.
        "player_entered_cover" => match key {
            None => Some(Trigger::OnPlayerEnteredCover { cover_set_id: None }),
            Some(s) => Some(Trigger::OnPlayerEnteredCover {
                cover_set_id: Some(s.parse().ok()?),
            }),
        },
        "player_left_cover" => match key {
            None => Some(Trigger::OnPlayerLeftCover { cover_set_id: None }),
            Some(s) => Some(Trigger::OnPlayerLeftCover {
                cover_set_id: Some(s.parse().ok()?),
            }),
        },
        // `player_in_cover_duration` requires both a duration (in
        // `event_key` as seconds) and an optional cover_set_id, but a
        // single string can't carry both. Convention: `event_key`
        // = `"<seconds>"` (no set filter) or `"<seconds>:<set_id>"`.
        "player_in_cover_duration" => {
            let s = key?;
            let (secs_str, set_str) = match s.split_once(':') {
                Some((a, b)) => (a, Some(b)),
                None => (s, None),
            };
            Some(Trigger::OnPlayerInCoverDuration {
                seconds: secs_str.parse().ok()?,
                cover_set_id: match set_str {
                    Some(s) => Some(s.parse().ok()?),
                    None => None,
                },
            })
        }
        // `entity_health_below` needs a tag *and* a percentage in one
        // `event_key`. Convention: `"<tag>:<pct>"`, e.g.
        // `"Rinla_Malac:30"`.
        //
        // NOTE the field order is the reverse of
        // `player_in_cover_duration` above (`"<seconds>:<set_id>"`,
        // number first): here the numeric field is **last**, and it is
        // parsed with `rsplit_once` so a tag that itself contains a
        // colon still resolves.
        //
        // Every malformed shape rejects the chain rather than degrading:
        // no key, no colon, an empty tag, a non-integer percentage, or a
        // percentage outside [`HEALTH_PCT_RANGE`].
        //
        // The range is `1..=99`, not `1..=100`. The matcher
        // (`triggers::matching`) is a strict downward *crossing* test —
        // `pct_before > threshold && pct_after <= threshold` — so a
        // threshold of 100 can never match: a full-health entity is at
        // 100, which is not strictly greater than 100, and any hit that
        // damages it starts from below. `:100` would load a chain that
        // looks wired and never runs. `:0` is dead for the mirror-image
        // reason — an entity at 0% is dead and routes to
        // `entity_dead_tag`.
        "entity_health_below" => {
            let (tag, pct_str) = key?.rsplit_once(':')?;
            if tag.is_empty() {
                return None;
            }
            let pct: i32 = pct_str.parse().ok()?;
            if !HEALTH_PCT_RANGE.contains(&pct) {
                // Distinct from the loader's generic "unknown event_type"
                // warn: the event_type *is* known and the row is
                // well-formed, so that message would send the author
                // looking in the wrong place. Same disposal though — the
                // trigger is dropped, and `load_chains` drops the chain if
                // it has no surviving trigger.
                tracing::warn!(
                    chain_id = row.chain_id,
                    event_type = %row.event_type,
                    pct,
                    min = *HEALTH_PCT_RANGE.start(),
                    max = *HEALTH_PCT_RANGE.end(),
                    reason = "health_pct_out_of_range",
                    "entity_health_below: threshold outside the matchable band — \
                     the trigger is a strict downward crossing, so 100 can never \
                     fire (full health is not strictly above 100) and 0 is death \
                     (use entity_dead_tag); dropping this trigger row"
                );
                return None;
            }
            Some(Trigger::OnEntityHealthBelow {
                entity_tag: tag.to_string(),
                pct,
            })
        }
        "npc_flanked" => Some(Trigger::OnNpcFlanked {
            npc_template: key.map(|s| s.to_string()),
        }),
        "player_flanked_npc" => Some(Trigger::OnPlayerFlankedNpc {
            npc_template: key.map(|s| s.to_string()),
        }),
        _ => None,
    }
}
