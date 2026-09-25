//! `movement.npc event=ground_deviation` over the real `castle_cellblock.nav`.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use tracing::Level;

use super::{add_npc, cellblock_mgr, movement_tick, rows, NPC};
use crate::test_support::LogCapture;

/// `MessHall_Guard1`'s spawn (`spawnlist.sql`), on the mess-hall floor.
const MESSHALL: [f32; 3] = [-96.25, 34.591, -91.59];

/// Put the guard on the mess-hall floor (Y from the storey-aware query)
/// with one waypoint at `offset` from it.
fn guard_with_leg(offset: [f32; 3]) -> crate::cell::space_manager::SpaceManager {
    let (mut mgr, _) = cellblock_mgr();
    add_npc(
        &mut mgr,
        "Castle_CellBlock",
        MESSHALL,
        None,
        AiState::Fighting,
    );
    let floor = mgr
        .get_navmesh_height(NPC, MESSHALL[0], MESSHALL[1], MESSHALL[2])
        .expect("the mess hall has a floor");
    let start = Vector3::new(MESSHALL[0], floor, MESSHALL[2]);
    mgr.update_entity_position(NPC, [start.x, start.y, start.z], [0; 3], [0.0; 3]);
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.move_speed = 1.0;
    npc.nav_path.push_back(Vector3::new(
        start.x + offset[0],
        start.y + offset[1],
        start.z + offset[2],
    ));
    mgr
}

/// **Acceptance: ground deviation on a lerped Y.** The detector itself, fed
/// the step the pre-NA11 tick wrote on a leg whose far end is 4 units up
/// (a floor-then-ramp leg in the XZ plane, audit M1): after one 1-unit step
/// the chord lerp is ~0.7 above the mess-hall floor. The tick no longer
/// produces this step (NA11 clamps it), so it is fed in directly.
#[test]
fn a_lerped_chord_over_a_flat_floor_is_a_ground_deviation() {
    let mut mgr = guard_with_leg([4.0, 4.0, 0.0]);
    let from = mgr.get_entity(NPC).unwrap().position;
    let wp = *mgr.get_entity(NPC).unwrap().nav_path.front().unwrap();
    // 1 u of a sqrt(32) u chord that rises 4 u.
    let t = 1.0 / 32f32.sqrt();
    let lerped = Vector3::new(from.x + 4.0 * t, from.y + 4.0 * t, from.z);
    let logs = LogCapture::install();
    super::super::movement::check_ground_step(
        &mut mgr,
        super::super::movement::GroundStep {
            npc_id: NPC,
            pos: lerped,
            wp,
            from,
            y_source: super::super::movement::YSource::Lerp,
        },
        std::time::Instant::now(),
    );
    let found = rows(&logs, "movement.npc", "ground_deviation");
    assert_eq!(found.len(), 1, "{:#?}", logs.all());
    let row = &found[0];
    assert_eq!(row.level, Level::WARN);
    assert!(row.has_field("dir", "up"), "{row:?}");
    assert!(row.has_field("y_source", "lerp"), "{row:?}");
    assert!(row.has_field("world", "Castle_CellBlock"));
    let dy: f32 = row.fields["dy"]
        .trim_start_matches("Some(")
        .trim_end_matches(')')
        .parse()
        .unwrap();
    assert!(dy > 0.3 && dy < 1.0, "dy {dy}");
}

/// **NA11 guard: the clamp keeps the detector silent.** The same
/// floor-then-ramp leg through the real movement tick: the ground clamp
/// puts every step on the floor, so `ground_deviation` never fires. On the
/// pre-NA11 lerp this leg fired on the first step (the test above).
#[test]
fn a_clamped_floor_then_ramp_leg_is_not_a_deviation() {
    let mut mgr = guard_with_leg([4.0, 4.0, 0.0]);
    let logs = LogCapture::install();
    for _ in 0..8 {
        movement_tick(&mut mgr);
    }
    assert!(
        rows(&logs, "movement.npc", "ground_deviation").is_empty(),
        "{:#?}",
        logs.all()
    );
}

/// **NA11 guard, full walk.** The audit's 42.8 u floor-then-ramp leg from
/// the Cellblock guard spawn to the ramp top, walked end to end: no
/// `ground_deviation` row, and every step's `y_source` is `clamp`.
#[test]
fn the_guard_walk_up_the_ramp_raises_no_ground_deviation() {
    let (mut mgr, _) = cellblock_mgr();
    let foot = [-289.465, 68.542, -154.276];
    add_npc(&mut mgr, "Castle_CellBlock", foot, None, AiState::Fighting);
    let top = Vector3::new(-315.6, 73.6, -191.4);
    let path = mgr
        .find_path(NPC, &Vector3::new(foot[0], foot[1], foot[2]), &top)
        .expect("the guard column routes to the ramp top");
    crate::cell::service::npc_ai::replace_nav_path_on(
        mgr.get_entity_mut(NPC).unwrap(),
        path.into_iter().skip(1),
    );
    let logs = LogCapture::install();
    for _ in 0..200 {
        if mgr.get_entity(NPC).unwrap().nav_path.is_empty() {
            break;
        }
        movement_tick(&mut mgr);
    }
    assert!(
        mgr.get_entity(NPC).unwrap().nav_path.is_empty(),
        "walk finished"
    );
    assert!(
        rows(&logs, "movement.npc", "ground_deviation").is_empty(),
        "{:#?}",
        rows(&logs, "movement.npc", "ground_deviation")
    );
    let steps = rows(&logs, "movement.npc", "step");
    assert!(!steps.is_empty(), "precondition: step rows were sampled");
    assert!(
        steps.iter().all(|r| r.has_field("y_source", "clamp")),
        "{steps:#?}"
    );
}

/// A leg along the floor stays on it: no row.
#[test]
fn a_step_along_the_floor_is_not_a_deviation() {
    let mut mgr = guard_with_leg([4.0, 0.0, 0.0]);
    let logs = LogCapture::install();
    movement_tick(&mut mgr);
    assert!(
        rows(&logs, "movement.npc", "ground_deviation").is_empty(),
        "{:#?}",
        logs.all()
    );
}
