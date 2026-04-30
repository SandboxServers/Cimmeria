//! Ground-target auto-aim entry point.
//!
//! `useAbilityOnGroundTarget` arrives from the client when the player fires an
//! ability without picking an explicit entity (point-and-click on terrain).
//! We pick the nearest hostile NPC inside the auto-aim radius and resolve the
//! ability against it via the targeted path. If nothing is in range, the call
//! still consumes cooldown and ammo to prevent spam.

use tokio::sync::mpsc;

use super::super::combat;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::use_ability::handle_use_ability;

/// Default radius for ground-targeted ability auto-aim, in world units.
/// Picks the nearest enemy NPC within this radius of the click point and
/// resolves the ability against it. A future combat pass should replace
/// this with true AoE damage application driven by the ability's effect
/// definition (radius + per-tick damage parameters).
const GROUND_TARGET_AUTOAIM_RADIUS: f32 = 5.0;

/// Handle a ground-targeted ability: pick the nearest enemy NPC within
/// `GROUND_TARGET_AUTOAIM_RADIUS` of the click point and resolve the ability
/// against it, sharing all the cooldown/ammo/state-field accounting from the
/// targeted path. If no enemy is in range, the call still consumes cooldown
/// and ammo via `handle_use_ability(target_id=0)` so the player can't spam.
/// Returns `Some(target_eid)` if the ability resolved against an enemy NPC
/// (so callers can detect alive→dead transitions for content-engine death
/// events), or `None` if the cooldown was consumed without picking a target.
pub async fn handle_use_ability_on_ground(
    entity_id: u32,
    ability_id: i32,
    ground: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<u32> {
    // Find the nearest enemy NPC to the ground click. Players targeting
    // friendly factions or themselves with a ground ability fall through to
    // the no-target branch below.
    let nearest = {
        // Hostile-faction sentinel matches `cell_methods/player/interaction.rs`'s
        // hostile check (`!is_player && faction == 10`). Without it, auto-aim
        // would happily target vendors, quest givers, and neutral wildlife — a
        // ground-targeted ability fired near a friendly NPC would damage them.
        const HOSTILE_FACTION: u8 = 10;

        let mut best: Option<(u32, f32)> = None;
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
                if dist_sq <= GROUND_TARGET_AUTOAIM_RADIUS * GROUND_TARGET_AUTOAIM_RADIUS {
                    let better = best.map_or(true, |(_, d)| dist_sq < d);
                    if better {
                        best = Some((npc_eid, dist_sq));
                    }
                }
            }
        }
        best.map(|(eid, _)| eid)
    };

    // If we picked an auto-aim target, verify it's also within the ability's
    // own max_range from the attacker. Otherwise handle_use_ability would do
    // its targeted range check, fail, and bail out *before* the cooldown is
    // started — letting the player re-spam ground abilities by clicking far
    // away with an NPC near the click point. Falling back to target_id = 0
    // here keeps the cooldown/ammo charge regardless of whether the targeted
    // path or the no-target path runs.
    let ability_def = space_mgr.ability_defs.get(&ability_id).cloned();
    let max_range = ability_def
        .as_ref()
        .map_or(30.0, |d| if d.max_range > 0 { d.max_range as f32 } else { 30.0 });

    let target_in_range = match nearest {
        Some(target_eid) => match (space_mgr.get_entity(entity_id), space_mgr.get_entity(target_eid)) {
            (Some(attacker), Some(target)) => attacker.position.distance_to(&target.position) <= max_range,
            _ => false,
        },
        None => false,
    };

    match (nearest, target_in_range) {
        (Some(target_eid), true) => {
            tracing::debug!(
                entity_id, ability_id, ?ground, target_eid,
                "useAbilityOnGroundTarget: resolving against nearest enemy in radius"
            );
            handle_use_ability(entity_id, ability_id, target_eid as i32, tx, space_mgr).await;
            Some(target_eid)
        }
        (Some(target_eid), false) => {
            tracing::debug!(
                entity_id, ability_id, ?ground, target_eid, max_range,
                "useAbilityOnGroundTarget: auto-aimed target is outside attacker's ability max_range; charging cooldown/ammo only"
            );
            handle_use_ability(entity_id, ability_id, 0, tx, space_mgr).await;
            None
        }
        (None, _) => {
            tracing::debug!(
                entity_id, ability_id, ?ground,
                "useAbilityOnGroundTarget: no enemy in radius; consuming cooldown/ammo without damage"
            );
            handle_use_ability(entity_id, ability_id, 0, tx, space_mgr).await;
            None
        }
    }
}
