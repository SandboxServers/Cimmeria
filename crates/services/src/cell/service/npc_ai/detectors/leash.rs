//! `npc_ai.leash`: `event=enter`, `event=snap_fallback` / `event=arrived`,
//! `event=loop` and `event=damage_ignored`.
//!
//! The leash is today an instant snap (audit S4) and, for an aggressive NPC,
//! half of a 6-second aggro/leash loop (S5: NPC 100630 leashed 120 times in
//! 12 minutes without moving). `loop` counts leash entries per NPC in a
//! sliding window; it does not check whether the target came back into range
//! in between, because the loop never lets it — the NPC leashes on the tick
//! after it aggroes.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;

use super::{MoveSource, NpcIdent};
use crate::cell::space_manager::SpaceManager;

/// Leash entries inside [`LEASH_LOOP_WINDOW`] that make a loop.
pub(in crate::cell) const LEASH_LOOP_COUNT: usize = 3;
pub(in crate::cell) const LEASH_LOOP_WINDOW: Duration = Duration::from_secs(60);
const LEASH_LOOP_WARN_INTERVAL: Duration = Duration::from_secs(60);

/// Fighting -> Leashing, called right after the transition. The target is
/// passed in because the threat list is already cleared by then.
pub(in crate::cell) fn on_enter(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    target_id: u32,
    target_pos: Vector3,
    leash_distance: f32,
    now: Instant,
) {
    let Some(ident) = NpcIdent::of(space_mgr, npc_id) else {
        return;
    };
    let Some(e) = space_mgr.get_entity(npc_id) else {
        return;
    };
    let npc_pos = e.position;
    let spawn = e.spawn_position;
    let nav_path_len = e.nav_path.len();
    tracing::info!(
        target: "npc_ai.leash",
        event = "enter",
        npc_id,
        tag = %ident.tag,
        template_id = ident.template_id,
        world = %ident.world,
        space_id = ident.space_id,
        target_id,
        npc_to_spawn = spawn.map(|s| s.distance_to(&npc_pos)),
        target_to_spawn = spawn.map(|s| s.distance_to(&target_pos)),
        leash_distance,
        npc_x = npc_pos.x,
        npc_y = npc_pos.y,
        npc_z = npc_pos.z,
        target_x = target_pos.x,
        target_y = target_pos.y,
        target_z = target_pos.z,
        nav_path_len,
        "npc_ai.leash: target left the leash radius around spawn"
    );

    let track = space_mgr.npc_detectors.ai.entry(npc_id).or_default();
    track.leash_times.push_back(now);
    while track
        .leash_times
        .front()
        .is_some_and(|t| now.saturating_duration_since(*t) > LEASH_LOOP_WINDOW)
    {
        track.leash_times.pop_front();
    }
    let leash_count = track.leash_times.len();
    if leash_count < LEASH_LOOP_COUNT {
        return;
    }
    cimmeria_observability::counter!("npc_leash_loop_total", "world" => ident.world.clone());
    let Some(suppressed) =
        space_mgr
            .npc_detectors
            .admit_warn(npc_id, "leash_loop", now, LEASH_LOOP_WARN_INTERVAL)
    else {
        return;
    };
    tracing::warn!(
        target: "npc_ai.leash",
        event = "loop",
        npc_id,
        tag = %ident.tag,
        template_id = ident.template_id,
        world = %ident.world,
        space_id = ident.space_id,
        target_id,
        leash_count,
        window_secs = LEASH_LOOP_WINDOW.as_secs(),
        target_to_spawn = spawn.map(|s| s.distance_to(&target_pos)),
        suppressed,
        "npc_ai.leash: NPC is looping aggro -> leash -> snap home -> aggro"
    );
}

/// Leash recovery ran. `from` is where the NPC stood before it; `snapped`
/// is false for a follower, which the leash leaves in place.
pub(in crate::cell) fn on_complete(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    from: Vector3,
    snapped: bool,
) {
    let Some(ident) = NpcIdent::of(space_mgr, npc_id) else {
        return;
    };
    let Some(e) = space_mgr.get_entity(npc_id) else {
        return;
    };
    let to = e.position;
    let stale_path_len = e.nav_path.len();
    let spawn_on_mesh = e
        .spawn_position
        .and_then(|s| space_mgr.diagnose_point(npc_id, &s))
        .map(|v| v.valid);
    if snapped {
        space_mgr
            .npc_detectors
            .note_move_source(npc_id, MoveSource::Leash);
    }
    tracing::info!(
        target: "npc_ai.leash",
        event = if snapped { "snap_fallback" } else { "arrived" },
        npc_id,
        tag = %ident.tag,
        template_id = ident.template_id,
        world = %ident.world,
        space_id = ident.space_id,
        from_x = from.x,
        from_y = from.y,
        from_z = from.z,
        to_x = to.x,
        to_y = to.y,
        to_z = to.z,
        snap_dist = from.distance_to(&to),
        // Today's leash never walks: it is an instant snap.
        walk_secs = 0.0f32,
        path_ok = false,
        spawn_on_mesh,
        // The leash does not clear the chase path (audit S4): a non-zero
        // value here is the path the movement tick walks next.
        stale_path_len,
        snapped,
        "npc_ai.leash: leash complete"
    );
}

/// Damage landed on an NPC that is Leashing. The threat is accrued and then
/// discarded by the leash handler (audit S12).
pub(in crate::cell) fn on_damage_while_leashing(
    space_mgr: &SpaceManager,
    npc_id: u32,
    attacker_id: u32,
    amount: f32,
) {
    let Some(ident) = NpcIdent::of(space_mgr, npc_id) else {
        return;
    };
    tracing::debug!(
        target: "npc_ai.leash",
        event = "damage_ignored",
        npc_id,
        tag = %ident.tag,
        template_id = ident.template_id,
        world = %ident.world,
        space_id = ident.space_id,
        attacker_id,
        amount,
        "npc_ai.leash: threat while leashing is discarded when the leash completes"
    );
}
