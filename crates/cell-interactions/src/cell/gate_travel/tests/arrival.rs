//! The H01 arrival contract, end to end through `perform_gate_travel`.
//!
//! `cell::arrival`'s own tests prove the *decision* (pin vs gate row vs
//! respawner vs unrecoverable); these prove the *plumbing* — that the
//! resolved position is what actually reaches `CellToBaseMsg::GateTravel`,
//! and that an unrecoverable arrival sends nothing at all.
//!
//! Most tests here strip the origin world's gate region so the dial takes
//! the no-gate-volume fallback and reaches `perform_gate_travel` in one
//! call. CA10 funnels the walk-through crossing through that same
//! function, so one `validate_gate_arrival` call covers both entry points;
//! [`crossing_into_an_unrecoverable_arrival_sends_no_transfer`] is the
//! assertion that the crossing path really does share it.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use super::super::handle_dial_gate;
use super::{engine, grant_all_addresses, make_manager_with_stargates, strip_stargate_regions};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::StargateEntry;

/// Same Castle destination as fixture gate 2, but with an authored arrival
/// pin. Synthetic — H01 seeds no arrival coordinate for any gate; Harset's
/// is pinned in-game during milestone M0.
const PINNED_GATE: i32 = 99;
/// Castle_CellBlock gate whose row sits off the navmesh, with a respawner
/// available.
const OFF_MESH_GATE: i32 = 98;
/// Castle_CellBlock gate whose row sits off the navmesh with NO respawner.
const UNRECOVERABLE_GATE: i32 = 97;

/// Fixture coordinates, shared with `cell::arrival::tests`.
const ON_MESH: [f32; 3] = [-289.465, 68.542, -154.276];
const OFF_MESH: [f32; 3] = [-289.465, 268.542, -154.276];

fn gate(world: &str, pos: [f32; 3], yaw: f32, arrival: Option<([f32; 3], f32)>) -> StargateEntry {
    StargateEntry {
        world_name: world.to_string(),
        x: pos[0],
        y: pos[1],
        z: pos[2],
        yaw,
        address_origin: 18,
        arrival,
        event_set_id: None,
    }
}

/// The fallback fixture: no gate volume anywhere, one connected dialer on
/// Agnos, ready to travel on the dial.
fn fixture() -> SpaceManager {
    let mut mgr = make_manager_with_stargates();
    strip_stargate_regions(&mut mgr);
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    grant_all_addresses(&mut mgr, 1);
    mgr.connect_entity(1);
    mgr
}

fn saw_gate_travel(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> bool {
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::GateTravel { .. }) {
            return true;
        }
    }
    false
}

fn expect_gate_travel(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> ([f32; 3], [f32; 3]) {
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::GateTravel {
            position, rotation, ..
        } = msg
        {
            return (position, rotation);
        }
    }
    panic!("Expected GateTravel message");
}

/// An unpinned gate arrives on the gate row, facing the row's authored
/// yaw — the pre-H01 behaviour, pinned here so the arrival change can't
/// silently alter it for the 28 gates that have no pin.
#[tokio::test]
async fn gate_travel_without_an_arrival_pin_uses_the_gate_row() {
    let mut mgr = fixture();

    let (tx, mut rx) = mpsc::channel(16);
    assert!(handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await);

    let (position, rotation) = expect_gate_travel(&mut rx);
    assert!((position[0] - 761.677).abs() < 0.01);
    assert!((position[1] - 63.466).abs() < 0.01);
    assert!((position[2] - 551.716).abs() < 0.01);
    assert!((rotation[2] - 2.152).abs() < 0.01);
}

