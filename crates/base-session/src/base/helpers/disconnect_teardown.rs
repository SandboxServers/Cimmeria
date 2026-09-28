//! Regression guards for issue #999.
//!
//! `destroy_client_entities` used to return the player's entity id to
//! `EntityManager`'s free list *before* telling the cell to tear the
//! mirrored cell entity down, using a bare `let _ = tx.try_send(...)`. Two
//! failure shapes followed: a `try_send` against a full Base→Cell channel
//! silently dropped the teardown notice, and — independent of channel
//! capacity — a concurrent login could recycle the freed id via
//! `EntityManager::allocate_id`'s FIFO free list before the cell had even
//! seen the disconnect, so the old session's `DisconnectEntity` landed on
//! (and destroyed) the *new* player's cell entity.
//!
//! The fix: the id is only returned to the free list after the cell's
//! `DisconnectEntity` reply confirms teardown, and every way that
//! confirmation can fail to arrive logs a `WARN` naming `entity_id`,
//! `account_id` and `disconnect_reason`.

use super::*;
use tracing::Level;

use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};

fn test_transport() -> Arc<dyn cimmeria_mercury::transport::Transport> {
    Arc::new(TestTransport::new())
}

/// A `connected` + `EntityManager` pair with exactly one player entity,
/// `player_eid`, already allocated from a fresh manager -- so the manager's
/// free-list state matches what `destroy_client_entities` is being asked to
/// retire, not an incidental id.
fn staged_player_session(
    addr: SocketAddr,
    account_id: u32,
    player_eid: u32,
) -> (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<cimmeria_entity::manager::EntityManager>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let mut state = test_default_connected_client_state();
    state.account_id = account_id;
    state.player_entity_id = Some(player_eid);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));

    let mut mgr = cimmeria_entity::manager::EntityManager::new();
    let allocated = mgr.create_entity("SGWPlayer");
    assert_eq!(
        allocated.0 as u32, player_eid,
        "test setup: fixture assumes a fresh EntityManager hands out `player_eid` first"
    );
    let entity_manager = Arc::new(Mutex::new(mgr));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(player_eid, addr)])));
    (connected, entity_manager, entity_to_addr)
}

/// The ordering race at the heart of #999: while the cell has not yet
/// confirmed it tore the old entity down, a concurrent login must not be
/// handed the same id.
///
/// Drives `destroy_client_entities` on its own task against a controlled
/// cell channel, intercepts its `DisconnectEntity` before replying (holding
/// the id "mid-teardown"), and proves a concurrent
/// `EntityManager::create_entity` call — standing in for a racing login —
/// does not receive it. Only after the reply fires does the id recycle.
///
/// Revert shape: reverting to freeing the id up front (before the cell
/// round trip) makes the concurrent `create_entity` call recycle
/// `PLAYER_EID` immediately, failing the `assert_ne!` below.
#[tokio::test]
async fn player_entity_id_is_withheld_from_reuse_until_the_cell_confirms_teardown() {
    const PLAYER_EID: u32 = 1;
    let addr: SocketAddr = "127.0.0.1:55601".parse().unwrap();
    let (connected, entity_manager, entity_to_addr) = staged_player_session(addr, 500, PLAYER_EID);

    let (cell_tx, mut cell_rx) = tokio::sync::mpsc::channel(8);
    let cell_tx = Some(cell_tx);
    let transport = test_transport();

    let task_connected = Arc::clone(&connected);
    let task_entity_manager = Arc::clone(&entity_manager);
    let task_entity_to_addr = Arc::clone(&entity_to_addr);
    let task_transport = Arc::clone(&transport);
    let handle = tokio::spawn(async move {
        destroy_client_entities(
            &task_connected,
            &task_entity_manager,
            addr,
            &cell_tx,
            &task_entity_to_addr,
            &task_transport,
            &None,
            "client_disconnect",
        )
        .await;
    });

    // Intercept the DisconnectEntity and hold its reply -- simulating a
    // cell that has not yet finished tearing the old entity down.
    let msg = tokio::time::timeout(std::time::Duration::from_secs(5), cell_rx.recv())
        .await
        .expect("DisconnectEntity must be sent promptly")
        .expect("the channel must carry the message");
    let BaseToCellMsg::DisconnectEntity {
        entity_id,
        reply_tx,
    } = msg
    else {
        panic!("expected DisconnectEntity");
    };
    assert_eq!(entity_id, PLAYER_EID);

    // A concurrent login racing the still-unconfirmed disconnect must not
    // be handed the id under teardown.
    let concurrent_id = entity_manager.lock().unwrap().create_entity("SGWPlayer");
    assert_ne!(
        concurrent_id.0 as u32, PLAYER_EID,
        "issue #999: a concurrent login must never receive an entity id \
         whose cell-side teardown has not been confirmed yet"
    );

    // Now let the cell confirm teardown.
    let _ = reply_tx.send(());
    tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .expect("destroy_client_entities must not hang waiting on the ack")
        .unwrap();

    // Once confirmed, the id returns to the free list and recycles as usual
    // (allocate_id is FIFO, and the concurrent id above was already handed
    // out, so PLAYER_EID is next).
    let recycled = entity_manager.lock().unwrap().create_entity("SGWPlayer");
    assert_eq!(
        recycled.0 as u32, PLAYER_EID,
        "after the cell confirms teardown the id must recycle normally"
    );
}

