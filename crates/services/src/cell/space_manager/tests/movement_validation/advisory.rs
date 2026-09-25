//! The `advisory` navmesh mode at the inbound-position seam, against the
//! real `harset.nav`.
//!
//! `harset.nav` is the reason the mode exists, so these run on it rather
//! than on a synthetic mesh: the defect is not "a mesh can have a hole", it
//! is "the shipped Harset mesh has a hole across the only walk to the
//! Command Center door, and an ordinary player is snapped back at it while
//! a GM (warn-only) walks through and never reports it".
//!
//! Every test self-skips when the fixture is absent, per the repo's
//! navmesh-test pattern, and every test carries the control assertions that
//! make its verdict mean something — that the mesh actually loaded, and
//! that the "on-mesh" and "off-mesh" fixture points really are what the
//! test calls them. Without those a mesh that failed to load would make the
//! advisory half vacuously green.

use std::collections::HashMap;
use std::time::Instant;

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::{MovementReject, MovementValidator};
use cimmeria_entity::navigation::NavMesh;

use super::super::super::{ClientMoveOutcome, NavmeshMode, SpaceManager};
use crate::cell::spawner::WorldRow;

/// `resources.worlds.world_id` for Harset.
const HARSET_WORLD_ID: i32 = 57;

/// On the plaza floor, 2 units inside the last covered row of the walk
/// toward the Command Center. Measured 2026-09-19 on a 2-unit grid: at
/// Y -68 the mesh covers X -24..0 down to Z -198 and nothing from Z -200
/// to Z -228.
const ON_MESH: [f32; 3] = [-4.0, -68.0, -196.0];

/// Eight units further along the same walk, inside the hole. A single
/// ordinary walking step — well under `TELEPORT_JUMP_UNITS` — so nothing
/// but layer 4 can object to it.
const IN_THE_HOLE: [f32; 3] = [-4.0, -68.0, -204.0];

/// Ring region 4's pad row, a coordinate the shipped seed already stands a
/// player on. The live control that the mesh loaded and answers `true` for
/// something.
const RING_PAD_4: [f32; 3] = [-25.641, -67.828, 15.249];

fn harset_mesh() -> Option<NavMesh> {
    let path = std::path::Path::new("../../data/spaces/harset.nav");
    if !path.exists() {
        return None; // fixture-less checkout: skip
    }
    Some(NavMesh::load(path).expect("load data/spaces/harset.nav"))
}

/// A manager whose only world is a non-instanced Harset, so the startup
/// space lands in `world_spaces` and `create_entity` can find it.
///
/// Built here rather than by extending the module-wide `make_manager`
/// fixture: that one is shared with the space-lifecycle tests, which pin
/// `space_count()` and the exact `(cell_id << 16) | index` ids, and adding a
/// third startup world to it moves every one of those numbers. The AABB is
/// the real `entities/spaces.xml` row for Harset, but it is not load-bearing
/// — a meshed space takes its bounds from the mesh extents.
fn harset_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Harset" Instanced="false" MinX="-1000" MaxX="800" MinY="-800" MaxY="800" />
</Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Harset" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

/// A manager with a player standing on [`ON_MESH`] in a Harset space
/// carrying the real mesh, stamped to `mode`.
///
/// Returns `None` when the fixture is absent so callers skip in one line.
fn harset_with_player(mode: NavmeshMode) -> Option<(SpaceManager, u32)> {
    let mesh = harset_mesh()?;
    let mut mgr = harset_manager();
    mgr.stamp_world_rows(&HashMap::from([(
        "Harset".to_string(),
        WorldRow {
            world_id: HARSET_WORLD_ID,
            navmesh_mode: mode,
        },
    )]));
    let space_id = mgr
        .create_entity(100, "Harset", ON_MESH, [0.0; 3])
        .expect("Harset startup space must exist");
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(mesh);
    Some((mgr, space_id))
}

/// Assert the three fixture facts every verdict below rests on.
fn assert_fixture_controls(mgr: &SpaceManager) {
    let v = |p: [f32; 3]| Vector3::new(p[0], p[1], p[2]);
    assert!(
        mgr.is_position_valid(100, &v(RING_PAD_4)),
        "control: ring pad 4 must read on-mesh. If this fails the mesh did \
         not load (or did not get injected) and every verdict below is \
         meaningless — the advisory half would pass for the wrong reason",
    );
    assert!(
        mgr.is_position_valid(100, &v(ON_MESH)),
        "control: {ON_MESH:?} must read on-mesh — it is the player's start \
         and the snap-back target the enforce half expects",
    );
    assert!(
        !mgr.is_position_valid(100, &v(IN_THE_HOLE)),
        "control: {IN_THE_HOLE:?} must read OFF-mesh. This is the whole \
         premise: it is a single walking step along the plaza floor and the \
         mesh does not cover it. `is_position_valid` keeps answering \
         truthfully in advisory worlds, so this assertion holds in both modes",
    );
}

