use cimmeria_entity::stats::StatList;
use cimmeria_wire::state_field::BSF_MOVEMENT_LOCK;

use super::super::super::space_manager::SpaceManager;
use super::npc_ground::{grounded_vertical_speed, grounded_y, YSource, MOVEMENT_TICK_SECS};

/// 1-in-N sampling rate for in-between NPC movement steps. State
/// transitions (waypoint consumed, path complete) are always logged
/// — only the per-tick interpolated position updates are sampled,
/// since those are the high-volume noise. 10 = ~10% of step events.
///
/// The sample is taken over a global step counter, NOT over `npc_id`:
/// the original `npc_id % N` gate logged every step of 10% of NPCs and
/// made the other 90% permanently unobservable.
///
/// Tunable knob: the right rate is "enough to see the motion shape
/// for one NPC over a few seconds, not enough to drown the log
/// stream when 100 NPCs are pathing simultaneously." Bump up
/// (1-in-5) when actively debugging NPC pathing; back off (1-in-50)
/// when the field is quiet.
const NPC_STEP_LOG_SAMPLE: u32 = 10;
/// The first N steps of every leg are always logged: leg starts are where
/// facing and grounding go wrong, and a 1-in-10 sample usually misses them.
const NPC_LEG_HEAD_STEPS: u32 = 5;
static NPC_LEG_STEPS: std::sync::Mutex<Option<std::collections::HashMap<u32, (usize, u32)>>> =
    std::sync::Mutex::new(None);

/// 1-based index of this step within the NPC's current leg. A leg starts when
/// the path gets LONGER than it was last step (a new path was installed);
/// consuming waypoints only ever shortens it.
fn leg_step_index(npc_id: u32, path_len: usize) -> u32 {
    let mut guard = NPC_LEG_STEPS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let map = guard.get_or_insert_with(std::collections::HashMap::new);
    let e = map.entry(npc_id).or_insert((0, 0));
    if path_len > e.0 {
        e.1 = 0;
    }
    e.0 = path_len;
    e.1 += 1;
    e.1
}

static NPC_STEP_LOG_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

use super::super::npc_ai::detectors::movement::{check_ground_step, GroundStep};

/// NA02 `ground_deviation`: compare the Y this tick just wrote against the
/// storey-aware floor under it. Reporting only.
fn check_ground(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    from: cimmeria_common::Vector3,
    pos: cimmeria_common::Vector3,
    wp: cimmeria_common::Vector3,
    y_source: YSource,
) {
    let step = GroundStep {
        npc_id,
        pos,
        wp,
        from,
        y_source,
    };
    check_ground_step(space_mgr, step, std::time::Instant::now());
}

/// Scale an NPC's template `move_speed` (world units per 100ms tick) by its
/// `movementSpeedMod` stat.
///
/// Without the scale the stat had **no** server-side effect at all and a GM's
/// `.speed` (see `cimmeria_cell_console::cell::console::stats::set_speed`)
/// would desync the client's prediction from the authoritative path stepping.
/// See [`StatList::movement_speed_scale`] for the stat's contract and its
/// fallbacks.
fn effective_move_speed(base: f32, stats: &StatList) -> f32 {
    base * stats.movement_speed_scale()
}

/// Below this horizontal distance to its final waypoint an NPC is already
/// on it, and the leg has no heading to face along.
const FINAL_WAYPOINT_FACING_EPSILON: f32 = 0.001;