/// Acceptance criterion 1: a failed Base→Cell `DisconnectEntity` is never
/// silent. A closed cell channel (standing in for a full or shut-down
/// channel) must WARN with `entity_id`, `account_id` and
/// `disconnect_reason`, and the id must stay out of the free list -- the
/// cell was never told, so nothing can have torn the old entity down.
///
/// Revert shape: reverting to `let _ = tx.try_send(...)` drops this WARN
/// entirely and frees the id unconditionally.
#[tokio::test]
async fn disconnect_entity_send_failure_warns_and_withholds_the_id() {
    const PLAYER_EID: u32 = 1;
    const ACCOUNT_ID: u32 = 909;
    let addr: SocketAddr = "127.0.0.1:55602".parse().unwrap();
    let (connected, entity_manager, entity_to_addr) =
        staged_player_session(addr, ACCOUNT_ID, PLAYER_EID);

    // A closed receiver reproduces the "cell is gone / channel full and the
    // caller cannot wait" shape: the send itself fails.
    let (cell_tx, cell_rx) = tokio::sync::mpsc::channel(8);
    drop(cell_rx);
    let cell_tx = Some(cell_tx);

    let capture = LogCapture::install();
    destroy_client_entities(
        &connected,
        &entity_manager,
        addr,
        &cell_tx,
        &entity_to_addr,
        &test_transport(),
        &None,
        "client_disconnect",
    )
    .await;

    let event = capture
        .find_message(Level::WARN, "DisconnectEntity send failed")
        .expect("a closed cell channel must WARN, not fail silently (issue #999)");
    assert!(
        event.has_field("entity_id", &PLAYER_EID.to_string()),
        "WARN must carry entity_id; got {:?}",
        event.fields
    );
    assert!(
        event.has_field("account_id", &ACCOUNT_ID.to_string()),
        "WARN must carry account_id; got {:?}",
        event.fields
    );
    assert!(
        event.has_field("disconnect_reason", "client_disconnect"),
        "WARN must carry disconnect_reason; got {:?}",
        event.fields
    );

    // Nothing tore the cell entity down, so the id must not be handed out
    // again.
    let next = entity_manager.lock().unwrap().create_entity("SGWPlayer");
    assert_ne!(
        next.0 as u32, PLAYER_EID,
        "a DisconnectEntity send failure must withhold the id from reuse, \
         not free it unconditionally"
    );
}
