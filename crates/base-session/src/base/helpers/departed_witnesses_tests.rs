//! Regression guards for the teardown race behind the Discord
//! `entity_to_addr_miss` bursts (colo, 2026-09-29).
//!
//! A logOff or a disconnect unmaps the player's witness id at once, and the
//! position relays the cell had already queued for it arrive afterwards. On
//! the colo one logOff put 23 WARNs into the errors channel in half a
//! millisecond (witness 1, `entity_count_in_map` 0). Those misses log at
//! DEBUG with `reason = "witness_session_ended"`; a miss for any other
//! witness still WARNs.
//!
//! The departed record is process-wide, so every test here uses its own
//! witness id, far from the small ids other tests allocate.

use super::*;
use tracing::Level;

use crate::test_support::{
    test_default_connected_client_state, LogCapture, LogCaptureGuard, TestTransport,
};

type Maps = (
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
);

fn empty_maps() -> Maps {
    (
        Arc::new(TestTransport::default()),
        Arc::new(Mutex::new(HashMap::new())),
        Arc::new(Mutex::new(HashMap::new())),
    )
}

fn misses(capture: &LogCaptureGuard, level: Level, reason: &str) -> usize {
    capture
        .all()
        .into_iter()
        .filter(|c| c.level == level && c.has_field("reason", reason))
        .count()
}

/// Every witness-send helper logs a departed witness's miss at DEBUG and
/// posts no WARN. Revert-proof: dropping the `witness_recently_departed`
/// branch in `log_addr_miss` brings back three WARNs.
#[tokio::test]
async fn a_departed_witness_miss_is_debug_on_every_send_path() {
    const WITNESS: u32 = 0x5EED_0A01;
    let (transport, connected, entity_to_addr) = empty_maps();
    note_witness_departed(WITNESS);
    let capture = LogCapture::install();

    let unreliable = send_to_witness(
        &transport,
        &connected,
        &entity_to_addr,
        WITNESS,
        |_, _, _, _| vec![],
    )
    .await;
    let reliable = send_to_witness_reliable(
        &transport,
        &connected,
        &entity_to_addr,
        WITNESS,
        |_, _, _, _| vec![],
    )
    .await;
    let bundle = send_bundle_to_witness_reliable(
        &transport,
        &connected,
        &entity_to_addr,
        WITNESS,
        cimmeria_mercury::channel_bundle::ChannelBundle::new(true),
    )
    .await;

    // The outcome a caller sees does not change: the packet still had no
    // address.
    assert_eq!(unreliable, WitnessSendOutcome::AddrUnresolved);
    assert_eq!(reliable, WitnessSendOutcome::AddrUnresolved);
    assert_eq!(bundle, BundleSendOutcome::AddrUnresolved);
    assert_eq!(
        misses(&capture, Level::WARN, "entity_to_addr_miss"),
        0,
        "a departed witness must not WARN: {:#?}",
        capture.all()
    );
    assert_eq!(
        misses(&capture, Level::DEBUG, "witness_session_ended"),
        3,
        "{:#?}",
        capture.all()
    );
}

/// The control: a witness that did not just end its session is a live
/// player the server cannot reach, and still WARNs.
#[tokio::test]
async fn a_witness_that_never_departed_still_warns() {
    const WITNESS: u32 = 0x5EED_0A02;
    let (transport, connected, entity_to_addr) = empty_maps();
    note_witness_departed(0x5EED_0A03); // someone else's teardown
    let capture = LogCapture::install();

    send_to_witness(
        &transport,
        &connected,
        &entity_to_addr,
        WITNESS,
        |_, _, _, _| vec![],
    )
    .await;

    let warn = capture
        .find_event(
            Level::WARN,
            "no client addr for witness",
            "entity_to_addr_miss",
        )
        .unwrap_or_else(|| panic!("live witness miss must WARN: {:#?}", capture.all()));
    assert!(warn.has_field("witness_id", &WITNESS.to_string()));
    assert_eq!(misses(&capture, Level::DEBUG, "witness_session_ended"), 0);
}

/// The disconnect teardown records the departure itself: after
/// `destroy_client_entities`, a position relay still in flight for the
/// player logs at DEBUG. Revert-proof: unmapping with a bare `remove` again
/// (no `note_witness_departed`) turns it back into a WARN.
#[tokio::test]
async fn destroy_client_entities_marks_the_player_departed() {
    const PLAYER: u32 = 0x5EED_0A04;
    let addr: SocketAddr = "127.0.0.1:47104".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.account_id = 0x5EED;
    state.player_entity_id = Some(PLAYER);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(PLAYER, addr)])));
    let entity_manager = Arc::new(Mutex::new(cimmeria_entity::manager::EntityManager::new()));
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    destroy_client_entities(
        &connected,
        &entity_manager,
        addr,
        &None,
        &entity_to_addr,
        &transport,
        &None,
        "client_disconnect",
    );
    assert!(
        entity_to_addr.lock().unwrap().get(&PLAYER).is_none(),
        "the teardown unmaps the player"
    );
    assert!(witness_recently_departed(PLAYER));

    let capture = LogCapture::install();
    send_to_witness(
        &transport,
        &connected,
        &entity_to_addr,
        PLAYER,
        |_, _, _, _| vec![],
    )
    .await;
    assert_eq!(
        misses(&capture, Level::WARN, "entity_to_addr_miss"),
        0,
        "{:#?}",
        capture.all()
    );
    assert_eq!(misses(&capture, Level::DEBUG, "witness_session_ended"), 1);
}
