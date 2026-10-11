//! Tests for the player appearance refresh (`appearance.rs`): the cache
//! write, the witness fan-out, the skip and degraded outcome rows
//! (TG-ITM-01) and the abort after a failed load (TG-ITM-14).

use super::*;
use crate::test_support::TestTransport;
use cimmeria_mercury::encryption::MercuryEncryption;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::time::Instant;

fn make_connected_state(account_id: u32) -> ConnectedClientState {
    ConnectedClientState {
        enc: MercuryEncryption::from_session_key([0u8; 32]),
        key: [0u8; 32],
        enc_version: cimmeria_mercury::encryption::EncryptionVersion::V1,
        account_id,
        account_name: None,
        access_level: 0,
        session_kind: None,
        dnd_message: None,
        afk_message: None,
        ignore: Default::default(),
        char_list_sent: false,
        world_entry_sent: false,
        pending_player_entity_id: None,
        player_entity_id: None,
        next_seq: Arc::new(AtomicU32::new(1)),
        next_seq_unreliable: Arc::new(AtomicU32::new(0)),
        pending_acks: Arc::new(Mutex::new(Vec::new())),
        last_recv: Arc::new(Mutex::new(Instant::now())),
        connected_at: Instant::now(),
        account_entity_id: 1,
        next_data_id: 0,
        pending_world_entry: None,
        pending_player_load_data: None,
        pending_map_loaded: None,
        pending_client_ready: None,
        deferred_aoi_msgs: Vec::new(),
        cached_appearance_args: None,
        cached_tint_args: None,
        weapon_holstered: true,
        cancelled: Arc::new(AtomicBool::new(false)),
        cinematic_spam_cancel: Arc::new(AtomicBool::new(false)),
        cinematic_aoi_hold: None,
        listed_online: false,
        rate_limits: Default::default(),
        org_invites: Default::default(),
        player_name: None,
        player_level: None,
        player_archetype: None,
        player_class_id: None,
        player_alignment: None,
        world_name: None,
        player_xp: None,
        player_training_points: None,
        active_player_id: None,
        pending_destination_ring_id: None,
        channel: Mutex::new(cimmeria_mercury::channel::Channel::new(
            "127.0.0.1:9999".parse().unwrap(),
        )),
        extensions: Default::default(),
        plugins: Default::default(),
    }
}

fn make_transport() -> Arc<dyn Transport> {
    Arc::new(TestTransport::new())
}

/// Regression for the "F1-F4 changes the active slot but the player
/// keeps holding the old weapon" bug: the appearance refresh path
/// MUST populate `cached_appearance_args` on the connected client
/// state so that the next AoI re-broadcast (and any subsequent
/// world_entry replay) ships the new appearance.
///
/// Runs against `db_pool = None` so it doesn't need DATABASE_URL —
/// `query_player_load_data` returns the default-sentinel payload in
/// that mode, which is enough to exercise the cache-set path.
#[tokio::test]
async fn refresh_player_appearance_populates_cached_args() {
    let transport = make_transport();
    let addr: SocketAddr = "127.0.0.1:55700".parse().unwrap();

    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(42u32, addr);
        m
    }));
    let connected = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(addr, make_connected_state(0x1234));
        m
    }));

    // Pre-condition: cached args are empty so the assertion below is
    // genuinely checking the helper wrote them.
    {
        let clients = connected.lock().unwrap();
        assert!(
            clients.get(&addr).unwrap().cached_appearance_args.is_none(),
            "test setup invariant: cache must start empty",
        );
    }

    refresh_player_appearance(
        42,
        100,
        &None,
        &transport,
        &connected,
        &entity_to_addr,
        &None,
    )
    .await;

    let clients = connected.lock().unwrap();
    let state = clients.get(&addr).expect("connected state must exist");
    assert!(
        state.cached_appearance_args.is_some(),
        "refresh_player_appearance must populate cached_appearance_args; \
         without this, a later world_entry/AoI replay would still ship \
         the pre-refresh weapon visual",
    );
}

