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
fn guard_with_leg(offset: [f32; 3]) -> Option<crate::cell::space_manager::SpaceManager> {
    let (mut mgr, _) = cellblock_mgr()?;
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
    Some(mgr)
}

/// **Acceptance: ground deviation on a lerped chord.** A leg whose far end
/// is 4 units up (a floor-then-ramp leg in the XZ plane, audit M1) lerps
/// the NPC's Y upward over a flat floor: after one 1-unit step it is ~0.7
/// above the mess-hall floor. Revert-proof: removing the `check_ground`
/// call from the lerp branch of `npc_movement_tick` leaves no row.
#[test]
fn a_lerped_chord_over_a_flat_floor_is_a_ground_deviation() {
    let Some(mut mgr) = guard_with_leg([4.0, 4.0, 0.0]) else {
        return;
    };
    let logs = LogCapture::install();
    movement_tick(&mut mgr);
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

/// A leg along the floor stays on it: no row.
#[test]
fn a_step_along_the_floor_is_not_a_deviation() {
    let Some(mut mgr) = guard_with_leg([4.0, 0.0, 0.0]) else {
        return;
    };
    let logs = LogCapture::install();
    movement_tick(&mut mgr);
    assert!(
        rows(&logs, "movement.npc", "ground_deviation").is_empty(),
        "{:#?}",
        logs.all()
    );
}
