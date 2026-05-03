//! Ground-target AoE entry point.
//!
//! `useAbilityOnGroundTarget` arrives from the client when the player fires an
//! ability without picking an explicit entity (point-and-click on terrain).
//! We collect every hostile NPC inside the AoE radius (read from the ability's
//! effect definition, defaulting to `DEFAULT_GROUND_TARGET_RADIUS` when the
//! NVP is absent) and apply damage to all of them. The cooldown/ammo are
//! consumed exactly once via the primary-target call to `handle_use_ability`;
//! additional targets get `damage_apply::apply_damage_to_target` directly so
//! we don't re-charge per target.

use tokio::sync::mpsc;

use super::super::combat;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::damage_apply::apply_damage_to_target;
use super::use_ability::handle_use_ability;

/// Fallback radius for ground-target abilities when the effect definition's
/// `Radius` NVP is missing. Matches the pre-#81 hardcoded value so existing
/// abilities without explicit radius data behave the same as before.
const DEFAULT_GROUND_TARGET_RADIUS: f32 = 5.0;

/// Read the AoE radius from the ability's first effect definition's `Radius`
/// NVP. Returns `DEFAULT_GROUND_TARGET_RADIUS` when the ability is unknown,
/// has no effects, or the effects don't specify a radius — preserves
/// existing behavior for abilities without authored AoE data.
fn ability_radius(ability_def: &Option<cimmeria_entity::abilities::AbilityDef>, space_mgr: &SpaceManager) -> f32 {
    let Some(def) = ability_def else { return DEFAULT_GROUND_TARGET_RADIUS };
    for &eid in &def.effect_ids {
        if let Some(effect) = space_mgr.effect_defs.get(&eid) {
            let r = effect.param_f32("Radius");
            if r > 0.0 {
                return r;
            }
        }
    }
    DEFAULT_GROUND_TARGET_RADIUS
}

