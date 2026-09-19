//! Ring-side arrival contract (PR #662 review, findings 1 and 7).
//!
//! Ring arrival is **validate-only**. The destination pad's row coordinate is
//! the arrival — the client plays the ring matinee at that pad and the FSM
//! fires `FireTeleportIn` for that region — so the gate-travel respawner
//! fallback is wrong here: it would put the passengers somewhere else in the
//! world entirely while the ring sequence still played. And a pad the
//! destination world's navmesh rejects has no second answer at all, so the
//! trip aborts rather than landing them on a point the position validator
//! would suppress every update from.

use std::time::Duration;

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::support::{spawn_player, state_of, three_ring_mgr, FakeClock};
use crate::cell::arrival::{test_fixture_mesh, test_insert_navmesh_space};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::ring_transport::{
    handle_select_destination, run_tick_with_engine, State, BSF_MOVEMENT_LOCK,
};
use crate::cell::spawner::RespawnerDef;
use crate::mercury::method_idx::ON_VISIBLE;

/// A guard spawn coordinate the `castle_cellblock` fixture mesh accepts —
/// shared with `cell::arrival`'s own tests and
/// `crates/entity/src/navigation/tests.rs`.
const ON_MESH: [f32; 3] = [-289.465, 68.542, -154.276];

/// Drive one trip from ring 1 to ring 2 (whose pad row is at `(10, 20, 30)`,
/// which the fixture mesh rejects) and report what came out.
///
/// Returns `(saw_teleport, saw_visible, passenger_position)`.
async fn run_off_mesh_pad_trip(respawners: Vec<RespawnerDef>) -> (bool, bool, [f32; 3]) {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);

    // Graft the real `castle_cellblock` mesh onto the rings' world *after*
    // the player exists, so only the destination-pad lookup sees it.
    let mesh = test_fixture_mesh().expect("caller checked the fixture exists");
    test_insert_navmesh_space(&mut mgr, "Castle_CellBlock", mesh);
    mgr.respawners = respawners;

    let (tx, mut rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    // Player is already on pad 1, so selecting kicks warmup off immediately.
    mgr.ring_transporters
        .get_mut(1)
        .unwrap()
        .region_triggered(true, 42);
    handle_select_destination(1, 2, 42, &tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 1), State::SendWarmup);
    assert_eq!(state_of(&mgr, 2), State::RecvWarmup);
    let before = mgr.get_entity(42).unwrap().position;
    while rx.try_recv().is_ok() {}

    // Past the 3.5s hide and the 4.0s warmup.
    clock.advance(Duration::from_secs(5));
    run_tick_with_engine(&tx, &mut mgr, &engine).await;

    assert_eq!(
        state_of(&mgr, 1),
        State::Idle,
        "the source must be released, not left in SendWarmup"
    );
    assert_eq!(
        state_of(&mgr, 2),
        State::Idle,
        "the peer must be dragged back to Idle — with the guard reverted it \
         advances into RemoteLoadWait and beyond"
    );
    assert_eq!(
        mgr.get_entity(42).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0,
        "the abort must clear the passenger's movement lock"
    );
    assert_eq!(
        (before.x, before.y, before.z),
        (0.0, 0.0, 0.0),
        "precondition: the passenger starts at the fixture spawn"
    );

    let mut saw_teleport = false;
    let mut saw_visible = false;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::TeleportPlayer { .. } => saw_teleport = true,
            CellToBaseMsg::WitnessEntityMethod {
                entity_id: 42,
                method_index,
                ref args,
                ..
            } if method_index == ON_VISIBLE && args == &[1] => saw_visible = true,
            _ => {}
        }
    }

    // And it stays quiet: nothing re-fires the cleared warmup deadline.
    run_tick_with_engine(&tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 1), State::Idle);
    assert_eq!(state_of(&mgr, 2), State::Idle);

    let after = mgr.get_entity(42).unwrap().position;
    (saw_teleport, saw_visible, [after.x, after.y, after.z])
}

/// Finding 1, ring half. Ring 2's pad row is off the destination navmesh and
/// there is nothing to fall back to, so the trip aborts and the passenger is
/// released where they stood.
///
/// Reverting the `ArrivalCheck::OffMesh` guard in `run_one_deadline` fails
/// this on three independent assertions: ring 2 is left mid-trip, a
/// `TeleportPlayer` carrying the rejected pad coordinate reaches the base,
/// and the passenger is moved to it.
#[tokio::test]
async fn warmup_to_an_off_mesh_pad_aborts_and_releases_the_passengers() {
    if test_fixture_mesh().is_none() {
        return;
    }
    let (saw_teleport, saw_visible, after) = run_off_mesh_pad_trip(Vec::new()).await;

    assert!(
        !saw_teleport,
        "no teleport may reach the base — the rejected pad coordinate is the \
         silent-freeze bug the arrival helper exists to prevent"
    );
    assert!(saw_visible, "the abort must re-show the hidden passenger");
    assert_eq!(
        after,
        [0.0, 0.0, 0.0],
        "a refused arrival must leave the passenger exactly where they were"
    );
}

/// Finding 7. The same off-mesh pad, but now the world *does* have a
/// qualifying respawner — the case that used to relocate every passenger to
/// it while the FSM went right on emitting the ring sequence and
/// `FireTeleportIn` for a pad they never reached.
///
/// Reverting `run_one_deadline`'s warmup arm to `resolve_arrival` fails this:
/// the arrival resolves to `ArrivalSource::Respawner`, the trip completes,
/// and the passenger ends up at `ON_MESH` — nowhere near ring 2's pad, and
/// with nothing in the log connecting the ring they took to the place they
/// came out.
#[tokio::test]
async fn an_off_mesh_pad_is_never_silently_swapped_for_a_respawner() {
    if test_fixture_mesh().is_none() {
        return;
    }
    let respawners = vec![RespawnerDef {
        respawner_id: 1,
        world_name: "Castle_CellBlock".to_string(),
        name: "cellblock-default".to_string(),
        pos: ON_MESH,
    }];
    let (saw_teleport, saw_visible, after) = run_off_mesh_pad_trip(respawners).await;

    assert!(
        !saw_teleport,
        "a ring must never teleport a passenger to a respawner: the pad row is \
         the arrival, and the client is playing the ring sequence at the pad"
    );
    assert!(saw_visible, "the abort must re-show the hidden passenger");
    assert_ne!(
        after, ON_MESH,
        "the respawner must not become the ring's arrival"
    );
    assert_eq!(
        after,
        [0.0, 0.0, 0.0],
        "a refused arrival must leave the passenger exactly where they were"
    );
}
