//! AoI / witness-set diff tests covering enter / move / leave plus the
//! disconnect broadcast that fires `LeftAoI` to remaining witnesses.

use super::super::super::messages::CellToBaseMsg;
use super::make_manager;

#[test]
fn aoi_detects_nearby_players() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    let events = mgr.compute_aoi_changes();

    // Both players should see each other enter AoI
    let entered: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, CellToBaseMsg::EnteredAoI { .. }))
        .collect();
    assert_eq!(entered.len(), 2);
}

#[test]
fn aoi_detects_entity_leaving() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    // First tick: both enter AoI
    let _ = mgr.compute_aoi_changes();

    // Move entity 200 far away
    mgr.update_entity_position(200, [5000.0, 0.0, 5000.0], [0, 0, 0], [0.0; 3]);

    // Second tick: entity 200 should leave AoI of entity 100
    let events = mgr.compute_aoi_changes();
    let left: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, CellToBaseMsg::LeftAoI { .. }))
        .collect();
    assert_eq!(left.len(), 2); // Both should lose sight of each other
}

/// AoI second-tick path: when an entity is in BOTH the previous and
/// current witness sets, an `EntityMoved` event must fire so the
/// witness gets the position update. This covers the third arm of
/// `compute_aoi_changes` that the existing entered/left tests don't.
#[test]
fn aoi_emits_entity_moved_for_entities_in_both_previous_and_current_sets() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    // First tick populates the witness set.
    let _ = mgr.compute_aoi_changes();
    // Move 200 a small amount so it's still in 100's AoI.
    mgr.update_entity_position(200, [25.0, 0.0, 25.0], [0, 0, 0], [1.0, 0.0, 0.0]);
    let events = mgr.compute_aoi_changes();
    let moved: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            CellToBaseMsg::EntityMoved {
                witness_id,
                entity_id,
                velocity,
                ..
            } if *witness_id == 100 && *entity_id == 200 => Some(*velocity),
            _ => None,
        })
        .collect();
    assert_eq!(
        moved.len(),
        1,
        "expected exactly one EntityMoved for 200 to witness 100; got events={events:?}"
    );
    assert_eq!(
        moved[0],
        [1.0, 0.0, 0.0],
        "EntityMoved must carry the velocity from update_entity_position"
    );
}

/// Full ghost lifecycle as seen by witness 100 watching player 200
/// across four ticks: enter → move → leave → re-enter. Pin:
///
///   - tick 1 (enter): exactly one `EnteredAoI(witness=100, entity=200)`.
///   - tick 2 (move within radius): exactly one `EntityMoved` for
///     200 to witness 100, no further `EnteredAoI` for the same pair.
///     Without this, a regression that re-fires EnteredAoI on every
///     tick would silently leak ghosts on the client.
///   - tick 3 (leave): exactly one `LeftAoI` for 200 to witness 100,
///     no `EntityMoved` for 200 (it's no longer in current_aoi).
///   - tick 4 (re-enter): `EnteredAoI` fires again — the witness set
///     was cleared on leave, so the diff sees the entity as fresh.
///
/// This is the cross-tick integration sibling of the per-tick tests
/// above: they pin one transition each, this one pins the *sequence*
/// the C++ server dispatches in `cached_entity.cpp:199` /
/// `client_handler.cpp:516-556`.
#[test]
fn ghost_lifecycle_for_witness_enter_move_leave_reenter() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    // Helper to count events targeting witness 100 about entity 200.
    fn count_for_pair(
        events: &[CellToBaseMsg],
        witness: u32,
        entity: u32,
    ) -> (usize, usize, usize) {
        let mut entered = 0;
        let mut moved = 0;
        let mut left = 0;
        for e in events {
            match e {
                CellToBaseMsg::EnteredAoI {
                    witness_id,
                    entity_id,
                    ..
                } if *witness_id == witness && *entity_id == entity => entered += 1,
                CellToBaseMsg::EntityMoved {
                    witness_id,
                    entity_id,
                    ..
                } if *witness_id == witness && *entity_id == entity => moved += 1,
                CellToBaseMsg::LeftAoI {
                    witness_id,
                    entity_id,
                } if *witness_id == witness && *entity_id == entity => left += 1,
                _ => {}
            }
        }
        (entered, moved, left)
    }

    // ── tick 1: 200 enters 100's AoI ────────────────────────────────
    let t1 = mgr.compute_aoi_changes();
    let (e, m, l) = count_for_pair(&t1, 100, 200);
    assert_eq!(
        (e, m, l),
        (1, 0, 0),
        "tick 1 must fire exactly one EnteredAoI for the (100, 200) pair, no Moved/Left",
    );

    // ── tick 2: 200 moves a small amount, still inside 100's radius ──
    mgr.update_entity_position(200, [12.0, 0.0, 12.0], [0, 0, 0], [1.0, 0.0, 0.0]);
    let t2 = mgr.compute_aoi_changes();
    let (e, m, l) = count_for_pair(&t2, 100, 200);
    assert_eq!(
        (e, m, l),
        (0, 1, 0),
        "tick 2 must fire EntityMoved (no re-enter, no leave) for the (100, 200) pair",
    );

    // ── tick 3: 200 leaves 100's AoI ────────────────────────────────
    mgr.update_entity_position(200, [5000.0, 0.0, 5000.0], [0, 0, 0], [0.0; 3]);
    let t3 = mgr.compute_aoi_changes();
    let (e, m, l) = count_for_pair(&t3, 100, 200);
    assert_eq!(
        (e, m, l),
        (0, 0, 1),
        "tick 3 must fire LeftAoI exactly once and NOT emit EntityMoved for an out-of-range entity",
    );

    // ── tick 4: 200 returns into 100's AoI — must re-fire EnteredAoI ──
    // Without this, a "remember-forever" witness set would silently
    // suppress the second-life ghost packet and the client would never
    // see the entity again.
    mgr.update_entity_position(200, [10.0, 0.0, 10.0], [0, 0, 0], [0.0; 3]);
    let t4 = mgr.compute_aoi_changes();
    let (e, m, l) = count_for_pair(&t4, 100, 200);
    assert_eq!(
        (e, m, l),
        (1, 0, 0),
        "tick 4 must re-fire EnteredAoI when 200 returns; the diff is stateful per (witness, entity) pair",
    );
}

