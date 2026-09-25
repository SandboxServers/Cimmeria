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
    crate::cell::console::is_gm(player.access_level)
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