/// **The P0.** One ordinary walking step across the measured hole is
/// accepted in `advisory` and written through to the cell entity.
///
/// Reverting `enforces_navmesh_containment` to "a mesh is loaded" makes
/// this a `Rejected { OffNavmesh }` — which is exactly the live bug: no
/// non-GM character can walk from the Harset gate plaza to the Command
/// Center door.
#[test]
fn a_step_into_a_mesh_hole_is_accepted_in_an_advisory_world() {
    let Some((mut mgr, space_id)) = harset_with_player(NavmeshMode::Advisory) else {
        return;
    };
    assert_fixture_controls(&mgr);

    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, IN_THE_HOLE, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == IN_THE_HOLE),
        "an advisory world must not gate on its own mesh — got {outcome:?}",
    );
    assert_eq!(
        mgr.spaces[&space_id].entities[&100].position,
        Vector3::new(IN_THE_HOLE[0], IN_THE_HOLE[1], IN_THE_HOLE[2]),
        "an accepted position must be written to the cell entity, or \
         witnesses keep seeing the player at the hole boundary",
    );
}

/// The control half of the pair: the *same* step, the *same* mesh, under
/// `enforce` is rejected `OffNavmesh` and never written.
///
/// This is what proves [`IN_THE_HOLE`] is genuinely off-mesh and that the
/// accept above came from the mode and not from a mesh that answers `true`
/// everywhere.
#[test]
fn the_same_step_is_rejected_off_navmesh_in_an_enforcing_world() {
    let Some((mut mgr, space_id)) = harset_with_player(NavmeshMode::Enforce) else {
        return;
    };
    assert_fixture_controls(&mgr);

    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, IN_THE_HOLE, [0, 0, 0], [0.0; 3]);
    match outcome {
        ClientMoveOutcome::Rejected {
            reason, last_valid, ..
        } => {
            assert_eq!(reason, MovementReject::OffNavmesh);
            assert_eq!(
                last_valid, ON_MESH,
                "the snap-back target is the player's own on-mesh position",
            );
        }
        other => panic!("expected Rejected(OffNavmesh) under enforce, got {other:?}"),
    }
    assert_eq!(
        mgr.spaces[&space_id].entities[&100].position,
        Vector3::new(ON_MESH[0], ON_MESH[1], ON_MESH[2]),
        "a rejected position must never be written",
    );
}

/// Advisory demotes **only** layer 4. The bounds layer still hard-rejects,
/// so an advisory world is not an unvalidated world.
///
/// A coordinate far outside the mesh's own AABB — which is where the
/// bounds come from on a meshed space — is the shape a corrupt or spoofed
/// packet has.
#[test]
fn an_out_of_bounds_move_is_still_rejected_in_an_advisory_world() {
    let Some((mut mgr, space_id)) = harset_with_player(NavmeshMode::Advisory) else {
        return;
    };
    let nav_max_x = mgr.spaces[&space_id].navmesh.as_ref().unwrap().bmax[0];
    let outside = [nav_max_x + 5_000.0, -68.0, -196.0];

    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, outside, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::OutOfBounds,
                ..
            }
        ),
        "advisory relaxes navmesh containment only — the AABB still holds; \
         got {outcome:?}",
    );
    assert_eq!(
        mgr.spaces[&space_id].entities[&100].position,
        Vector3::new(ON_MESH[0], ON_MESH[1], ON_MESH[2]),
    );
}

