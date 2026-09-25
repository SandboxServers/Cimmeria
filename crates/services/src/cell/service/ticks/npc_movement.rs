use cimmeria_entity::stats::StatList;

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
/// `.speed` (see [`crate::cell::console::stats::set_speed`]) would desync the
/// client's prediction from the authoritative path stepping. See
/// [`StatList::movement_speed_scale`] for the stat's contract and its
/// fallbacks.
fn effective_move_speed(base: f32, stats: &StatList) -> f32 {
    base * stats.movement_speed_scale()
}

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
            } else {
                // Last waypoint — stopping, keep current facing
                ([0.0; 3], dx.atan2(dz))
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
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use cimmeria_entity::stats::MOVEMENT_SPEED_MOD;

    #[test]
    fn npc_movement_tick_advances_along_nav_path() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(200, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(200) {
            npc.is_player = false;
            npc.class_id = 0x04;
            npc.move_speed = 5.0;
            npc.nav_path
                .push_back(cimmeria_common::Vector3::new(10.0, 0.0, 0.0));
        }

        npc_movement_tick(&mut mgr);

        let npc = mgr.get_entity(200).unwrap();
        assert_eq!(npc.position.x, 5.0);
        assert_eq!(npc.position.y, 0.0);
        assert_eq!(npc.position.z, 0.0);
        assert_eq!(npc.nav_path.len(), 1);
    }

    #[test]
    fn npc_movement_tick_does_not_panic_on_empty_path() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(200, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(200) {
            npc.is_player = false;
            npc.class_id = 0x04;
            npc.nav_path.clear();
        }
        // Must not panic.
        npc_movement_tick(&mut mgr);
        let npc = mgr.get_entity(200).unwrap();
        assert_eq!(npc.position.x, 0.0, "stationary NPC must not move");
    }

    #[test]
    fn npc_snaps_to_waypoint_when_within_move_speed_and_advances() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(200, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(200) {
            npc.is_player = false;
            npc.class_id = 0x04;
            npc.move_speed = 10.0; // larger than distance to first waypoint
            npc.nav_path
                .push_back(cimmeria_common::Vector3::new(3.0, 0.0, 4.0)); // dist = 5
            npc.nav_path
                .push_back(cimmeria_common::Vector3::new(20.0, 0.0, 0.0));
        }

        npc_movement_tick(&mut mgr);

        let npc = mgr.get_entity(200).unwrap();
        assert_eq!(npc.position.x, 3.0, "must snap to first waypoint X");
        assert_eq!(npc.position.y, 0.0, "must snap to first waypoint Y");
        assert_eq!(npc.position.z, 4.0, "must snap to first waypoint Z");
        assert_eq!(
            npc.nav_path.len(),
            1,
            "first waypoint consumed, second remains"
        );
    }

    #[test]
    fn npc_stops_at_final_waypoint() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(200, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(200) {
            npc.is_player = false;
            npc.class_id = 0x04;
            npc.move_speed = 20.0; // overshoots the only waypoint
            npc.nav_path
                .push_back(cimmeria_common::Vector3::new(5.0, 0.0, 0.0));
        }

        npc_movement_tick(&mut mgr);

        let npc = mgr.get_entity(200).unwrap();
        assert_eq!(npc.position.x, 5.0, "must snap to final waypoint X");
        assert_eq!(npc.position.y, 0.0, "must snap to final waypoint Y");
        assert_eq!(npc.position.z, 0.0, "must snap to final waypoint Z");
        assert!(
            npc.nav_path.is_empty(),
            "path must be empty after reaching final waypoint"
        );
    }

    // ── P47 (`.speed`) — tick-level movement effect ─────────────────────────
    //
    // The stat-level half of P47 lives in
    // `crate::cell::console::tests::p47`; these prove the setter's effect
    // reaches real tick behavior rather than stopping at the stat value.
    // `cargo test --lib legacy_p47_` runs both halves.

    /// Build a GM caller + a pathing NPC in one space. The NPC starts at the
    /// origin with a single waypoint 80 units down +X and `move_speed = 5.0`,
    /// so every per-tick step in these tests (`t = speed / 80`) lands on an
    /// exact binary fraction and the position assertions can be exact `f32`
    /// equality rather than an epsilon compare.
    fn speed_fixture() -> (SpaceManager, u32, u32) {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();

        let gm = 1u32;
        mgr.create_entity(gm, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(gm) {
            e.is_player = true;
            e.access_level = 2;
        }

        let npc = 200u32;
        mgr.create_entity(npc, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(npc) {
            e.is_player = false;
            e.class_id = 0x04;
            e.move_speed = 5.0;
            e.nav_path
                .push_back(cimmeria_common::Vector3::new(80.0, 0.0, 0.0));
        }
        (mgr, gm, npc)
    }

    /// Run `.speed <arg>` through the real console dispatch, then one movement
    /// tick, and report how far along +X the NPC actually got.
    async fn speed_then_tick(arg: &str) -> f32 {
        let (mut mgr, gm, npc) = speed_fixture();
        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(64);
        crate::cell::console::exec("speed", gm, &[arg], Some(npc), &tx, &mut mgr, &engine).await;
        npc_movement_tick(&mut mgr);
        mgr.get_entity(npc).unwrap().position.x
    }

    /// The acceptance criterion: `.speed` changes how far the NPC actually
    /// moves on the next tick, not merely what the stat reads back.
    /// `move_speed = 5.0` at the default mod of 100 steps 5.0 units; `.speed
    /// 200` must step exactly 10.0 and `.speed 50` exactly 2.5.
    ///
    /// Reverting `effective_move_speed` back to a bare `npc.move_speed` makes
    /// every non-100 row collapse onto 5.0 and fails here.
    #[tokio::test]
    async fn legacy_p47_speed_scales_the_npc_tick_step_proportionally() {
        for (arg, expected) in [("50", 2.5f32), ("100", 5.0), ("200", 10.0), ("400", 20.0)] {
            let got = speed_then_tick(arg).await;
            assert_eq!(
                got, expected,
                ".speed {arg} must move the NPC exactly {expected} units on the next tick"
            );
        }
    }

    /// `.speed 0` freezes the NPC in place: no displacement, and the waypoint
    /// is NOT consumed (a zero-length step must not be mistaken for "reached
    /// the waypoint" — `dist <= move_speed` would be `80.0 <= 0.0`, false).
    #[tokio::test]
    async fn legacy_p47_speed_zero_freezes_the_npc_without_consuming_its_path() {
        let (mut mgr, gm, npc) = speed_fixture();
        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(64);
        crate::cell::console::exec("speed", gm, &["0"], Some(npc), &tx, &mut mgr, &engine).await;

        npc_movement_tick(&mut mgr);
        npc_movement_tick(&mut mgr);

        let e = mgr.get_entity(npc).unwrap();
        assert_eq!(e.position.x, 0.0, ".speed 0 must stop the NPC moving");
        assert_eq!(e.position.z, 0.0, ".speed 0 must stop the NPC moving");
        assert_eq!(
            e.nav_path.len(),
            1,
            "a frozen NPC must not consume waypoints"
        );
    }

    /// The effect accumulates across ticks rather than applying once: two
    /// ticks at `.speed 200` cover exactly 20.0 units.
    #[tokio::test]
    async fn legacy_p47_speed_effect_persists_across_ticks() {
        let (mut mgr, gm, npc) = speed_fixture();
        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(64);
        crate::cell::console::exec("speed", gm, &["200"], Some(npc), &tx, &mut mgr, &engine).await;

        npc_movement_tick(&mut mgr);
        assert_eq!(mgr.get_entity(npc).unwrap().position.x, 10.0);
        npc_movement_tick(&mut mgr);
        assert_eq!(
            mgr.get_entity(npc).unwrap().position.x,
            20.0,
            "the speed mod must apply on every subsequent tick, not just the first"
        );
    }

    // ── GC1b-0 — `entity_templates.move_speed` DB wiring ─────────────────────
    //
    // The tests above pin the per-tick math from a raw `npc.move_speed`
    // field write. This one closes the loop from the other end: a
    // `SpawnRecord.move_speed` value (what `load_spawns_from_db` reads
    // off `entity_templates.move_speed`, COALESCEd to the historical
    // 0.6 default) must actually reach `CellEntity.move_speed` via
    // `spawn_npc_from_record`, and a template that opts into a faster
    // pace (e.g. an escort NPC, ~0.9/tick per the GC1b-0 feasibility
    // pass) must move measurably farther per tick than the 0.6 default
    // — not just carry a different number that nothing reads.

    /// Build a minimal `SpawnRecord` for the movement-speed wiring test.
    /// Field values mirror `spawner::tests::spawn_records::make_test_record`
    /// (this module can't reach that private test helper across the
    /// `spawner`/`service` module boundary, so it's duplicated narrowly).
    fn make_spawn_record(move_speed: f32) -> crate::cell::spawner::SpawnRecord {
        crate::cell::spawner::SpawnRecord {
            spawn_id: 1,
            world_name: "Castle".to_string(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            heading: 0.0,
            tag: None,
            template_id: 10,
            template_name: "Test Escort".to_string(),
            // Must be "mob" (class_id 0x04) -- `npc_movement_tick` sources
            // its candidate set from `ai_driven_npc_entity_ids`, which admits
            // a "being" (0x01) only in a behaviour state such as Follow
            // (NA24). This fixture stays Idle, so a being would be excluded
            // from the tick and both assertions would read 0.0.
            class: "mob".to_string(),
            static_mesh: None,
            body_set: "GLB_Components.WorldObject_Small".to_string(),
            components: None,
            flags: 0,
            interaction_type: 0,
            event_set_id: None,
            level: Some(1),
            alignment: Some(0),
            faction: Some(1),
            name_id: None,
            speaker_id: None,
            static_interaction_sets: vec![],
            has_dynamic_properties: true,
            loot_table_id: None,
            is_stationary: false,
            ability_ids: vec![],
            respawn_secs: None,
            patrol_path: vec![],
            patrol_point_delay_secs: 2.0,
            wander_radius: 0.0,
            wander_min_dwell_secs: 3.0,
            wander_max_dwell_secs: 8.0,
            follow_min_distance: 2.0,
            follow_max_distance: 5.0,
            move_speed,
            leash_distance: None,
            aggro_radius: None,
            assist_radius: None,
            aggression_override: None,
            use_cover: None,
        }
    }

    /// A template `move_speed` of 0.9 (GC1b-0's suggested escort speed) must
    /// move an NPC farther per tick than the 0.6 historical default — proving
    /// the DB column actually changes effective NPC speed, not just that it
    /// round-trips through `SpawnRecord`. 0.6 and 0.9 are chosen to match
    /// `construction.rs`'s hardcoded default and Marsh's seeded template
    /// value exactly, so the assertions are exact `f32` equality.
    #[test]
    fn spawn_record_move_speed_produces_proportionally_faster_movement() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();

        let default_npc = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(default_npc, &make_spawn_record(0.6))
            .unwrap();
        let escort_npc = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(escort_npc, &make_spawn_record(0.9))
            .unwrap();

        assert_eq!(
            mgr.get_entity(default_npc).unwrap().move_speed,
            0.6,
            "SpawnRecord.move_speed=0.6 must land on CellEntity.move_speed unchanged"
        );
        assert_eq!(
            mgr.get_entity(escort_npc).unwrap().move_speed,
            0.9,
            "SpawnRecord.move_speed=0.9 must land on CellEntity.move_speed unchanged"
        );

        for npc_id in [default_npc, escort_npc] {
            if let Some(e) = mgr.get_entity_mut(npc_id) {
                e.nav_path
                    .push_back(cimmeria_common::Vector3::new(100.0, 0.0, 0.0));
            }
        }

        npc_movement_tick(&mut mgr);

        let default_x = mgr.get_entity(default_npc).unwrap().position.x;
        let escort_x = mgr.get_entity(escort_npc).unwrap().position.x;
        assert_eq!(
            default_x, 0.6,
            "the 0.6 default must move exactly 0.6 units in one tick"
        );
        assert_eq!(
            escort_x, 0.9,
            "the 0.9 escort-speed template must move exactly 0.9 units in \
             one tick -- 50% farther than the 0.6 default per tick"
        );
        assert!(
            escort_x > default_x,
            "a higher template move_speed must produce more per-tick \
             movement than the default -- the DB column must actually \
             change effective NPC speed"
        );
    }

    /// Guards `effective_move_speed`'s two defensive branches directly.
    ///
    /// The absent-stat fallback can't be reached through a real `StatList`:
    /// `StatList::new()` always installs `movementSpeedMod` and exposes no
    /// public way to remove an entry (same constraint `stats.rs`'s
    /// `format_stat_line` documents for its `None` arm), so only the
    /// default-mod and negative-`cur` branches are exercised here. A negative
    /// `cur` is itself only reachable by poking the field directly — the
    /// `.speed` path rejects out-of-range values and `Stat::set_current`
    /// clamps — but the floor is what keeps a stray write from driving an NPC
    /// backwards along its own path.
    #[test]
    fn legacy_p47_effective_move_speed_fallbacks_are_safe() {
        let mut stats = cimmeria_entity::stats::StatList::new();
        assert_eq!(
            effective_move_speed(5.0, &stats),
            5.0,
            "the default mod of 100 must leave move_speed unscaled"
        );

        stats.get_mut(MOVEMENT_SPEED_MOD).unwrap().cur = -250;
        assert_eq!(
            effective_move_speed(5.0, &stats),
            0.0,
            "a negative mod must stall the NPC, never reverse it"
        );
    }
}

#[cfg(test)]
mod leg_step_tests {
    use super::leg_step_index;

    /// A leg starts when the path gets longer; consuming waypoints never
    /// restarts the count.
    #[test]
    fn leg_step_restarts_only_when_a_new_path_is_installed() {
        let npc = 4_200_001;
        assert_eq!(leg_step_index(npc, 3), 1);
        assert_eq!(leg_step_index(npc, 3), 2);
        assert_eq!(leg_step_index(npc, 2), 3, "waypoint consumed: same leg");
        assert_eq!(leg_step_index(npc, 1), 4);
        assert_eq!(leg_step_index(npc, 4), 1, "longer path = new leg");
        assert_eq!(leg_step_index(npc + 1, 1), 1, "per NPC");
    }
}
