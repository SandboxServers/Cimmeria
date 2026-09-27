//! Lifecycle tests for the first-login cinematic AoI hold.

use super::*;
use crate::base::deferred_aoi::DeferredAoiMsg;
use crate::base::world_entry_appearance::{handle_cancel_movie, handle_on_client_ready};
use crate::base::PendingClientReadyInfo;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use tracing::Level;

const PLAYER: u32 = 2;
/// The Castle_CellBlock guard corpse from the repro: `class_id 0`.
const CORPSE: u32 = 100_150;

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
type EntityToAddr = Arc<Mutex<HashMap<u32, SocketAddr>>>;

fn corpse_entered() -> DeferredAoiMsg {
    DeferredAoiMsg::EnteredAoI {
        entity_id: CORPSE,
        class_id: 0,
        position: [-322.5, 73.47, -209.83],
        direction: [0.0; 3],
        level: 1,
        npc_data: None,
        player_data: None,
    }
}

fn session(addr: SocketAddr, state: ConnectedClientState) -> (Connected, EntityToAddr) {
    (
        Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        Arc::new(Mutex::new(HashMap::from([(PLAYER, addr)]))),
    )
}

/// A session mid world-entry, with `first_login` set as given and one
/// entity introduction plus one player-self call already buffered.
fn pre_ready_session(addr: SocketAddr, first_login: i32) -> (Connected, EntityToAddr) {
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(PLAYER);
    state.player_name = Some("Tester".to_string());
    state.pending_client_ready = Some(PendingClientReadyInfo {
        entity_id: PLAYER,
        player_id: 42,
        world_name: "Castle_CellBlock".to_string(),
        appearance_args: vec![0xAB],
        tint_args: vec![0xCD],
        first_login,
    });
    state.deferred_aoi_msgs.push(corpse_entered());
    state
        .deferred_aoi_msgs
        .push(DeferredAoiMsg::EntityMethodCall {
            entity_id: PLAYER,
            method_index: 0x42,
            args: vec![0x01],
        });
    session(addr, state)
}

fn buffered(connected: &Connected, addr: SocketAddr) -> usize {
    connected.lock().unwrap()[&addr].deferred_aoi_msgs.len()
}

fn hold_active(connected: &Connected, addr: SocketAddr) -> bool {
    connected.lock().unwrap()[&addr]
        .cinematic_aoi_hold
        .is_some()
}

/// The repro, end to end: first login, a `class_id 0` entity already
/// waiting in the pre-ready buffer. `onClientReady` must NOT flush it
/// alongside `onPlayMovie`; it goes out only once the movie has had time
/// to finish. Reverting `begin` in `handle_on_client_ready` flushes it at
/// ready and fails the first buffer assertion.
#[tokio::test(start_paused = true)]
async fn first_login_holds_entity_intro_until_the_movie_has_run_out() {
    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:55810".parse().unwrap();
    let (connected, entity_to_addr) = pre_ready_session(addr, /*first_login=*/ 1);
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();

    handle_on_client_ready(
        addr,
        [0u8; 32],
        &connected,
        &None,
        &transport,
        &entity_to_addr,
        &None,
    )
    .await
    .unwrap();

    assert!(hold_active(&connected, addr), "first login starts a hold");
    assert!(
        matches!(
            connected.lock().unwrap()[&addr]
                .deferred_aoi_msgs
                .as_slice(),
            [DeferredAoiMsg::EnteredAoI {
                entity_id: CORPSE,
                ..
            }]
        ),
        "the entity introduction stays buffered; the player-self call has gone"
    );
    let sent_at_ready = typed_transport.len();

    // Just short of the hold: still nothing for the corpse.
    tokio::time::sleep(HOLD_DURATION - Duration::from_millis(50)).await;
    assert_eq!(buffered(&connected, addr), 1, "held for the whole movie");
    assert_eq!(typed_transport.len(), sent_at_ready);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!hold_active(&connected, addr), "timeout lifts the hold");
    assert_eq!(buffered(&connected, addr), 0);
    assert_eq!(
        typed_transport.len() - sent_at_ready,
        2,
        "the held introduction flushes as its phase-1 + phase-2 bundles"
    );
    let released = capture
        .find_message(Level::INFO, "Cinematic AoI hold: released")
        .expect("release must be logged for the next repro");
    assert!(released.has_field("reason", "timeout"), "{released:#?}");
    assert!(released.has_field("flushed", "1"), "{released:#?}");
}

/// No cinematic, no hold: everything buffered pre-ready flushes at
/// `onClientReady` as before.
#[tokio::test(start_paused = true)]
async fn returning_player_gets_no_hold_and_a_full_flush_at_ready() {
    let addr: SocketAddr = "127.0.0.1:55811".parse().unwrap();
    let (connected, entity_to_addr) = pre_ready_session(addr, /*first_login=*/ 0);
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());

    handle_on_client_ready(
        addr,
        [0u8; 32],
        &connected,
        &None,
        &transport,
        &entity_to_addr,
        &None,
    )
    .await
    .unwrap();

    assert!(!hold_active(&connected, addr));
    assert_eq!(buffered(&connected, addr), 0);
}