/// Handle a ground-targeted ability — applies damage to **every** hostile
/// NPC within the AoE radius, not just the nearest. The radius comes from
/// the ability's effect definition (`Radius` NVP), falling back to a
/// hardcoded default. Cooldown and ammo are consumed exactly once: the
/// primary (nearest) target goes through `handle_use_ability`, then each
/// additional target gets `apply_damage_to_target` directly with
/// `needs_ammo_stat_send: false` so we don't double-flush the bandolier
/// stat.
///
/// If no enemy is in range, the call still consumes cooldown and ammo via
/// `handle_use_ability(target_id=0)` so the player can't spam.
///
/// Returns `Some(primary_target_eid)` if the ability resolved against any
/// enemy NPC (so callers can detect alive→dead transitions for content-
/// engine death events on the primary target), or `None` if the cooldown
/// was consumed without picking a target. AoE secondary kills don't fire
/// the content-engine death event today — the python reference fired
/// `onDead` per target, but routing each AoE death through the
/// fire-entity-death event chain is a follow-up.
pub async fn handle_use_ability_on_ground(
    entity_id: u32,
    ability_id: i32,
    ground: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<u32> {
    let ability_def = space_mgr.ability_defs.get(&ability_id).cloned();
    let radius = ability_radius(&ability_def, space_mgr);
    let radius_sq = radius * radius;

    // Collect every hostile NPC within the AoE radius, sorted by distance
    // from the click point. The first becomes the primary (consumes
    // cooldown/ammo), the rest get damage applied directly.
    //
    // Hostile-faction sentinel matches `cell_methods/player/interaction.rs`'s
    // hostile check (`!is_player && faction == 10`). Without it, AoE
    // would happily damage vendors, quest givers, and neutral wildlife.
    const HOSTILE_FACTION: u8 = 10;
    let mut targets: Vec<(u32, f32)> = Vec::new();
    for npc_eid in space_mgr.all_npc_entity_ids() {
        if let Some(npc) = space_mgr.get_entity(npc_eid) {
            if combat::is_dead_state(npc.state_field) {
                continue;
            }
            if npc.faction != HOSTILE_FACTION {
                continue;
            }
            let dx = npc.position.x - ground[0];
            let dy = npc.position.y - ground[1];
            let dz = npc.position.z - ground[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq <= radius_sq {
                targets.push((npc_eid, dist_sq));
            }
        }
    }
    targets.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Primary range check: the closest target must also be within the
    // ability's own `max_range` from the attacker, otherwise
    // handle_use_ability bails before the cooldown starts and the player
    // can spam-click. Falling back to target_id = 0 keeps the
    // cooldown/ammo charge in that case.
    let max_range = ability_def
        .as_ref()
        .map_or(30.0, |d| if d.max_range > 0 { d.max_range as f32 } else { 30.0 });

    let primary_in_range = targets.first().map_or(false, |&(target_eid, _)| {
        match (space_mgr.get_entity(entity_id), space_mgr.get_entity(target_eid)) {
            (Some(attacker), Some(target)) => attacker.position.distance_to(&target.position) <= max_range,
            _ => false,
        }
    });

    if targets.is_empty() {
        tracing::debug!(
            entity_id, ability_id, ?ground, radius,
            "useAbilityOnGroundTarget: no enemy in AoE radius; consuming cooldown/ammo without damage"
        );
        handle_use_ability(entity_id, ability_id, 0, tx, space_mgr).await;
        return None;
    }

    if !primary_in_range {
        let (primary_eid, _) = targets[0];
        tracing::debug!(
            entity_id, ability_id, ?ground, primary_eid, max_range,
            "useAbilityOnGroundTarget: nearest target outside attacker's ability max_range; charging cooldown/ammo only"
        );
        handle_use_ability(entity_id, ability_id, 0, tx, space_mgr).await;
        return None;
    }

    // Primary target: full handle_use_ability path (consumes ammo,
    // starts cooldown, sends timer/sequence/state-field, applies damage).
    let (primary_eid, _) = targets[0];
    tracing::debug!(
        entity_id, ability_id, ?ground, primary_eid, radius,
        target_count = targets.len(),
        "useAbilityOnGroundTarget: AoE — primary target via handle_use_ability"
    );
    handle_use_ability(entity_id, ability_id, primary_eid as i32, tx, space_mgr).await;

    // Secondary targets: damage only, fresh effect_seq per target so the
    // client can correlate per-target effect packets independently. We
    // reuse the primary's effect_seq base by reading the next ID from
    // the entity's ability manager — each `next_effect_id()` call mints
    // a unique value.
    for &(secondary_eid, _) in targets.iter().skip(1) {
        let secondary_seq = space_mgr.get_entity_mut(entity_id)
            .map(|e| e.abilities.next_effect_id())
            .unwrap_or(0);
        tracing::debug!(
            entity_id, ability_id, secondary_eid, secondary_seq,
            "useAbilityOnGroundTarget: AoE — secondary target via apply_damage_to_target"
        );
        apply_damage_to_target(
            entity_id,
            secondary_eid,
            ability_id,
            &ability_def,
            secondary_seq as u32,
            // No ammo flush on secondaries — primary already flushed.
            false,
            tx,
            space_mgr,
        ).await;
    }

    Some(primary_eid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthesize an `AbilityDef` + matching `EffectDef` with a `Radius`
    /// NVP and assert `ability_radius` reads it.
    #[test]
    fn ability_radius_reads_effect_nvp() {
        use cimmeria_entity::abilities::{AbilityDef, EffectDef};
        use std::collections::HashMap;

        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        let mut effect_params = HashMap::new();
        effect_params.insert("Radius".to_string(), "12.5".to_string());
        mgr.effect_defs.insert(500, EffectDef {
            effect_id: 500,
            ability_id: 999,
            delay: 0,
            effect_sequence: 0,
            event_set_id: None,
            script_name: None,
            params: effect_params,
        });

        let ability = AbilityDef {
            ability_id: 999,
            name: "GroundTargetAoE".to_string(),
            cooldown: 1.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![500],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        };

        assert_eq!(ability_radius(&Some(ability), &mgr), 12.5);
    }

    #[test]
    fn ability_radius_falls_back_to_default_when_unset() {
        let mgr = SpaceManager::new(1);
        // No ability def → default
        assert_eq!(ability_radius(&None, &mgr), DEFAULT_GROUND_TARGET_RADIUS);
    }

    #[test]
    fn ability_radius_falls_back_when_effects_have_no_radius() {
        use cimmeria_entity::abilities::{AbilityDef, EffectDef};
        use std::collections::HashMap;

        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        // Effect with HealthDamage but no Radius → fall back to default.
        let mut effect_params = HashMap::new();
        effect_params.insert("HealthDamage".to_string(), "20".to_string());
        mgr.effect_defs.insert(501, EffectDef {
            effect_id: 501, ability_id: 998, delay: 0, effect_sequence: 0,
            event_set_id: None, script_name: None, params: effect_params,
        });

        let ability = AbilityDef {
            ability_id: 998, name: "NoRadius".to_string(),
            cooldown: 1.0, warmup: 0.0, flags: 0, is_ranged: true,
            min_range: 0, max_range: 30, target_type_id: 0,
            effect_ids: vec![501], moniker_ids: vec![],
            required_ammo: 0, event_set_id: None, velocity: 0.0,
        };

        assert_eq!(ability_radius(&Some(ability), &mgr), DEFAULT_GROUND_TARGET_RADIUS);
    }
}