/// Regression guard for "other players never see a weapon draw / gear
/// change": the refresh used to reach only the player's own client. It
/// must now also hand the freshly built `BeingAppearance` args to the
/// cell for witness fan-out — the same bytes it cached for late
/// arrivals, so every viewer converges on one appearance.
#[tokio::test]
async fn refresh_player_appearance_asks_the_cell_to_fan_out_to_witnesses() {
    let transport = make_transport();
    let addr: SocketAddr = "127.0.0.1:55701".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(42u32, addr)])));
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        make_connected_state(0x1234),
    )])));
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(4);

    refresh_player_appearance(
        42,
        100,
        &None,
        &transport,
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
    )
    .await;

    let cached = connected
        .lock()
        .unwrap()
        .get(&addr)
        .unwrap()
        .cached_appearance_args
        .clone()
        .expect("refresh caches the args");
    match cell_rx
        .try_recv()
        .expect("refresh must ask the cell to fan out")
    {
        BaseToCellMsg::BroadcastToWitnesses {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 42);
            assert_eq!(method_index, method_idx::BEING_APPEARANCE);
            assert_eq!(args, cached, "witnesses get the bytes that were cached");
        }
        _ => panic!("expected BroadcastToWitnesses"),
    }
}

/// When the entity has no addr mapping (already disconnected, never
/// connected), the helper must return cleanly without panicking and
/// without modifying any shared state. This is the realistic shape
/// of a logout race: the cell sends `ActiveSlotUpdate`, the addr
/// is already torn down by the time the base processes it.
#[tokio::test]
async fn refresh_player_appearance_no_addr_is_noop() {
    let transport = make_transport();
    let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> = Arc::new(Mutex::new(HashMap::new()));
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Should not panic; should not insert anything into `connected`.
    refresh_player_appearance(
        42,
        100,
        &None,
        &transport,
        &connected,
        &entity_to_addr,
        &None,
    )
    .await;

    assert!(
        connected.lock().unwrap().is_empty(),
        "no-addr path must not synthesize a client state entry",
    );
}

/// TG-ITM-01: a refresh that gives up says why. Offline mode
/// (`db_pool = None`) is only a DEBUG; an entity with no session, or no
/// client state, is a WARN; the same miss after its session was torn down
/// is only a DEBUG. (A failed load with a live pool is TG-ITM-14's test.)
#[tokio::test]
async fn appearance_refresh_logs_degraded_and_skips() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    // A player id no other test uses, so the global name cache can't collide.
    const PLAYER: i32 = 0x7A10_0100;
    known_names::remember_player(PLAYER, "TgItm01Player");
    let capture = LogCapture::install();
    let transport = make_transport();
    let addr: SocketAddr = "127.0.0.1:55702".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(42u32, addr)])));
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        make_connected_state(0x1234),
    )])));
    let errors = || {
        capture
            .all()
            .iter()
            .filter(|c| c.level == Level::ERROR)
            .count()
    };

    // db_pool = None: the load returns the default payload (player_id 0),
    // which is not the requested player. Offline mode is expected, so it
    // logs DEBUG and no ERROR.
    let errors_before = errors();
    refresh_player_appearance(
        42,
        PLAYER,
        &None,
        &transport,
        &connected,
        &entity_to_addr,
        &None,
    )
    .await;
    assert!(
        capture
            .find_message(Level::DEBUG, "sending default appearance")
            .is_some_and(|c| c.has_field("entity_name", "TgItm01Player")),
        "an offline load must log appearance_refresh_degraded at DEBUG, naming the \
         entity. Captured: {:#?}",
        capture.all()
    );
    assert_eq!(
        errors_before,
        errors(),
        "db_pool = None must not log an ERROR. Captured: {:#?}",
        capture.all()
    );

    // The offline refresh still sends, and logs its success row marked
    // degraded. (A live pool that fails the load aborts instead: see
    // `failed_load_does_not_cache_or_send_default_appearance`.)
    assert!(
        capture
            .find_message(Level::DEBUG, "appearance refreshed")
            .is_some_and(
                |c| c.fields.get("degraded").map(String::as_str) == Some("true")
                    && c.has_field("entity_name", "TgItm01Player")
            ),
        "a degraded refresh must log appearance_refreshed with degraded = true. Captured: {:#?}",
        capture.all()
    );

    // A mapped entity whose address has no client state is a WARN.
    let orphan: u32 = 0x7A10_0003;
    let orphan_addr: SocketAddr = "127.0.0.1:55703".parse().unwrap();
    entity_to_addr.lock().unwrap().insert(orphan, orphan_addr);
    refresh_player_appearance(
        orphan,
        100,
        &None,
        &transport,
        &connected,
        &entity_to_addr,
        &None,
    )
    .await;
    assert!(
        capture
            .find_event(
                Level::WARN,
                "refresh_player_appearance: skipped",
                "client_state_missing"
            )
            .is_some(),
        "a mapped entity without client state must log the skip at WARN. Captured: {:#?}",
        capture.all()
    );

    // An entity with no transport addr and no departure record is a WARN.
    let unmapped: u32 = 0x7A10_0001;
    refresh_player_appearance(
        unmapped,
        100,
        &None,
        &transport,
        &connected,
        &entity_to_addr,
        &None,
    )
    .await;
    assert!(
        capture
            .find_event(
                Level::WARN,
                "refresh_player_appearance: skipped",
                "entity_to_addr_miss"
            )
            .is_some(),
        "an unmapped entity must log the skip at WARN. Captured: {:#?}",
        capture.all()
    );

    // After its session ended, the same miss is a DEBUG and no new WARN.
    let departed: u32 = 0x7A10_0002;
    helpers::note_witness_departed(departed);
    let warns_before = capture
        .all()
        .iter()
        .filter(|c| c.level == Level::WARN)
        .count();
    refresh_player_appearance(
        departed,
        100,
        &None,
        &transport,
        &connected,
        &entity_to_addr,
        &None,
    )
    .await;
    assert!(
        capture
            .find_event(
                Level::DEBUG,
                "refresh_player_appearance: skipped",
                "witness_session_ended"
            )
            .is_some(),
        "a departed witness must log the skip at DEBUG. Captured: {:#?}",
        capture.all()
    );
    let warns_after = capture
        .all()
        .iter()
        .filter(|c| c.level == Level::WARN)
        .count();
    assert_eq!(
        warns_before, warns_after,
        "a departed witness must not add a WARN"
    );
}