/// Esc ends the movie early, so the hold ends with it — and traffic that
/// depends on the held create follows it out rather than being lost.
#[tokio::test(start_paused = true)]
async fn cancel_movie_releases_the_hold_and_flushes_in_order() {
    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:55812".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(PLAYER);
    begin(&mut state);
    state.deferred_aoi_msgs.push(corpse_entered());
    state
        .deferred_aoi_msgs
        .push(DeferredAoiMsg::WitnessEntityMethod {
            entity_id: CORPSE,
            method_index: 3,
            args: vec![0x00, 0x00, 0x40, 0x00],
            entity_is_player: false,
        });
    let (connected, entity_to_addr) = session(addr, state);
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();

    handle_cancel_movie(&transport, addr, PLAYER, &connected, &entity_to_addr).await;

    assert!(!hold_active(&connected, addr));
    assert_eq!(buffered(&connected, addr), 0);
    assert_eq!(
        typed_transport.len(),
        3,
        "phase-1 bundle, phase-2 bundle, then the held witness method \
         (no cached appearance in this fixture, so no resend packet)"
    );
    let released = capture
        .find_message(Level::INFO, "Cinematic AoI hold: released")
        .expect("release must be logged");
    assert!(
        released.has_field("reason", "cancel_movie"),
        "{released:#?}"
    );
    assert!(released.has_field("flushed", "2"), "{released:#?}");
}

/// A timeout task that outlives its hold must not end a later one: Esc
/// the movie, back out to character select, make another new character
/// and enter the world again, all inside the first hold's 16 seconds.
#[tokio::test(start_paused = true)]
async fn stale_timeout_does_not_release_a_later_hold() {
    let addr: SocketAddr = "127.0.0.1:55813".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(PLAYER);
    let stale = begin(&mut state).token;
    let current = begin(&mut state).token;
    assert_ne!(stale, current);
    state.deferred_aoi_msgs.push(corpse_entered());
    let (connected, entity_to_addr) = session(addr, state);
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();

    release(
        Some(stale),
        ReleaseReason::Timeout,
        PLAYER,
        addr,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    assert!(hold_active(&connected, addr), "the later hold survives");
    assert_eq!(buffered(&connected, addr), 1);
    assert!(typed_transport.is_empty());
}

/// `cancelMovie` and the timeout can both fire at the 16-second boundary.
/// The owner drains the buffer, then awaits its sends with the buffer
/// momentarily empty — exactly the state a second releaser used to read as
/// "nothing left, lift the hold", letting live traffic overtake the owner's
/// in-flight creates. A claimed hold must survive a second release attempt.
/// Dropping the `releasing` check lifts the hold here.
#[tokio::test(start_paused = true)]
async fn second_releaser_leaves_a_claimed_hold_alone() {
    let addr: SocketAddr = "127.0.0.1:55814".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(PLAYER);
    let hold = begin(&mut state);
    // The owner has claimed the hold and drained the buffer; it is awaiting
    // its sends.
    state.cinematic_aoi_hold.as_mut().unwrap().releasing = true;
    let (connected, entity_to_addr) = session(addr, state);
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();

    release_on_cancel(PLAYER, addr, &transport, &connected, &entity_to_addr).await;
    assert!(
        hold_active(&connected, addr),
        "cancelMovie must not lift a hold another task is still flushing"
    );

    release(
        Some(hold.token),
        ReleaseReason::Timeout,
        PLAYER,
        addr,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    assert!(
        hold_active(&connected, addr),
        "nor may the timeout, even with the matching token"
    );
}

/// `handle_on_client_ready` arms the timeout only after its DB reads and
/// cell sends. The 16 seconds run from `begin`, so time spent before arming
/// comes off the wait instead of stretching the hold past the movie.
/// `sleep(HOLD_DURATION)` in `arm_timeout` leaves the hold up at the first
/// assertion after the deadline.
#[tokio::test(start_paused = true)]
async fn timeout_runs_from_hold_start_not_from_arming() {
    let addr: SocketAddr = "127.0.0.1:55815".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(PLAYER);
    let hold = begin(&mut state);
    state.deferred_aoi_msgs.push(corpse_entered());
    let (connected, entity_to_addr) = session(addr, state);
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());

    // A slow dependency between `begin` and `arm_timeout`.
    let arming_delay = Duration::from_secs(10);
    tokio::time::sleep(arming_delay).await;
    arm_timeout(hold, PLAYER, addr, &transport, &connected, &entity_to_addr);

    tokio::time::sleep(HOLD_DURATION - arming_delay - Duration::from_millis(50)).await;
    assert!(hold_active(&connected, addr), "still inside the 16 seconds");

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !hold_active(&connected, addr),
        "released 16 s after the hold began, not 16 s after arming"
    );
    assert_eq!(buffered(&connected, addr), 0);
}