/// `compute_aoi_changes` must persist the new witness set on the
/// player so that the NEXT tick's diff sees it. A regression that
/// drops the `entity.witnesses = current_aoi` write would re-fire
/// EnteredAoI on every tick.
#[test]
fn aoi_persists_witness_set_so_subsequent_ticks_diff_correctly() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    let first = mgr.compute_aoi_changes();
    let first_entered: Vec<(u32, u32)> = first
        .iter()
        .filter_map(|e| match e {
            CellToBaseMsg::EnteredAoI {
                witness_id,
                entity_id,
                ..
            } => Some((*witness_id, *entity_id)),
            _ => None,
        })
        .collect();
    // Two connected players in the same space, both within AoI radius:
    // each must see the other once and exactly once. Tighter than
    // ">= 2" so a regression that double-fires EnteredAoI in the same
    // tick is caught.
    assert_eq!(first_entered.len(), 2, "first tick must populate AoI");
    assert!(first_entered.contains(&(100, 200)));
    assert!(first_entered.contains(&(200, 100)));

    // Second tick with no movement: no further EnteredAoI events.
    let second = mgr.compute_aoi_changes();
    let second_entered = second
        .iter()
        .filter(|e| matches!(e, CellToBaseMsg::EnteredAoI { .. }))
        .count();
    assert_eq!(
        second_entered, 0,
        "second tick must not re-fire EnteredAoI; the witness set should already pin both",
    );
}

/// Disconnecting a player must broadcast `LeftAoI` to every other
/// player who currently has that player in their witness set.
/// Regression guard: the broadcast iterates `space.players`, not the
/// disconnecting entity's own `witnesses` (those point in the wrong
/// direction).
#[tokio::test]
async fn disconnect_emits_left_aoi_to_observers_then_destroys_entity() {
    use tokio::sync::mpsc;
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);
    let _ = mgr.compute_aoi_changes(); // populate witness sets

    let (tx, mut rx) = mpsc::channel(8);
    mgr.disconnect_entity(100, &tx).await;

    let mut left_msgs: Vec<(u32, u32)> = Vec::new();
    while let Ok(m) = rx.try_recv() {
        if let CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id,
        } = m
        {
            left_msgs.push((witness_id, entity_id));
        }
    }
    assert_eq!(
        left_msgs,
        vec![(200, 100)],
        "player 200 must receive LeftAoI for the disconnecting 100"
    );
    // The disconnecting entity must be fully destroyed (entity_space
    // entry removed) — `destroy_entity` chains off the disconnect path.
    assert!(!mgr.entity_space.contains_key(&100));
}