/// TG-ITM-14 / D-TG11: a player load that fails with a live pool is
/// retried once, and when the retry fails too the refresh aborts. The
/// default "naked human male" payload must not replace the cached
/// appearance, reach the player's own client, or go to the cell for
/// witness fan-out; the abort is a WARN naming the player. Before the
/// fix, the default payload was cached and sent both ways.
#[tokio::test]
async fn failed_load_does_not_cache_or_send_default_appearance() {
    use crate::test_support::LogCapture;
    use std::time::Duration;
    use tracing::Level;

    // A player id no other test uses, so the global name cache can't collide.
    const PLAYER: i32 = 0x7A14_0100;
    known_names::remember_player(PLAYER, "TgItm14Player");
    let capture = LogCapture::install();
    let test_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = test_transport.clone();
    let addr: SocketAddr = "127.0.0.1:55710".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(42u32, addr)])));
    // The last good appearance, as world entry or an earlier refresh left it.
    let previous = vec![0xA5u8, 0x5A, 0x01];
    let mut state = make_connected_state(0x1234);
    state.cached_appearance_args = Some(previous.clone());
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(4);

    // A live pool that cannot connect fails the load for real. Options,
    // not a URL string: the URL guard rejects any `postgres://` literal
    // other than the dead 127.0.0.1:1.
    let pool = Arc::new(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_millis(50))
            .connect_lazy_with(
                sqlx::postgres::PgConnectOptions::new()
                    .host("127.0.0.1")
                    .port(1)
                    .username("nobody")
                    .password("nobody")
                    .database("none"),
            ),
    );
    refresh_player_appearance(
        42,
        PLAYER,
        &Some(pool),
        &transport,
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
    )
    .await;

    assert_eq!(
        connected
            .lock()
            .unwrap()
            .get(&addr)
            .unwrap()
            .cached_appearance_args
            .as_deref(),
        Some(previous.as_slice()),
        "a failed load must leave the last good appearance cached"
    );
    assert!(
        cell_rx.try_recv().is_err(),
        "a failed load must not ask the cell to fan the default appearance out"
    );
    assert_eq!(
        test_transport.send_count_to(addr),
        0,
        "a failed load must not send the default appearance to the player's client"
    );

    // D-TG11: one retry, so the loader failed exactly twice.
    let load_failures = capture
        .all()
        .iter()
        .filter(|c| {
            c.level == Level::ERROR
                && c.has_field("reason", "player_query_failed")
                && c.has_field("player_name", "TgItm14Player")
        })
        .count();
    assert_eq!(
        load_failures,
        2,
        "the load must be retried exactly once. Captured: {:#?}",
        capture.all()
    );
    assert!(
        capture
            .find_message(Level::WARN, "keeping the last appearance")
            .is_some_and(|c| c.has_field("event", "appearance_refresh_aborted")
                && c.has_field("player_name", "TgItm14Player")),
        "an aborted refresh must log appearance_refresh_aborted at WARN, naming the \
         player. Captured: {:#?}",
        capture.all()
    );
    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.has_field("event", "appearance_refreshed")),
        "an aborted refresh must not log the success row. Captured: {:#?}",
        capture.all()
    );
}