/// Same for the teleport layer. A first-packet jump further than
/// `TELEPORT_JUMP_UNITS` is rejected in an advisory world, and rejected as
/// `Teleport` — not as `OffNavmesh`, which would mean layer 4 had caught it
/// first and the teleport gate was never reached.
///
/// The destination is chosen on-mesh so the only layer that can object is
/// the kinematic one.
#[test]
fn a_teleport_sized_jump_is_still_rejected_in_an_advisory_world() {
    let Some((mut mgr, space_id)) = harset_with_player(NavmeshMode::Advisory) else {
        return;
    };
    let far = RING_PAD_4;
    let jump = Vector3::new(far[0], far[1], far[2])
        .distance_to(&Vector3::new(ON_MESH[0], ON_MESH[1], ON_MESH[2]));
    assert!(
        jump > MovementValidator::TELEPORT_JUMP_UNITS,
        "control: the fixture jump ({jump}) must exceed the teleport \
         threshold or this test proves nothing",
    );

    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, far, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::Teleport,
                ..
            }
        ),
        "advisory must not disarm the teleport gate; got {outcome:?}",
    );
    assert_eq!(
        mgr.spaces[&space_id].entities[&100].position,
        Vector3::new(ON_MESH[0], ON_MESH[1], ON_MESH[2]),
    );
}

/// A player already standing in a hole is a *sound* snap-back target in an
/// advisory world, so an unrelated hard reject corrects them back to where
/// they are rather than relocating them onto the nearest polygon.
///
/// This is the back door the `reject_outcome` half of the change closes.
/// With only layer 4 routed through the predicate, the first out-of-bounds
/// packet from a player standing in the Command-Center corridor would judge
/// their own position unusable and teleport them onto whatever the mesh's
/// nearest polygon happens to be — on `harset.nav`, a different floor.
#[test]
fn a_player_standing_in_a_hole_is_corrected_back_not_relocated() {
    let Some((mut mgr, space_id)) = harset_with_player(NavmeshMode::Advisory) else {
        return;
    };
    assert_fixture_controls(&mgr);

    // Walk into the hole first — accepted, per the P0 test above.
    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, IN_THE_HOLE, [0, 0, 0], [0.0; 3]);
    assert!(matches!(outcome, ClientMoveOutcome::Accepted { .. }));

    // Now send something the bounds layer rejects, from that off-mesh spot.
    let nav_max_x = mgr.spaces[&space_id].navmesh.as_ref().unwrap().bmax[0];
    let outside = [nav_max_x + 5_000.0, -68.0, -204.0];
    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, outside, [0, 0, 0], [0.0; 3]);

    match outcome {
        ClientMoveOutcome::Rejected { last_valid, .. } => assert_eq!(
            last_valid, IN_THE_HOLE,
            "the correction must point at the player's own position",
        ),
        other => panic!(
            "expected an ordinary Rejected correction in an advisory world, \
             got {other:?} — a Recovered here means the off-mesh position was \
             judged unusable and the player was teleported onto the mesh"
        ),
    }
    assert_eq!(
        mgr.spaces[&space_id].entities[&100].position,
        Vector3::new(IN_THE_HOLE[0], IN_THE_HOLE[1], IN_THE_HOLE[2]),
        "the player must not have been relocated",
    );
}

/// The startup summary must count the world's off-mesh spawn rows
/// correctly, and must name the mode.
///
/// This line is the only automated signal that a mesh which loads cleanly
/// describes a different map from the one it is named after — a number that
/// is silently wrong is worse than no line, because it is the number an
/// operator would use to decide a rebake worked. Two rows, one on the mesh
/// and one in the measured hole, so the count can only be right for the
/// right reason.
#[test]
fn the_startup_summary_counts_off_mesh_spawn_rows() {
    let Some(mesh) = harset_mesh() else { return };
    let mut mgr = harset_manager();
    let space_id = *mgr.world_spaces.get("Harset").unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(mesh);
    mgr.stamp_world_rows(&HashMap::from([(
        "Harset".to_string(),
        WorldRow {
            world_id: HARSET_WORLD_ID,
            navmesh_mode: NavmeshMode::Advisory,
        },
    )]));

    let rows = [
        spawn_row("Harset", ON_MESH),
        spawn_row("Harset", IN_THE_HOLE),
        // A different world's row must not be counted against Harset.
        spawn_row("Agnos", IN_THE_HOLE),
    ];

    let capture = crate::test_support::LogCapture::install();
    mgr.log_navmesh_summary(&rows);
    let event = capture
        .find_event(
            tracing::Level::INFO,
            "mesh resident for this world",
            "navmesh_mode_summary",
        )
        .expect("the startup summary must emit one INFO per meshed world");

    assert!(event.has_field("world_name", "Harset"));
    assert!(
        event.has_field("navmesh_mode", "advisory"),
        "the line must name the mode, or it cannot answer \
         'did the seed load' -- which is the first thing an operator checks",
    );
    assert!(
        event.has_field("spawn_rows", "2"),
        "only this world's rows are counted",
    );
    assert!(
        event.has_field("spawn_rows_off_mesh", "1"),
        "exactly the row in the measured hole is off-mesh; a count that \
         cannot distinguish the two rows cannot tell an operator whether a \
         rebake worked",
    );
}