/// A pinned gate arrives on the pin, yaw included. Reverting
/// `perform_gate_travel` to `[gate.x, gate.y, gate.z]` / `gate.yaw` — the
/// pre-H01 line — fails this.
#[tokio::test]
async fn gate_travel_with_an_arrival_pin_uses_the_pin() {
    let mut mgr = fixture();
    mgr.stargates.insert(
        PINNED_GATE,
        gate(
            "Castle",
            [761.677, 63.466, 551.716],
            2.152,
            Some(([700.5, 60.25, 540.75], 1.0)),
        ),
    );
    // Re-grant: the earlier grant covered the cache as it stood, and this
    // test added a gate afterwards. A dial to an address the player does not
    // hold is refused before the arrival contract is ever reached
    // (CAT-O-01), which would make the assertions below pass vacuously.
    grant_all_addresses(&mut mgr, 1);

    let (tx, mut rx) = mpsc::channel(16);
    assert!(handle_dial_gate(1, PINNED_GATE, 0, &tx, &mut mgr, &engine()).await);

    let (position, rotation) = expect_gate_travel(&mut rx);
    // Exact, not tolerance-based: these literals are copied verbatim
    // through the call chain with no arithmetic applied.
    assert_eq!(position, [700.5, 60.25, 540.75]);
    assert_eq!(rotation[2], 1.0);
    assert_ne!(
        position,
        [761.677, 63.466, 551.716],
        "the prefab-origin gate row must not win over an authored pin"
    );
}

/// End to end, on the wire the base actually receives: a gate whose
/// arrival is off the destination world's navmesh must hand BaseApp the
/// respawner position, not the authored one.
///
/// Without this the defect the packet exists to prevent — an off-mesh
/// coordinate reaching `CellToBaseMsg::GateTravel` — is only covered in
/// two disjoint halves.
#[tokio::test]
async fn gate_travel_to_an_off_mesh_arrival_sends_the_respawner_position() {
    use crate::cell::arrival::{test_fixture_mesh, test_insert_navmesh_space};
    use crate::cell::spawner::RespawnerDef;

    let Some(mesh) = test_fixture_mesh() else {
        return;
    };

    let mut mgr = fixture();
    test_insert_navmesh_space(&mut mgr, "Castle_CellBlock", mesh);
    mgr.respawners.push(RespawnerDef {
        respawner_id: 1,
        world_name: "Castle_CellBlock".to_string(),
        name: "test".to_string(),
        pos: ON_MESH,
    });
    mgr.stargates.insert(
        OFF_MESH_GATE,
        gate("Castle_CellBlock", OFF_MESH, 1.75, None),
    );
    // Re-grant: the earlier grant covered the cache as it stood, and this
    // test added a gate afterwards. A dial to an address the player does not
    // hold is refused before the arrival contract is ever reached
    // (CAT-O-01), which would make the assertions below pass vacuously.
    grant_all_addresses(&mut mgr, 1);

    let (tx, mut rx) = mpsc::channel(16);
    assert!(handle_dial_gate(1, OFF_MESH_GATE, 0, &tx, &mut mgr, &engine()).await);

    let (position, rotation) = expect_gate_travel(&mut rx);
    assert_eq!(position, ON_MESH, "the off-mesh gate row must be replaced");
    assert_ne!(
        position, OFF_MESH,
        "an off-navmesh arrival must never reach the base — that is the \
         silent-freeze bug"
    );
    assert_eq!(
        rotation[2], 1.75,
        "a respawner fallback carries the authored facing through"
    );
}

/// PR #662 review, finding 1. When the destination world's navmesh
/// rejects the arrival **and** no respawner qualifies, there is nowhere
/// safe to land — so the transfer is refused outright rather than
/// shipping the rejected coordinate into `GateTravel`.
///
/// Regression shape: before the fix `resolve_arrival_with` returned the
/// invalid point with `source = UnrecoverableOffMesh` and
/// `perform_gate_travel` passed `arrival.position` straight through, so
/// the traveller was destroyed cell-side and re-created off-mesh on a
/// world they could not stand in — the silent `CorrectionSuppressed`
/// freeze H01 exists to prevent, one layer further along. Deleting the
/// `!arrival.is_usable()` guard fails every assertion below.
#[tokio::test]
async fn dial_gate_to_an_unrecoverable_arrival_sends_no_transfer() {
    use crate::cell::arrival::{test_fixture_mesh, test_insert_navmesh_space};

    let Some(mesh) = test_fixture_mesh() else {
        return;
    };

    let mut mgr = fixture();
    // Meshed destination, and deliberately NO respawner for it: the
    // fallback has no candidate, which is the unrecoverable case.
    test_insert_navmesh_space(&mut mgr, "Castle_CellBlock", mesh);
    assert!(
        mgr.respawners
            .iter()
            .all(|r| r.world_name != "Castle_CellBlock"),
        "precondition: the unrecoverable case needs zero qualifying respawners"
    );
    mgr.stargates.insert(
        UNRECOVERABLE_GATE,
        gate("Castle_CellBlock", OFF_MESH, 1.75, None),
    );
    // Re-grant: the earlier grant covered the cache as it stood, and this
    // test added a gate afterwards. A dial to an address the player does not
    // hold is refused before the arrival contract is ever reached
    // (CAT-O-01), which would make the assertions below pass vacuously.
    grant_all_addresses(&mut mgr, 1);
    let space_before = mgr.get_entity_space_id(1);

    let (tx, mut rx) = mpsc::channel(16);
    let dialed = handle_dial_gate(1, UNRECOVERABLE_GATE, 0, &tx, &mut mgr, &engine()).await;

    assert!(!dialed, "an unrecoverable arrival must refuse the dial");
    assert!(
        !saw_gate_travel(&mut rx),
        "no GateTravel may reach the base — an off-mesh arrival with no \
         recovery is the silent-freeze bug"
    );
    assert!(
        mgr.get_entity(1).is_some(),
        "a refused dial must leave the traveller in their own space, not \
         tear them down with no transfer in flight"
    );
    assert_eq!(mgr.get_entity_space_id(1), space_before);
}

