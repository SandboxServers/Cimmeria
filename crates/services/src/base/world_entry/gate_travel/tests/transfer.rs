//! Base-side half of the GM cross-instance transfer primitive (packet P45).
//!
//! Three properties, each of which the packet's acceptance criteria name:
//!
//! 1. **Exact instance.** The `destination_space_id` the cell put on
//!    `GateTravel` must reach `BaseToCellMsg::CreateEntity` unchanged, and
//!    the space the cell replies with must be the one written into
//!    `pending_world_entry` — that value becomes the client's world-entry
//!    wire packet.
//! 2. **Validate before teardown.** The fail-closed `active_player_id` guard
//!    has to fire *before* `CreateEntity`. `CreateEntity` is the point of no
//!    return on this side: the cell has already removed the entity from its
//!    origin space, so an abort after the create leaves a player whose entity
//!    moved worlds while their client never got `RESET_ENTITIES`.
//! 3. **Disconnect mid-transfer.** If the client drops while the create
//!    round-trip is in flight, the freshly created destination entity is a
//!    clientless ghost and must be reaped.

use super::*;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::TestTransport;
use std::time::Duration;
use tokio::time::timeout;

const ENTITY_ID: u32 = 42;
const DEST_SPACE: u32 = 0x0001_ABCD;
const DEST_WORLD: &str = "Castle_CellBlock";

struct Fixture {
    transport: Arc<dyn Transport>,
    /// Typed handle onto the same transport, so "did RESET_ENTITIES go out?"
    /// can be asserted directly rather than inferred from `pending_world_entry`.
    sent: Arc<TestTransport>,
    addr: SocketAddr,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

async fn fixture(port: u16) -> Fixture {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::new()));
    connected.lock().unwrap().insert(addr, make_state());
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(ENTITY_ID, addr);
        m
    }));
    let sent = Arc::new(TestTransport::new());
    Fixture {
        transport: sent.clone(),
        sent,
        addr,
        connected,
        entity_to_addr,
    }
}

/// Stand in for the cell loop: accept the `CreateEntity`, assert what it
/// carried, and reply with `reply_space`.
async fn expect_create_entity(
    rx: &mut mpsc::Receiver<BaseToCellMsg>,
    reply_space: u32,
) -> Option<u32> {
    let msg = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("CreateEntity must not hang")
        .expect("CreateEntity expected");
    match msg {
        BaseToCellMsg::CreateEntity {
            entity_id,
            world_name,
            destination_space_id,
            reply_tx,
            ..
        } => {
            assert_eq!(entity_id, ENTITY_ID);
            assert_eq!(world_name, DEST_WORLD);
            let _ = reply_tx.send(reply_space);
            destination_space_id
        }
        // `BaseToCellMsg` carries a `oneshot::Sender` and so has no `Debug`;
        // the variant name is the useful half anyway.
        _ => panic!("expected CreateEntity as the first base->cell message"),
    }
}

/// End of the chain that starts at `space_transfer::transfer_player_to_space`:
/// the exact instance it resolved has to survive the base hop and land in the
/// client's `pending_world_entry`.
///
/// Regression shape: drop the field anywhere between `GateTravel` and
/// `CreateEntity` and the cell falls back to by-world-name resolution, which
/// for an instanced world allocates a fresh private instance — the GM arrives
/// in an empty copy of the map rather than beside the player they targeted.
#[tokio::test]
async fn destination_space_id_reaches_create_entity_and_pending_world_entry() {
    let f = fixture(55730).await;
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);

    let connected = Arc::clone(&f.connected);
    let entity_to_addr = Arc::clone(&f.entity_to_addr);
    let transport = Arc::clone(&f.transport);
    let handle = tokio::spawn(async move {
        handle_gate_travel(
            ENTITY_ID,
            DEST_WORLD,
            [11.0, 22.0, 33.0],
            [0.0; 3],
            None,
            Some(DEST_SPACE),
            &transport,
            &connected,
            &entity_to_addr,
            &Some(cell_tx),
            &None,
        )
        .await
    });

    let forwarded = expect_create_entity(&mut cell_rx, DEST_SPACE).await;
    assert_eq!(
        forwarded,
        Some(DEST_SPACE),
        "the exact destination instance must be forwarded to the cell verbatim"
    );

    timeout(Duration::from_secs(2), handle)
        .await
        .expect("gate travel must not hang")
        .unwrap()
        .expect("gate travel completes");

    let map = f.connected.lock().unwrap();
    let entry = map
        .get(&f.addr)
        .unwrap()
        .pending_world_entry
        .as_ref()
        .expect("pending_world_entry must be populated");
    assert_eq!(
        entry.space_id, DEST_SPACE,
        "the world-entry packet must describe the instance the cell placed the entity in"
    );
    assert_eq!(entry.world_name, DEST_WORLD);
    assert_eq!(entry.pos, [11.0, 22.0, 33.0]);
    drop(map);
    assert_eq!(
        f.sent.send_count_to(f.addr),
        1,
        "the committed transfer must send exactly one packet (RESET_ENTITIES) to the traveller"
    );
}