/// Minimal `SpawnRecord` for the summary test — only `world_name` and the
/// coordinate are read by it.
fn spawn_row(world: &str, pos: [f32; 3]) -> crate::cell::spawner::SpawnRecord {
    crate::cell::spawner::SpawnRecord {
        spawn_id: -1,
        world_name: world.to_string(),
        x: pos[0],
        y: pos[1],
        z: pos[2],
        heading: 0.0,
        tag: None,
        template_id: 1,
        template_name: "H53 fixture".to_string(),
        class: "mob".to_string(),
        static_mesh: None,
        body_set: "BS_HumanMale.BS_HumanMale".to_string(),
        components: None,
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: None,
        alignment: None,
        faction: None,
        name_id: None,
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: false,
        loot_table_id: None,
        is_stationary: true,
        ability_ids: vec![],
        respawn_secs: None,
        patrol_path: vec![],
        patrol_point_delay_secs: 2.0,
        wander_radius: 0.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.6,
        leash_distance: None,
        aggro_radius: None,
        assist_radius: None,
        aggression_override: None,
        use_cover: None,
    }
}

/// A GM is unaffected in an advisory world: they were already allowed off
/// the mesh, and the mode must not accidentally turn the GM allowance into
/// something narrower.
#[test]
fn a_gm_still_moves_freely_in_an_advisory_world() {
    let Some((mut mgr, _space_id)) = harset_with_player(NavmeshMode::Advisory) else {
        return;
    };
    mgr.get_entity_mut(100).unwrap().access_level =
        cimmeria_commands::permissions::AccessLevel::GameMaster as u32;

    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, IN_THE_HOLE, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { .. }),
        "got {outcome:?}",
    );
}

/// NA25: `movement.navmesh` `advisory_off_mesh_accepted` is the record of
/// which parts of an advisory world's mesh players actually walk. Until the
/// `cimmeria-trace` index existed no layer enabled it, so it never fired;
/// now it does, on every off-mesh packet, and it must be throttled per
/// player with the skipped packets counted.
///
/// Removing the `advisory_off_mesh_log.admit` gate makes the burst write
/// five rows and fails the first assertion.
#[test]
fn advisory_off_mesh_row_fires_throttled_with_a_suppressed_count() {
    let Some((mut mgr, _space_id)) = harset_with_player(NavmeshMode::Advisory) else {
        return;
    };
    assert_fixture_controls(&mgr);
    let capture = crate::test_support::LogCapture::install();
    let rows = |c: &crate::test_support::LogCaptureGuard| {
        c.all()
            .into_iter()
            .filter(|e| {
                e.target == "movement.navmesh"
                    && e.has_field("reason", "advisory_off_mesh_accepted")
            })
            .collect::<Vec<_>>()
    };

    // Five packets in the hole inside one 500 ms window.
    let t0 = Instant::now();
    for i in 0..5u64 {
        let outcome = mgr.apply_client_position_update_at(
            t0 + std::time::Duration::from_millis(i * 100),
            100,
            IN_THE_HOLE,
            [0, 0, 0],
            [0.0; 3],
        );
        assert!(matches!(outcome, ClientMoveOutcome::Accepted { .. }));
    }
    let burst = rows(&capture);
    assert_eq!(
        burst.len(),
        1,
        "one row per window, not one per packet: {burst:#?}"
    );
    assert_eq!(burst[0].level, tracing::Level::TRACE);
    assert!(burst[0].has_field("suppressed", "0"));
    assert!(burst[0].has_field("world", "Harset"));

    // Past the window: the next packet writes, accounting for the four.
    mgr.apply_client_position_update_at(
        t0 + std::time::Duration::from_millis(1_000),
        100,
        IN_THE_HOLE,
        [0, 0, 0],
        [0.0; 3],
    );
    let after = rows(&capture);
    assert_eq!(after.len(), 2);
    assert!(
        after[1].has_field("suppressed", "4"),
        "the four packets inside the window must be counted: {:#?}",
        after[1]
    );

    // An on-mesh packet is not an advisory row at all.
    mgr.apply_client_position_update_at(
        t0 + std::time::Duration::from_millis(3_000),
        100,
        ON_MESH,
        [0, 0, 0],
        [0.0; 3],
    );
    assert_eq!(rows(&capture).len(), 2, "on-mesh moves must not log");
}
