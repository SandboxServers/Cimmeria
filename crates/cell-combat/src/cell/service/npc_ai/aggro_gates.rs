//! The Idle auto-aggro scan's candidate gates (NA13, D-NA01/D-NA02/D-NA08).
//!
//! A witness becomes a proximity-aggro candidate only when every gate
//! passes, in this order (cheap first, the navmesh ray last):
//!
//! 1. it is a player (`not_player`);
//! 2. it is alive (`dead`);
//! 3. it is not on the NPC's server-side faction (`same_faction`);
//! 4. the NPC's effective aggression toward it is HOSTILE (`not_hostile`):
//!    the override, else the faction reaction table;
//! 5. it is not a GM with `.aggro off` set (`gm_ignored`);
//! 6. it is on the NPC's floor, `|dy| <= 4` (`out_of_vertical_band`);
//! 7. it is inside the NPC's aggro radius, horizontally (`out_of_radius`);
//! 8. it is in line of sight (`no_los`): the world's collision-geometry
//!    occluder when it ships one (NA27, D-NA13), else the navmesh ray.
//!    `Unknown` (an endpoint off the occluder's grid, or off the mesh) fails
//!    **closed** here, unlike the attack check (D-NA08). A space with neither
//!    source has nothing to check and passes; the vertical band is the only
//!    storey guard there. Without an occluder, an NPC standing at a cover
//!    slot looks from the slot's peek point past the prop (NA23, D-NA12; see
//!    `SpaceManager::npc_line_of_sight`).
//!
//! NPC candidates (NPC-vs-NPC, #1009) go through [`evaluate_npc_candidate`]
//! instead: a HOSTILE NPC combatant, alive, not evading or unavailable, then
//! the same room.
//!
//! Gates 6-8 are [`same_room`], which the NA14 assist fan-out
//! (`super::assist`) reuses between a would-be assister and the neighbour
//! that just engaged.
//!
//! The reasons are [`super::detectors::aggro_scan::ScanReject`], the
//! `reason` values of `npc_ai.aggro_scan event=candidate_rejected` (NA02's
//! throttled reporter). Treat them as API.

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::navigation::LineOfSight;

use super::detectors::aggro_scan::ScanReject as AggroReject;
use crate::cell::combat;
use crate::cell::space_manager::SpaceManager;

/// Whether `player` is a GM who switched proximity aggro off with `.aggro
/// off`. The toggle is keyed by character id and only honoured while the
/// entity still holds GM access (server-side `access_level`).
pub(in crate::cell) fn gm_ignores_aggro(space_mgr: &SpaceManager, player: &CellEntity) -> bool {
    crate::cell::dispatch::is_gm(player.access_level)
        && player
            .player_id
            .is_some_and(|c| space_mgr.gm_aggro_off.contains(&c))
}

/// Run every gate for witness `pid` of `npc`. `Ok` carries the horizontal
/// distance, used to pick the closest candidate.
pub(in crate::cell) fn evaluate_candidate(
    space_mgr: &SpaceManager,
    npc: &CellEntity,
    pid: u32,
) -> Result<f32, AggroReject> {
    let Some(p) = space_mgr.get_entity(pid) else {
        return Err(AggroReject::NotPlayer);
    };
    if !p.is_player {
        return Err(AggroReject::NotPlayer);
    }
    if combat::is_dead_state(p.state_field) {
        return Err(AggroReject::Dead);
    }
    if p.faction == npc.faction {
        return Err(AggroReject::SameFaction);
    }
    // Every player reacts as the wire faction today, so this is per NPC in
    // practice; it is evaluated per candidate so a future per-player faction
    // needs no change here.
    if !combat::is_hostile_to_players(npc) {
        return Err(AggroReject::NotHostile);
    }
    if gm_ignores_aggro(space_mgr, p) {
        return Err(AggroReject::GmIgnored);
    }
    same_room(space_mgr, npc, p, combat::aggro_radius(npc))
}

/// The NPC-vs-NPC half of the Idle scan (#1009): run the gates for NPC
/// `cid` near `npc`.
///
/// `None` when `cid` is not a candidate at all: not an NPC combatant (a
/// player, a pet, a being, the NPC itself) or not an NPC `npc` is HOSTILE to
/// (`combat::npc_may_target_npc`, the reaction table read with the NPC as the
/// viewer). Those log nothing: a guard's own post would otherwise log a row
/// per neighbour per tick. `Some(Err)` is a hostile NPC refused, in this
/// order: `dead`, `target_evading` (walking home, NA12), `target_unavailable`
/// (surrendered, despawning, spawning, error), then the [`same_room`]
/// geometry against the scanner's aggro radius. `Some(Ok)` carries the
/// horizontal distance.
pub(in crate::cell) fn evaluate_npc_candidate(
    space_mgr: &SpaceManager,
    npc: &CellEntity,
    cid: u32,
) -> Option<Result<f32, AggroReject>> {
    use cimmeria_entity::cell_entity::AiState;
    let c = space_mgr.get_entity(cid)?;
    if !combat::npc_may_target_npc(npc, c) {
        return None;
    }
    let zero_health = c
        .stats
        .get(cimmeria_entity::stats::HEALTH)
        .is_some_and(|s| s.cur <= 0);
    if combat::is_dead_state(c.state_field) || c.ai_state() == AiState::Dead || zero_health {
        return Some(Err(AggroReject::Dead));
    }
    match c.ai_state() {
        AiState::Leashing => return Some(Err(AggroReject::TargetEvading)),
        AiState::Submit | AiState::Despawning | AiState::Spawning | AiState::Error => {
            return Some(Err(AggroReject::TargetUnavailable))
        }
        _ => {}
    }
    Some(same_room(space_mgr, npc, c, combat::aggro_radius(npc)))
}

/// The geometric half of the gates, shared by the Idle scan and the NA14
/// assist fan-out: `other` is on `npc`'s floor (`|dy| <= 4`), within
/// `radius` horizontally, and in navmesh line of sight from `npc`, with
/// `Unknown` failing closed where a mesh exists (D-NA08). `Ok` carries the
/// horizontal distance.
pub(in crate::cell) fn same_room(
    space_mgr: &SpaceManager,
    npc: &CellEntity,
    other: &CellEntity,
    radius: f32,
) -> Result<f32, AggroReject> {
    if (other.position.y - npc.position.y).abs() > combat::AGGRO_VERTICAL_BAND {
        return Err(AggroReject::OutOfVerticalBand);
    }
    let dist = horizontal_distance(npc, other);
    if dist > radius {
        return Err(AggroReject::OutOfRadius);
    }
    let npc_id = npc.entity_id.0 as u32;
    // From the cover peek point when the NPC stands at a cover slot (NA23,
    // D-NA12): its own ray hits the prop it hides behind.
    match space_mgr
        .npc_line_of_sight(npc_id, other.entity_id.0 as u32)
        .los
    {
        LineOfSight::Clear => {}
        LineOfSight::Blocked => return Err(AggroReject::NoLos),
        LineOfSight::Unknown if space_mgr.space_has_line_of_sight_source(npc_id) => {
            return Err(AggroReject::NoLos)
        }
        LineOfSight::Unknown => {}
    }
    Ok(dist)
}

/// Horizontal (XZ) distance between two entities; the radius metric.
pub(in crate::cell) fn horizontal_distance(a: &CellEntity, b: &CellEntity) -> f32 {
    let dx = b.position.x - a.position.x;
    let dz = b.position.z - a.position.z;
    (dx * dx + dz * dz).sqrt()
}