/// The same refusal on the CA10 crossing path: the dial arms on a world
/// that *does* have a gate volume, the gate opens, the player walks in —
/// and the unrecoverable destination still sends no transfer. One
/// `validate_gate_arrival` call in `perform_gate_travel` covers both
/// entry points; this is the assertion that says so.
#[tokio::test]
async fn crossing_into_an_unrecoverable_arrival_sends_no_transfer() {
    use crate::cell::arrival::{test_fixture_mesh, test_insert_navmesh_space};

    use super::super::{gate_dial_tick, handle_stargate_region_entered};

    let Some(mesh) = test_fixture_mesh() else {
        return;
    };

    // Deliberately NOT `fixture()`: this path needs the gate region left
    // in place so the dial arms instead of travelling.
    let mut mgr = make_manager_with_stargates();
    test_insert_navmesh_space(&mut mgr, "Castle_CellBlock", mesh);
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    grant_all_addresses(&mut mgr, 1);
    mgr.connect_entity(1);
    mgr.stargates.insert(
        UNRECOVERABLE_GATE,
        gate("Castle_CellBlock", OFF_MESH, 1.75, None),
    );
    // Re-grant: the earlier grant covered the cache as it stood, and this
    // test added a gate afterwards. A dial to an address the player does not
    // hold is refused before the arrival contract is ever reached
    // (CAT-O-01), which would make the assertions below pass vacuously.
    grant_all_addresses(&mut mgr, 1);
    let space_before = mgr.get_entity_space_id(1);

    let (tx, mut rx) = mpsc::channel(16);
    assert!(
        handle_dial_gate(1, UNRECOVERABLE_GATE, 0, &tx, &mut mgr, &engine()).await,
        "the dial itself is accepted — the arrival is only resolved at the \
         crossing, which is where the transfer is chosen"
    );

    // Rewind the deadline so `gate_dial_tick`'s internal `Instant::now()`
    // sees it as elapsed, then open the gate and walk in.
    mgr.pending_gate_dials
        .get_mut(&1)
        .expect("a dial must be armed")
        .open_at = Instant::now() - Duration::from_millis(1);
    gate_dial_tick(&tx, &mut mgr).await;
    handle_stargate_region_entered(1, &tx, &mut mgr, &engine()).await;

    // NA35: the crossing no longer runs `perform_gate_travel` (and its
    // arrival check) synchronously — it arms a `CROSSING_CINEMATIC_HOLD`
    // and defers to `crossing_tick`. Without expiring and draining the
    // hold here, `saw_gate_travel` would be false regardless of whether
    // the arrival is recoverable, making this assertion pass vacuously.
    mgr.pending_crossings
        .get_mut(&1)
        .expect("a crossing hold must be armed")
        .travel_at = Instant::now() - Duration::from_millis(1);
    super::super::crossing_tick(&tx, &mut mgr).await;

    assert!(
        !saw_gate_travel(&mut rx),
        "a crossing into an unrecoverable arrival must send no GateTravel"
    );
    assert!(
        mgr.get_entity(1).is_some(),
        "the traveller must survive a refused crossing"
    );
    assert_eq!(mgr.get_entity_space_id(1), space_before);
}