/// The space the *cell* actually resolved wins, not the one base asked for.
/// When the requested instance died mid-flight the cell degrades to a fresh
/// one and replies with it; building the world-entry packet from the
/// requested id instead would describe a space the entity is not in.
#[tokio::test]
async fn pending_world_entry_uses_the_space_the_cell_replied_with() {
    const CELL_CHOSE: u32 = 0x0001_9999;
    let f = fixture(55731).await;
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);

    let connected = Arc::clone(&f.connected);
    let entity_to_addr = Arc::clone(&f.entity_to_addr);
    let transport = Arc::clone(&f.transport);
    let handle = tokio::spawn(async move {
        handle_gate_travel(
            ENTITY_ID,
            DEST_WORLD,
            [0.0; 3],
            [0.0; 3],
            None,
            Some(DEST_SPACE),
            &transport,
            &connected,
            &entity_to_addr,
            &Some(cell_tx),
            &None,
        )
        .await
    });

    // Cell was asked for DEST_SPACE but resolved CELL_CHOSE.
    assert_eq!(
        expect_create_entity(&mut cell_rx, CELL_CHOSE).await,
        Some(DEST_SPACE)
    );
    timeout(Duration::from_secs(2), handle)
        .await
        .expect("gate travel must not hang")
        .unwrap()
        .unwrap();

    let map = f.connected.lock().unwrap();
    let entry = map
        .get(&f.addr)
        .unwrap()
        .pending_world_entry
        .as_ref()
        .expect("pending_world_entry must be populated");
    assert_eq!(
        entry.space_id, CELL_CHOSE,
        "the cell's resolved space is authoritative over the requested one"
    );
}

/// Ordering guard. Without a cached `active_player_id` the transfer is
/// refused — and the refusal must happen before `CreateEntity` is sent.
///
/// Regression shape: move the guard back below the create round-trip (where
/// it used to live) and the cell entity is moved into the destination world
/// while the client never receives `RESET_ENTITIES` and never gets a
/// `pending_world_entry` — a desynced player with no way back.
///
/// The abort is *not* a clean no-op, and this test must not pretend it is:
/// by the time the handler runs, the cell has already torn the entity out of
/// its origin space, and the base cannot put it back (it was never told the
/// origin space id). So the abort ends the session — see
/// [`aborted_transfer_ends_the_session_rather_than_stranding_an_unspaced_client`],
/// which is the half of this contract that keeps the player recoverable.
#[tokio::test]
async fn missing_active_player_id_aborts_before_create_entity_is_sent() {
    let f = fixture(55732).await;
    f.connected
        .lock()
        .unwrap()
        .get_mut(&f.addr)
        .unwrap()
        .active_player_id = None;
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);

    // Bounded: with the guard reverted to its old position the handler sends
    // `CreateEntity` and then awaits a oneshot reply nobody will ever send,
    // so the regression manifests as a hang. The timeout turns that into a
    // fast, legible failure instead of a wedged suite.
    timeout(
        Duration::from_secs(5),
        handle_gate_travel(
            ENTITY_ID,
            DEST_WORLD,
            [0.0; 3],
            [0.0; 3],
            None,
            Some(DEST_SPACE),
            &f.transport,
            &f.connected,
            &f.entity_to_addr,
            // Clone so the local `cell_tx` keeps the channel open — otherwise
            // `try_recv` reports Disconnected and "no message" becomes ambiguous.
            &Some(cell_tx.clone()),
            &None,
        ),
    )
    .await
    .expect(
        "the fail-closed abort must return promptly — hanging here means it \
         sent CreateEntity and is waiting on a reply that will never come",
    )
    .expect("the fail-closed abort returns Ok");

    // No CreateEntity. The only thing the cell should hear is the session
    // teardown the abort performs (asserted in detail by the sibling test).
    while let Ok(msg) = cell_rx.try_recv() {
        match msg {
            BaseToCellMsg::CreateEntity { .. } => panic!(
                "fail-closed abort must happen BEFORE the cell is told to create \
                 the destination entity"
            ),
            BaseToCellMsg::DisconnectEntity { .. } => {}
            _ => panic!("the aborted transfer must only tear the session down"),
        }
    }
    assert!(
        f.connected.lock().unwrap().get(&f.addr).is_none(),
        "the aborted transfer must not leave a live session behind"
    );
    assert!(
        f.sent.is_empty(),
        "an aborted transfer must not send RESET_ENTITIES — tearing the client's \
         entity system down and then never re-entering is the worst of both"
    );
}

