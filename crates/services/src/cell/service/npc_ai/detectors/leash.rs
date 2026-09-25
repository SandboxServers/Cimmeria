//! `npc_ai.leash`: `event=enter`, `event=snap_fallback` / `event=arrived`,
//! `event=loop` and `event=damage_ignored`.
//!
//! Before NA12 the leash was an instant snap (audit S4) and, for an
//! aggressive NPC, half of a 6-second aggro/leash loop (S5: NPC 100630
//! leashed 120 times in 12 minutes without moving). NA12 walks the NPC home
//! and measures the leash on the NPC itself; these rows are how that is
//! checked in play. `loop` counts leash entries per NPC in a sliding window
//! and should read zero after NA12.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;

use super::NpcIdent;
use crate::cell::space_manager::SpaceManager;

/// Leash entries inside [`LEASH_LOOP_WINDOW`] that make a loop.
pub(in crate::cell) const LEASH_LOOP_COUNT: usize = 3;
pub(in crate::cell) const LEASH_LOOP_WINDOW: Duration = Duration::from_secs(60);
const LEASH_LOOP_WARN_INTERVAL: Duration = Duration::from_secs(60);

/// Why and how an NPC entered Leashing, for the `enter` row.
pub(in crate::cell) struct LeashEntry {
    /// The target the NPC gave up on, when there was one (a leash on the
    /// NPC's own distance has one; a lost last target does not).
    pub target: Option<(u32, Vector3)>,
    pub leash_distance: f32,
    /// The transition reason label (`leash_out`, `target_lost`,
    /// `threat_empty`).
    pub reason: &'static str,
    /// What fired it (`beyond_band`, `chase_outward`, `vertical_cap`,
    /// `target_dead`, `target_gone`, `target_out_of_aoi`, `threat_empty`).
    pub trigger: &'static str,
}

/// Fighting -> Leashing, called once the route home is installed, so
/// `nav_path_len` is the walk (0 = no route: the snap fallback follows).
/// The target is passed in because the threat list is already cleared.
pub(in crate::cell) fn on_enter(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    entry: LeashEntry,
    now: Instant,
) {
    let LeashEntry {
        target,
        leash_distance,
        reason,
        trigger,
    } = entry;
    let target_id = target.map_or(0, |(id, _)| id);
    let target_pos = target.map(|(_, p)| p);
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
        reason,
        trigger,
        // Horizontal: the distance the NA12 leash measures.
        npc_to_spawn = spawn.map(|s| horizontal(&npc_pos, &s)),
        target_to_spawn = spawn.zip(target_pos).map(|(s, t)| s.distance_to(&t)),
        leash_distance,
        npc_x = npc_pos.x,
        npc_y = npc_pos.y,
        npc_z = npc_pos.z,
        target_x = target_pos.map(|t| t.x),
        target_y = target_pos.map(|t| t.y),
        target_z = target_pos.map(|t| t.z),
        nav_path_len,
        "npc_ai.leash: NPC gave up its fight and is heading home"
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
        target_to_spawn = spawn.zip(target_pos).map(|(s, t)| s.distance_to(&t)),
        suppressed,
        "npc_ai.leash: NPC is looping aggro -> leash -> home -> aggro"
    );
}

fn horizontal(a: &Vector3, b: &Vector3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// How a leash ended, for the `arrived` / `snap_fallback` row.
pub(in crate::cell) struct LeashEnd {
    /// Where the NPC stood just before the reset (before the snap, if any).
    pub from: Vector3,
    /// `walked` | `in_place` (follower or no spawn) | `snap_no_path` |
    /// `snap_timeout`.
    pub arrival: &'static str,
    /// The NPC was teleported to spawn (the fallback).
    pub snapped: bool,
    /// Seconds since the walk started; `None` when no walk clock was set.
    pub walk_secs: Option<f32>,
}

/// Leash recovery ran: `event=arrived` for a walk (or an in-place reset),
/// `event=snap_fallback` for a snap.
pub(in crate::cell) fn on_complete(space_mgr: &mut SpaceManager, npc_id: u32, end: LeashEnd) {
    let LeashEnd {
        from,
        arrival,
        snapped,
        walk_secs,
    } = end;
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
        arrival,
        walk_secs,
        path_ok = arrival == "walked",
        spawn_on_mesh,
        // Since NA10 the snap goes through `snap_npc_to`, which stops the
        // NPC: this must read 0. Non-zero is the pre-NA10 bug (audit S4),
        // the chase path the movement tick would walk back out along.
        stale_path_len,
        snapped,
        "npc_ai.leash: leash complete"
    );
}

/// Damage landed on an NPC that is Leashing. Since NA12 the NPC evades:
/// `generate_threat` refuses the threat and the attacker does not enter
/// combat (audit S12). This row is the only trace.
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
        "npc_ai.leash: NPC is walking home (evading): threat ignored"
    );
}
