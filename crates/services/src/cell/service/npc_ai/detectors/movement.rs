//! Movement-tick detectors: `movement.npc event=stale_velocity` and
//! `movement.npc event=ground_deviation`.
//!
//! **stale_velocity** is the running-in-place detector (audit S1). The AoI
//! tick sends every witness the stored `velocity` every 100 ms whether the
//! NPC moved or not, and eight paths clear `nav_path` mid-leg without zeroing
//! it (`attack_in_place` is the common one). The client is then told "moving
//! at 6 u/s" about an NPC that is standing still.
//!
//! It also covers the telemetry plan's `animating_without_path`. That event
//! was specified against the last `setMovementType` sent, but NA10's Ghidra
//! pass showed the client animates NPC movement from **velocity alone** —
//! there is no server-to-client movement-type message. Re-based on velocity
//! it becomes "non-zero velocity, empty path, not moving", which is this
//! detector with `path_state = "empty"`. One row per bug instead of two;
//! `path_state = "stalled"` is the other shape (a path the NPC is not
//! walking: zero speed, or a waypoint it cannot reach).
//!
//! **ground_deviation** is the floating detector (audit M1/M2/T3): the
//! movement tick lerps Y along 2D Detour legs, so a floor-then-ramp leg
//! climbs an invisible slope. It reads the storey-aware ground query from
//! NA01 (`get_height_near`), never the old origin-biased one.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;

use super::NpcIdent;
use crate::cell::space_manager::SpaceManager;

/// A velocity is "stale" after this many consecutive movement ticks
/// (300 ms at the 100 ms cadence) with no displacement.
pub(in crate::cell) const STALE_VELOCITY_TICKS: u32 = 3;
const STALE_VELOCITY_WARN_INTERVAL: Duration = Duration::from_secs(10);
/// Displacement below this is "did not move" (float noise, not motion).
const STILL_EPSILON: f32 = 1e-3;

/// `|y - ground_y|` above this is a deviation.
pub(in crate::cell) const GROUND_DEVIATION_THRESHOLD: f32 = 0.3;
const GROUND_DEVIATION_WARN_INTERVAL: Duration = Duration::from_secs(5);

/// Run once per movement tick, after `npc_movement_tick`, over **every**
/// NPC — the movement tick itself only visits NPCs with a path, and the
/// stale-velocity case is precisely an NPC whose path was cleared.
pub(in crate::cell) fn after_movement_tick(space_mgr: &mut SpaceManager, now: Instant) {
    for npc_id in space_mgr.all_npc_entity_ids() {
        let Some(e) = space_mgr.get_entity(npc_id) else {
            continue;
        };
        let pos = e.position;
        let velocity = e.velocity;
        let moving = velocity.iter().any(|v| *v != 0.0);
        let dead = e.ai_state() == cimmeria_entity::cell_entity::AiState::Dead;

        let track = space_mgr.npc_detectors.movement.entry(npc_id).or_default();
        let moved = track
            .last_pos
            .is_some_and(|last| last.distance_to(&pos) > STILL_EPSILON);
        track.moved_last_tick = moved;
        let first_sample = track.last_pos.is_none();
        track.last_pos = Some(pos);
        if moving && !moved && !first_sample && !dead {
            track.still_ticks = track.still_ticks.saturating_add(1);
        } else {
            track.still_ticks = 0;
        }
        if track.still_ticks >= STALE_VELOCITY_TICKS {
            report_stale_velocity(space_mgr, npc_id, now);
        }
    }
}