/// The un-spaced recovery contract, and the packet's third acceptance
/// criterion at its sharpest.
///
/// When the fail-closed guard fires, the cell has *already* removed the entity
/// from its origin space and the base has no way to restore it. Simply
/// returning would leave a connected client bound to an entity that is in no
/// space at all: every position update it sends is dropped as `EntityMissing`,
/// nobody can see it, and nothing ever fixes it. So the abort ends the
/// session, which is the one recovery available here — the client reconnects
/// and is rebuilt from the DB.
///
/// Regression shape: delete the `abandon_unspaced_session` call and this test
/// fails with a live session still in `connected` — the "never leave the
/// entity un-spaced" criterion silently violated on a path whose other test
/// still passes.
#[tokio::test]
async fn aborted_transfer_ends_the_session_rather_than_stranding_an_unspaced_client() {
    let f = fixture(55735).await;
    f.connected
        .lock()
        .unwrap()
        .get_mut(&f.addr)
        .unwrap()
        .active_player_id = None;
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);

    timeout(
        Duration::from_secs(5),
        handle_gate_travel(
            ENTITY_ID,
            DEST_WORLD,
            [0.0; 3],
            [0.0; 3],
            None,
            Some(DEST_SPACE),
            &f.transport,
            &f.connected,
            &f.entity_to_addr,
            &Some(cell_tx.clone()),
            &None,
        ),
    )
    .await
    .expect("the abort must return promptly")
    .expect("the abort returns Ok");

    assert!(
        f.connected.lock().unwrap().get(&f.addr).is_none(),
        "an unrecoverable abort must end the session — leaving it live strands a \
         client whose entity is in no space, with no way back short of a relog \
         they have no reason to attempt"
    );
    assert!(
        f.entity_to_addr.lock().unwrap().get(&ENTITY_ID).is_none(),
        "the reverse mapping must go with the session"
    );

    let mut told_cell = false;
    while let Ok(msg) = cell_rx.try_recv() {
        if let BaseToCellMsg::DisconnectEntity { entity_id } = msg {
            assert_eq!(entity_id, ENTITY_ID);
            told_cell = true;
        }
    }
    assert!(
        told_cell,
        "the cell must be told to drop the player, or it keeps stale session state"
    );
}

/// What the session bookkeeping looks like by the time the create round-trip
/// resolves.
enum MidTransfer {
    /// `destroy_client_entities` ran in full: mapping and session both gone.
    FullyDisconnected,
    /// Only the reverse mapping was pulled (the order
    /// `destroy_client_entities` actually does it in).
    MappingDropped,
    /// Only the session entry was pulled.
    SessionDropped,
    /// The id was recycled: it is mapped again, but to a *different* session.
    RecycledToAnotherSession,
}

/// Drive a transfer, mutate the session bookkeeping while the create
/// round-trip is in flight, and report whether the entity got reaped.
async fn reap_outcome_when(port: u16, what: MidTransfer) -> bool {
    let f = fixture(port).await;
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);

    let connected = Arc::clone(&f.connected);
    let entity_to_addr = Arc::clone(&f.entity_to_addr);
    let transport = Arc::clone(&f.transport);
    let handle = tokio::spawn(async move {
        handle_gate_travel(
            ENTITY_ID,
            DEST_WORLD,
            [0.0; 3],
            [0.0; 3],
            None,
            Some(DEST_SPACE),
            &transport,
            &connected,
            &entity_to_addr,
            &Some(cell_tx),
            &None,
        )
        .await
    });

    // Take the create, then apply the interleaving before replying.
    let msg = timeout(Duration::from_secs(2), cell_rx.recv())
        .await
        .expect("CreateEntity must not hang")
        .expect("CreateEntity expected");
    let BaseToCellMsg::CreateEntity { reply_tx, .. } = msg else {
        panic!("expected CreateEntity");
    };
    match what {
        MidTransfer::FullyDisconnected => {
            f.entity_to_addr.lock().unwrap().remove(&ENTITY_ID);
            f.connected.lock().unwrap().remove(&f.addr);
        }
        MidTransfer::MappingDropped => {
            f.entity_to_addr.lock().unwrap().remove(&ENTITY_ID);
        }
        MidTransfer::SessionDropped => {
            f.connected.lock().unwrap().remove(&f.addr);
        }
        MidTransfer::RecycledToAnotherSession => {
            let other: SocketAddr = "127.0.0.1:1".parse().unwrap();
            f.entity_to_addr.lock().unwrap().insert(ENTITY_ID, other);
            f.connected.lock().unwrap().remove(&f.addr);
            f.connected.lock().unwrap().insert(other, make_state());
        }
    }
    let _ = reply_tx.send(DEST_SPACE);

    timeout(Duration::from_secs(2), handle)
        .await
        .expect("gate travel must not hang")
        .unwrap()
        .expect("a mid-transfer session change is handled, not propagated");

    let mut reaped = false;
    while let Ok(msg) = cell_rx.try_recv() {
        if let BaseToCellMsg::DestroyEntity { entity_id } = msg {
            assert_eq!(entity_id, ENTITY_ID);
            reaped = true;
        }
    }
    reaped
}

