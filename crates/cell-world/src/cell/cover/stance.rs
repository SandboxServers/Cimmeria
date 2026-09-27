//! The NPC side of holding cover: the spawn hold, Cover Stance, and the one
//! release every combat-end path goes through (NA22).
//!
//! - [`hold_spawn_cover`]: an NPC authored standing at a cover marker
//!   (within [`COVER_ARRIVE_RADIUS`]) spawns holding that slot, so it stays
//!   in cover when combat starts instead of competing for it (audit C4).
//!   Run at spawn, at respawn, when a leash walk gets home, and once for
//!   the startup population when cover finishes loading
//!   ([`hold_spawn_cover_all`]).
//! - [`grant_cover_stance`] / [`revoke_cover_stance`]: ability 1451 "Cover
//!   Stance". Granted when the NPC reaches its slot, removed when it leaves
//!   it. Applied through the effect-script layer
//!   ([`crate::cell::effects::cover_stance`]) with the seed's own effect
//!   rows (4565 buff, 1742 remove). The ids are server constants, never a
//!   client field, so this entry point stays outside the reach concerns of
//!   `abilities-and-effects-system.md` §16.
//! - [`release_npc_cover`]: give the slot back, drop the seek deferral and
//!   revoke the stance. Death, leash and surrender call it.
//!
//! Pose: nothing here reaches the client beyond the NPC's position and
//! velocity. There is no movement-type or pose message (D-NA10).

use cimmeria_common::EntityId;

// `super` is the services `cell::cover` shim, which re-exports
// `cimmeria_cell_cover::cell::cover`.
use super::{horizontal, lock_or_recover, CoverSlotKey, COVER_ARRIVE_RADIUS};
use crate::cell::effects::{self, EffectContext};
use crate::cell::space_manager::SpaceManager;

/// Ability 1451 "Cover Stance" (`abilities.sql`).
pub const COVER_STANCE_ABILITY: i32 = 1451;
/// Effect 4565, the stance buff ("+100 CoverDefense").
pub const COVER_STANCE_EFFECT: i32 = 4565;
/// Effect 1742, "Remove Stance Moniker".
pub const COVER_STANCE_REMOVE_EFFECT: i32 = 1742;

/// Vertical band for the spawn hold: a marker on the floor above or below
/// is not the one the NPC stands at.
const SPAWN_HOLD_MAX_DY: f32 = 2.0;

/// Run one of the stance effects on `npc_id`, self-sourced. Returns whether
/// a script ran. A missing def or script name is a seed problem and warns.
fn run_stance_effect(space_mgr: &mut SpaceManager, npc_id: u32, effect_id: i32) -> bool {
    let Some(effect) = space_mgr.effect_defs.get(&effect_id).cloned() else {
        tracing::warn!(
            target: "cover.stance",
            event = "effect_missing",
            npc_id,
            ability_id = COVER_STANCE_ABILITY,
            effect_id,
            reason = "no_effect_def",
            "Cover Stance effect has no server def -- the stance is tracked but no stat changes"
        );
        return false;
    };
    let Some(script) = effect.script_name.clone() else {
        tracing::warn!(
            target: "cover.stance",
            event = "effect_missing",
            npc_id,
            ability_id = COVER_STANCE_ABILITY,
            effect_id,
            reason = "no_script_name",
            "Cover Stance effect row has no script_name -- check effects.sql"
        );
        return false;
    };
    let mut ctx = EffectContext {
        source_id: npc_id,
        target_id: npc_id,
        effect: &effect,
        space_mgr,
    };
    effects::dispatch_by_name(&script, &mut ctx)
}

/// Grant Cover Stance to an NPC that reached its slot. Idempotent: returns
/// `true` only on the tick it was granted.
pub fn grant_cover_stance(space_mgr: &mut SpaceManager, npc_id: u32) -> bool {
    let newly =
        lock_or_recover(&space_mgr.cover.reservations).enter_stance(EntityId(npc_id as i32));
    if !newly {
        return false;
    }
    let applied = run_stance_effect(space_mgr, npc_id, COVER_STANCE_EFFECT);
    tracing::debug!(
        target: "cover.stance",
        event = "granted",
        npc_id,
        ability_id = COVER_STANCE_ABILITY,
        applied,
        "Cover Stance granted on reaching the cover slot"
    );
    true
}