/// NPC movement along nav paths — runs every AoI tick (100ms) for smooth pathing.
///
/// For each NPC with a non-empty `nav_path`, move it toward the next waypoint
/// by its [`effective_move_speed`] (template `move_speed` scaled by the
/// `movementSpeedMod` stat). When it reaches (or overshoots) a waypoint,
/// consume it and continue to the next. Position updates propagate to
/// witnesses via the AoI tick's `EntityMoved` messages.
pub(in crate::cell::service) fn npc_movement_tick(space_mgr: &mut SpaceManager) {
    // Collect NPCs that have active paths
    let moving_npcs: Vec<u32> = space_mgr
        .ai_driven_npc_entity_ids()
        .iter()
        .filter(|&&eid| {
            space_mgr
                .get_entity(eid)
                .is_some_and(|e| !e.nav_path.is_empty())
        })
        .copied()
        .collect();

    for npc_id in moving_npcs {
        // Stunned or knocked down (`BSF_MovementLock`, a timed-effect
        // entry, ability mechanics AB-09a): stand still and keep the route
        // for when the lock clears. The AoI tick resends velocity every
        // 100 ms, so zero it or witnesses see the NPC run in place.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            if npc.has_state_flag(BSF_MOVEMENT_LOCK) {
                npc.velocity = [0.0; 3];
                continue;
            }
        }
        // Read the next waypoint, move_speed, and remaining path length
        let (next_wp, move_speed, cur_pos, path_len) = {
            let npc = match space_mgr.get_entity(npc_id) {
                Some(e) if !e.nav_path.is_empty() => e,
                _ => continue,
            };
            let next_wp = match npc.nav_path.front() {
                Some(wp) => *wp,
                None => continue,
            };
            (
                next_wp,
                effective_move_speed(npc.move_speed, &npc.stats),
                npc.position,
                npc.nav_path.len(),
            )
        };

        let dx = next_wp.x - cur_pos.x;
        let dy = next_wp.y - cur_pos.y;
        let dz = next_wp.z - cur_pos.z;
        // Step budget is horizontal. Y now follows the floor rather than the
        // chord, so the chord's dy is not distance the NPC walks, and on a
        // floor-then-ramp leg it used to eat budget over the flat part and
        // slow the NPC for no visible reason. A waypoint straight overhead
        // (a corner on the next storey up) reads as arrived and snaps.
        let dist = (dx * dx + dz * dz).sqrt();

        // Speed in world units per second (tick is 100ms = 0.1s)
        let speed_per_sec = move_speed / MOVEMENT_TICK_SECS;

        if dist <= move_speed {
            // Reached (or overshot) the waypoint — snap to it and consume.
            // Only the first and last corners of a straight path are
            // detail-surface points; intermediate corners are poly-mesh
            // portal vertices, so ground the snap too.
            let (snap_y, snap_source) =
                grounded_y(space_mgr, npc_id, next_wp.x, next_wp.y, next_wp.z);
            let vy = grounded_vertical_speed(cur_pos.y, snap_y, speed_per_sec);

            // Peek at the NEXT waypoint (index 1) to compute velocity toward it
            let next_next_wp = if path_len > 1 {
                space_mgr
                    .get_entity(npc_id)
                    .and_then(|e| e.nav_path.get(1).copied())
            } else {
                None
            };

            let (velocity, yaw) = if let Some(nn) = next_next_wp {
                // Still more waypoints — compute velocity toward the next one
                let ndx = nn.x - next_wp.x;
                let ndz = nn.z - next_wp.z;
                let nd = (ndx * ndx + ndz * ndz).sqrt();
                if nd > 0.001 {
                    (
                        [ndx / nd * speed_per_sec, vy, ndz / nd * speed_per_sec],
                        ndx.atan2(ndz),
                    )
                } else {
                    // Coincident waypoints: no heading to derive. Keep the
                    // current facing -- 0.0 here snapped the NPC to north.
                    let keep = space_mgr.get_entity(npc_id).map_or(0.0, |e| e.direction.y);
                    ([0.0; 3], keep)
                }
            } else if dist > FINAL_WAYPOINT_FACING_EPSILON {
                // Last waypoint — stopping, facing the way the last leg went.
                ([0.0; 3], dx.atan2(dz))
            } else {
                // Last waypoint, already standing on it: the leg has no
                // heading. Keep the current facing -- atan2(0, 0) = 0 turned
                // the NPC to due north on arrival (NA41).
                let keep = space_mgr.get_entity(npc_id).map_or(0.0, |e| e.direction.y);
                ([0.0; 3], keep)
            };

            space_mgr.update_entity_position(
                npc_id,
                [next_wp.x, snap_y, next_wp.z],
                [0, 0, 0],
                velocity,
            );
            // Compare the Y actually written, not the waypoint's poly-mesh Y.
            let snapped = cimmeria_common::Vector3::new(next_wp.x, snap_y, next_wp.z);
            check_ground(space_mgr, npc_id, cur_pos, snapped, next_wp, snap_source);
            let remaining_after = if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.nav_path.pop_front();
                npc.direction = cimmeria_common::Vector3::new(0.0, yaw, 0.0);
                npc.nav_path.len()
            } else {
                0
            };
            // Always-log: NPC reached a waypoint. State transitions are
            // low-volume (once per waypoint, not per tick) and the most
            // diagnostic event for "NPC pathing looks wrong" bug
            // reports. `path_complete = remaining_after == 0` is the
            // signal SigNoz operators look for when checking
            // "did the NPC actually finish its path?"
            tracing::debug!(
                target: "movement.npc",
                event = "waypoint_reached",
                npc_id,
                wp_x = next_wp.x,
                wp_y = next_wp.y,
                wp_z = next_wp.z,
                remaining_waypoints = remaining_after,
                path_complete = (remaining_after == 0),
                "NPC reached waypoint"
            );
        } else {
            // Move toward waypoint by move_speed units
            let t = move_speed / dist;
            let new_x = cur_pos.x + dx * t;
            let new_z = cur_pos.z + dz * t;

            // The chord lerp does NOT stay near the floor: Detour corners
            // are XZ turns only, so a floor-then-ramp leg is one segment
            // and its lerp floats over the flat part (audit M1). The lerp
            // only picks the storey; the floor under the step is the Y.
            let lerp_y = cur_pos.y + dy * t;
            let (new_y, y_source) = grounded_y(space_mgr, npc_id, new_x, lerp_y, new_z);

            // Face the direction of movement (yaw = atan2(dx, dz) in radians)
            // Direction is [pitch, yaw, roll] — only yaw matters for facing
            let yaw = dx.atan2(dz);

            // Horizontal speed along the leg; vertical from the grounded
            // rise, so the client's filter does not extrapolate the chord.
            let velocity = [
                dx / dist * speed_per_sec,
                grounded_vertical_speed(cur_pos.y, new_y, speed_per_sec),
                dz / dist * speed_per_sec,
            ];

            // Sampled per-tick movement log. Stable `target:
            // "movement.npc"` so SigNoz can filter by the canonical
            // movement view (and the operator can dial the sampling
            // by tuning `NPC_STEP_LOG_SAMPLE`). State transitions
            // above are always-on; only these interpolated steps
            // are sampled.
            let leg_step = leg_step_index(
                npc_id,
                space_mgr.get_entity(npc_id).map_or(0, |e| e.nav_path.len()),
            );
            let sampled = NPC_STEP_LOG_COUNTER
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                .is_multiple_of(NPC_STEP_LOG_SAMPLE);
            if leg_step <= NPC_LEG_HEAD_STEPS || sampled {
                // The clamp's own reading; `None` = the step kept the lerp.
                let ground_y = (y_source == YSource::Clamp).then_some(new_y);
                tracing::debug!(
                    target: "movement.npc",
                    event = "step",
                    npc_id,
                    cur_x = cur_pos.x, cur_y = cur_pos.y, cur_z = cur_pos.z,
                    new_x, new_y, new_z,
                    wp_x = next_wp.x, wp_y = next_wp.y, wp_z = next_wp.z,
                    dist_remaining = dist - move_speed,
                    yaw_rad = yaw,
                    yaw_byte = crate::mercury::aoi::pack_angle(yaw),
                    leg_step,
                    y_source = y_source.label(),
                    lerp_y,
                    ?ground_y,
                    y_offset_from_ground = ?ground_y.map(|g| new_y - g),
                    "NPC movement step (sampled)"
                );
            }

            space_mgr.update_entity_position(npc_id, [new_x, new_y, new_z], [0, 0, 0], velocity);
            let stepped = cimmeria_common::Vector3::new(new_x, new_y, new_z);
            check_ground(space_mgr, npc_id, cur_pos, stepped, next_wp, y_source);
            // Set yaw directly as radians (pack_angle reads direction.y)
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.direction = cimmeria_common::Vector3::new(0.0, yaw, 0.0);
            }
        }
    }
}

#[cfg(test)]
#[path = "npc_movement_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "npc_movement_cc_tests.rs"]
mod cc_tests;
