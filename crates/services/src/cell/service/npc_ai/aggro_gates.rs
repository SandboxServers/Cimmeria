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
//! 8. the navmesh sees it (`no_los`). `Unknown` (an endpoint off the mesh)
//!    fails **closed** here, unlike the attack check (D-NA08). A space with no
//!    navmesh at all has nothing to check and passes; the vertical band is the
//!    only storey guard there.
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
    if (p.position.y - npc.position.y).abs() > combat::AGGRO_VERTICAL_BAND {
        return Err(AggroReject::OutOfVerticalBand);
    }
    let dx = p.position.x - npc.position.x;
    let dz = p.position.z - npc.position.z;
    let dist = (dx * dx + dz * dz).sqrt();
    if dist > combat::aggro_radius(npc) {
        return Err(AggroReject::OutOfRadius);
    }
    let npc_id = npc.entity_id.0 as u32;
    match space_mgr.line_of_sight(npc_id, pid) {
        LineOfSight::Clear => {}
        LineOfSight::Blocked => return Err(AggroReject::NoLos),
        LineOfSight::Unknown if space_mgr.space_has_navmesh(npc_id) => {
            return Err(AggroReject::NoLos)
        }
        LineOfSight::Unknown => {}
    }
    Ok(dist)
}