/// Disconnect while the create round-trip is in flight. `entity_to_addr` is
/// cleared by `destroy_client_entities` before it queues its own
/// `DisconnectEntity`, so the cell may process that teardown *before* our
/// `CreateEntity` — leaving a clientless entity parked in the destination
/// space forever. It has to be reaped explicitly.
///
/// Each half of the condition is exercised on its own: checking only the
/// "both gone" case cannot tell `||` from `&&`.
#[tokio::test]
async fn disconnect_during_create_round_trip_reaps_the_destination_entity() {
    assert!(
        reap_outcome_when(55733, MidTransfer::FullyDisconnected).await,
        "a completed disconnect must reap the ghost entity"
    );
    assert!(
        reap_outcome_when(55736, MidTransfer::MappingDropped).await,
        "a dropped entity_to_addr mapping alone must reap — this is the state \
         destroy_client_entities is in when it queues its DisconnectEntity"
    );
    assert!(
        reap_outcome_when(55737, MidTransfer::SessionDropped).await,
        "a dropped session alone must reap"
    );
}

/// The reap's false-positive guard, and the one case where NOT reaping is the
/// correct answer.
///
/// `EntityManager::allocate_id` recycles ids from a free list, and
/// `destroy_client_entities` frees ours on the way out. So while we are
/// blocked on the create oneshot, a second client can legitimately be handed
/// this exact entity id and register it against their own address. Treating
/// "mapped to a different addr" like "unmapped" would then send
/// `DestroyEntity` for a **live player's** entity, leaving them connected, in
/// no space, invisible, with every position update dropped — strictly worse
/// than the leak the reap exists to prevent.
///
/// Regression shape: collapse the check back to
/// `mapped != Some(addr) => reap` and this test fails with a `DestroyEntity`
/// aimed at the new owner's entity.
#[tokio::test]
async fn recycled_entity_id_is_never_reaped_out_from_under_its_new_owner() {
    assert!(
        !reap_outcome_when(55738, MidTransfer::RecycledToAnotherSession).await,
        "an entity id remapped to another live session must NOT be destroyed — \
         that id now belongs to somebody who is still playing"
    );
}

/// Disconnect *before* the base ever sees the transfer. The cell has already
/// torn the entity out of its origin space, so the only correct outcome is to
/// not re-create it anywhere — `handle_gate_travel` must bail at the address
/// lookup without touching the cell.
///
/// (`gate_travel_with_unknown_entity_id_returns_err` in the parent module
/// pins the `Err`; this pins the stronger property that no `CreateEntity`
/// escapes, which is what would resurrect a ghost.)
#[tokio::test]
async fn disconnect_before_base_processing_never_creates_a_destination_entity() {
    let f = fixture(55734).await;
    f.entity_to_addr.lock().unwrap().remove(&ENTITY_ID);
    f.connected.lock().unwrap().remove(&f.addr);
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);

    let result = handle_gate_travel(
        ENTITY_ID,
        DEST_WORLD,
        [0.0; 3],
        [0.0; 3],
        None,
        Some(DEST_SPACE),
        &f.transport,
        &f.connected,
        &f.entity_to_addr,
        // Clone so the local `cell_tx` keeps the channel open — otherwise
        // `try_recv` reports Disconnected and "no message" becomes ambiguous.
        &Some(cell_tx.clone()),
        &None,
    )
    .await;

    assert!(result.is_err(), "an already-disconnected client must Err");
    match cell_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        Ok(_) => panic!("nothing must reach the cell after the client is gone"),
        Err(e) => panic!("unexpected channel state: {e:?}"),
    }
}
