//! Same-room assist aggro (NA14, D-NA04): when an NPC enters Fighting from
//! damage or proximity, its hostile same-faction neighbours in the same room
//! join the fight on the same target (`cause=assist`).
//!
//! **A deliberate deviation from legacy.** The 2009 server had no assist:
//! `SGWMob` only ever took threat from its own attackers, so shooting one
//! of two guards standing side by side left the other watching. The owner
//! approved same-room assist (D-NA04) because that reads as broken AI.
//!
//! A neighbour is pulled in only when every gate passes (the order of
//! [`evaluate_assister`], cheap first, the navmesh ray last):
//!
//! 1. it is an NPC in the victim's space, on the victim's server-side
//!    faction, and within twice its assist radius (the *considered* set:
//!    only these log a reject row);
//! 2. it is alive (`dead`);
//! 3. it is Idle, patrolling or wandering (`not_idle`): an NPC already
//!    fighting, walking home (Leashing), investigating, following or dead
//!    is never pulled;
//! 4. it is itself HOSTILE to players (`not_hostile`), so a chain-armed
//!    spawn seeded NEUTRAL (spawns 10 and 20) still waits for its chain;
//! 5. it is not inside its NA12 post-reset window (`post_reset_suppressed`);
//! 6. the target is not a GM with `.aggro off` (`gm_ignored`);
//! 7. it is within 4 u of the victim's height, within its own
//!    `entity_templates.assist_radius` (default 10 u) of the victim
//!    horizontally, and sees the victim through the navmesh, `Unknown`
//!    failing closed where a mesh exists ([`super::aggro_gates::same_room`]).
//!
//! **No chaining.** An assister enters Fighting with
//! [`combat::AggroCause::Assist`], which does not recruit
//! ([`combat::AggroCause::recruits_assist`]), so a fight never ripples from
//! room to room. A content chain's `generate_threat` does not recruit
//! either: a scripted fight stays as scripted.

use std::time::Instant;

use cimmeria_entity::cell_entity::{AiState, CellEntity};

use super::aggro_gates::{gm_ignores_aggro, horizontal_distance, same_room};
use super::detectors::aggro_scan::{report_assist_rejects, ScanReject};
use super::detectors::NpcIdent;
use crate::cell::combat;
use crate::cell::space_manager::SpaceManager;

/// Threat an assister takes on the target. The same tiny seed as proximity
/// aggro, so the assister's own attackers (and any content threat) dominate
/// its target choice as soon as the fight starts.
pub(in crate::cell) const ASSIST_THREAT_SEED: f32 = 1.0;

/// Neighbours further than this multiple of their assist radius are not
/// considered at all, so a big level does not log a reject row for every
/// same-faction NPC in it. Inside it, a near miss logs `out_of_radius`.
const CONSIDER_RADIUS_FACTOR: f32 = 2.0;

/// Pull `victim_id`'s qualifying neighbours into its fight against
/// `target_id`. Called by `combat::generate_threat` right after the victim
/// entered Fighting for a cause that recruits (damage or proximity).
///
/// Only a live player target recruits: NPC-on-NPC threat has no room to
/// defend.
pub(in crate::cell) fn recruit_assisters(
    space_mgr: &mut SpaceManager,
    victim_id: u32,
    target_id: u32,
) {
    let Some(target) = space_mgr.get_entity(target_id) else {
        return;
    };
    if !target.is_player || combat::is_dead_state(target.state_field) {
        return;
    }
    let gm_ignored = gm_ignores_aggro(space_mgr, target);
    let now = Instant::now();

    let mut candidates = space_mgr.npc_ids_in_space_of(victim_id);
    // Deterministic order for the logs and the threat seeding.
    candidates.sort_unstable();

    let mut joined: Vec<(u32, f32)> = Vec::new();
    let mut rejects: Vec<(u32, ScanReject)> = Vec::new();
    {
        let Some(victim) = space_mgr.get_entity(victim_id) else {
            return;
        };
        for id in candidates {
            let Some(npc) = space_mgr.get_entity(id) else {
                continue;
            };
            if npc.faction != victim.faction
                || horizontal_distance(npc, victim)
                    > CONSIDER_RADIUS_FACTOR * combat::assist_radius(npc)
            {
                continue;
            }
            match evaluate_assister(space_mgr, npc, victim, gm_ignored, now) {
                Ok(dist) => joined.push((id, dist)),
                Err(reason) => rejects.push((id, reason)),
            }
        }
    }
    report_assist_rejects(space_mgr, victim_id, target_id, &rejects, now);

    for (assister_id, dist) in joined {
        if let Some(ident) = NpcIdent::of(space_mgr, assister_id) {
            tracing::debug!(
                target: "npc_ai.aggro_scan",
                event = "assist_joined",
                npc_id = assister_id,
                tag = %ident.tag,
                template_id = ident.template_id,
                world = %ident.world,
                space_id = ident.space_id,
                victim_id,
                player_id = target_id,
                npc_to_victim = dist,
                "npc_ai.aggro_scan: neighbour pulled into the fight (assist)"
            );
        }
        // The victim's own `enter_player_combat` already ran, so the player
        // is in combat and this returns `None`: there is no state-field
        // update of its own to send. The `npc_ai.aggro event=acquired
        // cause=assist` row and the `npc_ai_aggro_total{cause=assist}`
        // count come from `generate_threat` itself.
        let _ = combat::generate_threat(
            space_mgr,
            target_id,
            assister_id,
            ASSIST_THREAT_SEED,
            combat::AggroCause::Assist,
        );
    }
}

/// Every assist gate for neighbour `npc` of `victim` (see the module doc).
/// `Ok` carries the horizontal distance to the victim.
fn evaluate_assister(
    space_mgr: &SpaceManager,
    npc: &CellEntity,
    victim: &CellEntity,
    target_gm_ignored: bool,
    now: Instant,
) -> Result<f32, ScanReject> {
    if npc.ai_state() == AiState::Dead || combat::is_dead_state(npc.state_field) {
        return Err(ScanReject::Dead);
    }
    if !matches!(
        npc.ai_state(),
        AiState::Idle | AiState::Patrol | AiState::Wander
    ) {
        return Err(ScanReject::NotIdle);
    }
    if !combat::is_hostile_to_players(npc) {
        return Err(ScanReject::NotHostile);
    }
    if npc.leash.reaggro_suppressed(now) {
        return Err(ScanReject::PostResetSuppressed);
    }
    if target_gm_ignored {
        return Err(ScanReject::GmIgnored);
    }
    same_room(space_mgr, npc, victim, combat::assist_radius(npc))
}