fn report_stale_velocity(space_mgr: &mut SpaceManager, npc_id: u32, now: Instant) {
    let Some(ident) = NpcIdent::of(space_mgr, npc_id) else {
        return;
    };
    cimmeria_observability::counter!(
        "npc_stale_velocity_total",
        "world" => ident.world.clone(),
    );
    let still_ticks = space_mgr
        .npc_detectors
        .movement
        .get(&npc_id)
        .map_or(0, |m| m.still_ticks);
    let Some(suppressed) = space_mgr.npc_detectors.admit_warn(
        npc_id,
        "stale_velocity",
        now,
        STALE_VELOCITY_WARN_INTERVAL,
    ) else {
        return;
    };
    let Some(e) = space_mgr.get_entity(npc_id) else {
        return;
    };
    let [vx, vy, vz] = e.velocity;
    let path_state = if e.nav_path.is_empty() {
        "empty"
    } else {
        "stalled"
    };
    tracing::warn!(
        target: "movement.npc",
        event = "stale_velocity",
        npc_id,
        tag = %ident.tag,
        template_id = ident.template_id,
        world = %ident.world,
        space_id = ident.space_id,
        vx,
        vy,
        vz,
        speed = (vx * vx + vy * vy + vz * vz).sqrt(),
        still_ticks,
        path_state,
        nav_path_len = e.nav_path.len(),
        ai_state = e.ai_state().label(),
        movement_type = ?e.last_movement_type,
        x = e.position.x,
        y = e.position.y,
        z = e.position.z,
        suppressed,
        "movement.npc: NPC is standing still while every witness is told it moves \
         -- velocity was never zeroed when its path was cleared (running in place)"
    );
}

/// How the movement tick arrived at the Y it just wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell) enum YSource {
    /// Linear interpolation along the leg toward the next waypoint.
    Lerp,
    /// Snapped onto a waypoint.
    Waypoint,
}

impl YSource {
    fn label(self) -> &'static str {
        match self {
            Self::Lerp => "lerp",
            Self::Waypoint => "waypoint",
        }
    }
}

/// One movement step, as the ground detector sees it.
#[derive(Debug, Clone, Copy)]
pub(in crate::cell) struct GroundStep {
    pub npc_id: u32,
    /// The position the tick just wrote.
    pub pos: Vector3,
    /// The waypoint the leg is heading for.
    pub wp: Vector3,
    /// Where the leg started this tick (the pre-step position).
    pub from: Vector3,
    pub y_source: YSource,
}

/// Check one step against the storey-aware ground. No-op in a space without
/// a navmesh. A `None` ground (no walkable surface within jump height of
/// `y`) is reported as `dir=unknown` — on a path Detour produced, that means
/// the NPC is more than a jump off every floor, not "off the mesh".
pub(in crate::cell) fn check_ground_step(
    space_mgr: &mut SpaceManager,
    step: GroundStep,
    now: Instant,
) {
    let GroundStep {
        npc_id,
        pos,
        wp,
        from,
        y_source,
    } = step;
    if !space_mgr.space_has_navmesh(npc_id) {
        return;
    }
    let ground_y = space_mgr.get_navmesh_height(npc_id, pos.x, pos.y, pos.z);
    let dy = ground_y.map(|g| pos.y - g);
    let dir = match dy {
        Some(d) if d > GROUND_DEVIATION_THRESHOLD => "up",
        Some(d) if d < -GROUND_DEVIATION_THRESHOLD => "down",
        Some(_) => return,
        None => "unknown",
    };
    let Some(ident) = NpcIdent::of(space_mgr, npc_id) else {
        return;
    };
    cimmeria_observability::counter!(
        "npc_ground_deviation_total",
        "world" => ident.world.clone(),
        "dir" => dir,
    );
    let Some(suppressed) = space_mgr.npc_detectors.admit_warn(
        npc_id,
        "ground_deviation",
        now,
        GROUND_DEVIATION_WARN_INTERVAL,
    ) else {
        return;
    };
    let ai_state = space_mgr
        .get_entity(npc_id)
        .map_or("unknown", |e| e.ai_state().label());
    tracing::warn!(
        target: "movement.npc",
        event = "ground_deviation",
        npc_id,
        tag = %ident.tag,
        template_id = ident.template_id,
        world = %ident.world,
        space_id = ident.space_id,
        dir,
        x = pos.x,
        y = pos.y,
        z = pos.z,
        ground_y,
        dy,
        y_source = y_source.label(),
        leg_len = from.distance_to(&wp),
        leg_dy = wp.y - from.y,
        wp_x = wp.x,
        wp_y = wp.y,
        wp_z = wp.z,
        ai_state,
        suppressed,
        "movement.npc: NPC is off the floor under it -- Y was lerped along a \
         leg whose 2D path does not follow the surface"
    );
}