/// Remove Cover Stance. Idempotent: returns `true` only when the NPC held
/// it.
pub fn revoke_cover_stance(space_mgr: &mut SpaceManager, npc_id: u32) -> bool {
    let held = lock_or_recover(&space_mgr.cover.reservations).leave_stance(EntityId(npc_id as i32));
    if !held {
        return false;
    }
    let applied = run_stance_effect(space_mgr, npc_id, COVER_STANCE_REMOVE_EFFECT);
    tracing::debug!(
        target: "cover.stance",
        event = "revoked",
        npc_id,
        ability_id = COVER_STANCE_ABILITY,
        applied,
        "Cover Stance removed on leaving the cover slot"
    );
    true
}

/// Release an NPC's cover slot, its seek deferral and its Cover Stance.
/// Idempotent. `reason` names the caller for the `cover.hold` row
/// (`death`, `leash`, `submit`, ...).
pub fn release_npc_cover(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    reason: &'static str,
) -> Option<CoverSlotKey> {
    let id = EntityId(npc_id as i32);
    let slot = {
        let mut r = lock_or_recover(&space_mgr.cover.reservations);
        r.clear_seek(id);
        r.release_for_entity(id)
    };
    let stance = revoke_cover_stance(space_mgr, npc_id);
    if let Some(slot) = slot {
        tracing::debug!(
            target: "cover.hold",
            event = "released",
            npc_id,
            chunk_id = slot.chunk_id,
            node_id = slot.node_id,
            reason,
            stance,
            "NPC cover slot released"
        );
    }
    slot
}

/// Reserve the cover slot an NPC stands at, if any: the nearest free node
/// of its world within [`COVER_ARRIVE_RADIUS`] horizontally and
/// `SPAWN_HOLD_MAX_DY` vertically. Only for a mobile NPC with `use_cover`.
/// Returns the held slot (an NPC already holding one keeps it).
pub fn hold_spawn_cover(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    cause: &'static str,
) -> Option<CoverSlotKey> {
    let npc = space_mgr.get_entity(npc_id)?;
    if !npc.use_cover || npc.is_stationary || npc.is_player {
        return None;
    }
    let pos = npc.position;
    let world_id = space_mgr.get_entity_world_id(npc_id)?;
    let cover = &space_mgr.cover;
    if cover.node_count() == 0 {
        return None;
    }
    let id = EntityId(npc_id as i32);
    let mut r = lock_or_recover(&cover.reservations);
    if let Some(held) = r.slot_for_entity(id) {
        return Some(held);
    }
    let (node, dist) = cover
        .index
        .nearby(world_id, &pos, COVER_ARRIVE_RADIUS, Some(SPAWN_HOLD_MAX_DY))
        .into_iter()
        .filter_map(|idx| cover.index.node(idx))
        .filter(|n| !r.is_reserved(n.key()))
        .map(|n| (n, horizontal(&n.pos, &pos)))
        .filter(|&(_, d)| d <= COVER_ARRIVE_RADIUS)
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    let slot = node.key();
    r.reserve_for_entity(id, slot).ok()?;
    tracing::info!(
        target: "cover.hold",
        event = "spawn_reserved",
        npc_id,
        chunk_id = slot.chunk_id,
        node_id = slot.node_id,
        dist,
        world_id,
        cause,
        "NPC holds the cover slot it stands at"
    );
    Some(slot)
}

/// [`hold_spawn_cover`] for every NPC standing at its spawn point. Run once
/// the cover index is loaded: the startup population spawns before cover
/// and world ids exist, so its per-spawn holds found nothing. Returns how
/// many NPCs now hold a slot.
pub fn hold_spawn_cover_all(space_mgr: &mut SpaceManager) -> usize {
    let mut ids = space_mgr.all_npc_entity_ids();
    ids.sort_unstable();
    let mut held = 0;
    for id in ids {
        let at_spawn = space_mgr.get_entity(id).is_some_and(|e| {
            e.spawn_position
                .is_some_and(|s| horizontal(&s, &e.position) <= f32::EPSILON)
        });
        if at_spawn && hold_spawn_cover(space_mgr, id, "startup").is_some() {
            held += 1;
        }
    }
    if held > 0 {
        tracing::info!(
            target: "cover.hold",
            event = "startup_summary",
            held,
            "NPCs holding the cover slot they spawned at"
        );
    }
    held
}
