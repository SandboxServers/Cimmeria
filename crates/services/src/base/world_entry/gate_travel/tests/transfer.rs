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
use std::time::Duration;
use tokio::time::timeout;

const ENTITY_ID: u32 = 42;
const DEST_SPACE: u32 = 0x0001_ABCD;
const DEST_WORLD: &str = "Castle_CellBlock";

struct Fixture {
    transport: Arc<dyn Transport>,
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
    Fixture {
        transport: make_socket().await,
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

    match cell_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        Ok(BaseToCellMsg::CreateEntity { .. }) => panic!(
            "fail-closed abort must happen BEFORE the cell is told to create \
             the destination entity"
        ),
        Ok(_) => panic!("the aborted transfer must not message the cell at all"),
        Err(e) => panic!("unexpected channel state: {e:?}"),
    }
    assert!(
        f.connected
            .lock()
            .unwrap()
            .get(&f.addr)
            .unwrap()
            .pending_world_entry
            .is_none(),
        "the aborted transfer must not populate pending_world_entry"
    );
}

/// Disconnect while the create round-trip is in flight. `entity_to_addr` is
/// cleared by `destroy_client_entities` before it queues its own
/// `DisconnectEntity`, so the cell may process that teardown *before* our
/// `CreateEntity` — leaving a clientless entity parked in the destination
/// space forever. It has to be reaped explicitly.
#[tokio::test]
async fn disconnect_during_create_round_trip_reaps_the_destination_entity() {
    let f = fixture(55733).await;
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

    // Take the create, then simulate the disconnect landing before we reply.
    let msg = timeout(Duration::from_secs(2), cell_rx.recv())
        .await
        .expect("CreateEntity must not hang")
        .expect("CreateEntity expected");
    let BaseToCellMsg::CreateEntity { reply_tx, .. } = msg else {
        panic!("expected CreateEntity");
    };
    f.entity_to_addr.lock().unwrap().remove(&ENTITY_ID);
    f.connected.lock().unwrap().remove(&f.addr);
    let _ = reply_tx.send(DEST_SPACE);

    timeout(Duration::from_secs(2), handle)
        .await
        .expect("gate travel must not hang")
        .unwrap()
        .expect("a mid-transfer disconnect is handled, not propagated");

    match timeout(Duration::from_secs(2), cell_rx.recv())
        .await
        .expect("DestroyEntity must not hang")
    {
        Some(BaseToCellMsg::DestroyEntity { entity_id }) => assert_eq!(
            entity_id, ENTITY_ID,
            "the ghost entity in the destination space must be reaped"
        ),
        _ => panic!(
            "a mid-transfer disconnect must reap the destination entity with \
             DestroyEntity — otherwise it sits in the destination space with \
             no client forever"
        ),
    }
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
