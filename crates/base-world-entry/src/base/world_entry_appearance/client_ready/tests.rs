//! `handle_on_client_ready` regression guards: the base→cell handoff
//! negative logs and the `first_login` UPDATE `rows_affected == 0` branch.

use super::*;

// ──────────────────────────────────────────────────────────────────
// Negative-logging regression guards: onClientReady
// cell_tx sends. ConnectEntity / InitPlayerState / AdvanceRing-
// Destination are critical handoffs from base to cell — dropping
// any of them strands the player in a half-loaded state. The
// guards drive `handle_on_client_ready` with a closed cell→base
// channel and assert each of the three ERROR logs fires
// independently so a partial revert (only one of three) is also
// caught.
// ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn on_client_ready_errors_each_cell_tx_send_independently_when_closed() {
    use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
    use tracing::Level;

    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:55600".parse().unwrap();
    let entity_id: u32 = 8888;
    let key = [0u8; 32];

    // Pre-stage pending_client_ready + pending_destination_ring_id
    // so the function reaches all three cell_tx send sites.
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    state.pending_client_ready = Some(crate::base::PendingClientReadyInfo {
        entity_id,
        player_id: 42,
        world_name: "Agnos".to_string(),
        appearance_args: vec![0xAB],
        tint_args: vec![0xCD],
        first_login: 0, // skip the cinematic + DB UPDATE branch
    });
    // Non-None forces the AdvanceRingDestination send to fire.
    state.pending_destination_ring_id = Some(17);
    state.player_name = Some("Tester".to_string());

    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(addr, state);
        m
    }));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    // Closed cell→base channel: all three sends will SendError.
    let (tx, rx) = mpsc::channel::<BaseToCellMsg>(8);
    drop(rx);
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = Some(tx);

    // db_pool=None so DB-querying branches return empty/default
    // without needing a live Postgres.
    let _ = handle_on_client_ready(
        addr,
        key,
        &connected,
        &cell_tx,
        &transport,
        &entity_to_addr,
        &None,
    )
    .await;

    // All three ERRORs must fire — assert each independently so a
    // partial revert (e.g. only one of three reverted to `let _`)
    // is also caught.
    assert!(
        capture
            .find_message(Level::ERROR, "ConnectEntity: base→cell send failed")
            .is_some(),
        "negative-logging convention: ConnectEntity ERROR missing. Captured: {:#?}",
        capture.all()
    );
    assert!(
        capture
            .find_message(Level::ERROR, "InitPlayerState: base→cell send failed")
            .is_some(),
        "negative-logging convention: InitPlayerState ERROR missing"
    );
    assert!(
        capture
            .find_message(
                Level::ERROR,
                "AdvanceRingDestination: base→cell send failed"
            )
            .is_some(),
        "negative-logging convention: AdvanceRingDestination ERROR missing (requires \
         pending_destination_ring_id to be Some; check test staging)"
    );
}

/// Live-DB regression guard for the `first_login` UPDATE's
/// `rows_affected == 0` branch. Stages `pending.first_login = 1`
/// with `pending.player_id` pointing at a `sgw_player` row that
/// does NOT exist; the UPDATE succeeds (no SQL error) but
/// touches zero rows. Pre-#304 this was silent — the flag stayed
/// set in some other table or the cinematic just re-fired every
/// login with no signal. The handler now emits an ERROR naming
/// `rows_affected=0` + `expected=1` so a single ops query
/// catches it.
///
/// Sentinel ID is well below `i32::MAX` and outside the live-DB
/// fixture base ranges; nothing to clean up because the test
/// does NOT insert the row.
#[tokio::test]
async fn first_login_update_errors_when_player_row_missing() {
    use crate::test_support::{
        require_db_or_skip, test_default_connected_client_state, LogCapture, TestTransport,
    };
    use tracing::Level;

    let pool = require_db_or_skip!();
    let capture = LogCapture::install();

    // Sentinel — must NOT exist in sgw_player. Picked outside
    // every other test base (TEST_BASE families peak around
    // 0x7000_0FFF).
    const MISSING_PLAYER_ID: i32 = 0x7FFE_FF99;

    // The handler runs a real UPDATE against this id. Refuse to go on if a
    // row owns it: the test would clear some character's `first_login` and
    // stop exercising the zero-row branch.
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sgw_player WHERE player_id = $1")
        .bind(MISSING_PLAYER_ID)
        .fetch_one(&pool)
        .await
        .expect("probe sgw_player for the sentinel id");
    assert_eq!(
        existing, 0,
        "sentinel player_id {MISSING_PLAYER_ID:#x} must not exist in sgw_player"
    );

    let addr: SocketAddr = "127.0.0.1:55700".parse().unwrap();
    let entity_id: u32 = 9999;
    let key = [0u8; 32];

    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    state.pending_client_ready = Some(crate::base::PendingClientReadyInfo {
        entity_id,
        player_id: MISSING_PLAYER_ID,
        world_name: "Agnos".to_string(),
        appearance_args: vec![0xAB],
        tint_args: vec![0xCD],
        first_login: 1, // forces the cinematic + UPDATE branch
    });
    state.player_name = Some("Tester".to_string());

    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(addr, state);
        m
    }));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    // Open cell→base channel; we don't care about its sends for this test.
    let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(32);
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = Some(tx);
    let db_pool = Some(Arc::new(pool));

    let _ = handle_on_client_ready(
        addr,
        key,
        &connected,
        &cell_tx,
        &transport,
        &entity_to_addr,
        &db_pool,
    )
    .await;

    let event = capture
        .find_message(Level::ERROR, "first_login flag NOT cleared")
        .expect(
            "negative-logging convention: missing player row must emit \
             ERROR with rows_affected=0",
        );
    assert!(
        event.has_field("rows_affected", "0"),
        "rows_affected field must be 0 — pin the structured shape so \
         ops can query (rows_affected != expected): {event:#?}"
    );
    assert!(
        event.has_field("expected", "1"),
        "expected field must be 1 — pin the paired structured field: {event:#?}"
    );
    assert!(
        event.has_field("player_id", &MISSING_PLAYER_ID.to_string()),
        "player_id field must carry the missing id for ops triage: {event:#?}"
    );
}

/// Live DB, the crafting login-sync guard: `onClientReady` pushes the player's
/// stored crafting state (discipline, expertise, the five paradigm levels,
/// blueprints, the ASP total) and the crafting options (140) as one bundle
/// to the player's own client. Removing the `push_crafting_on_login` call
/// leaves no such packet at any sequence number, and a separate options
/// push instead of the one in the bundle leaves no packet of this shape.
#[tokio::test]
async fn on_client_ready_pushes_the_stored_crafting_state() {
    use crate::test_support::{
        require_db_or_skip, test_default_connected_client_state, TestTransport,
    };
    use cimmeria_base_session::base::crafting::sync::build_login_bundle;
    use cimmeria_entity::crafting::CraftingState;
    use cimmeria_mercury::encryption::EncryptionVersion;
    use cimmeria_mercury::packet::{FLAG_ON_CHANNEL, FLAG_RELIABLE};
    use cimmeria_wire::crafting::CraftingOptions;

    let pool = require_db_or_skip!();
    // Crafting campaign sentinels (`0x7000_Cxxx`).
    const ACCOUNT_ID: i32 = 0x7000_CF00;
    const PLAYER_ID: i32 = 0x7000_CF01;
    let cleanup = || async {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(PLAYER_ID)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(ACCOUNT_ID)
            .execute(&pool)
            .await;
    };
    cleanup().await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT_ID)
        .bind(format!("cr03-login-{ACCOUNT_ID}"))
        .execute(&pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, pos_x, pos_y, pos_z, skin_color_id, \
            discipline_ids, racial_paradigm_levels, applied_science_points, blueprint_ids) \
         VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
            0.0, 0.0, 0.0, 0, '{78}', '{5,2,1,1,1}', 2, '{25}')",
    )
    .bind(ACCOUNT_ID)
    .bind(PLAYER_ID)
    .bind(format!("cr03-login-{PLAYER_ID}"))
    .execute(&pool)
    .await
    .expect("insert player");
    sqlx::query(
        "INSERT INTO sgw_player_discipline_expertise (player_id, discipline_id, expertise) \
         VALUES ($1, 78, 33)",
    )
    .bind(PLAYER_ID)
    .execute(&pool)
    .await
    .expect("insert expertise");

    let addr: SocketAddr = "127.0.0.1:55710".parse().unwrap();
    let entity_id: u32 = 9998;
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    state.pending_client_ready = Some(crate::base::PendingClientReadyInfo {
        entity_id,
        player_id: PLAYER_ID,
        world_name: "Agnos".to_string(),
        appearance_args: vec![0xAB],
        tint_args: vec![0xCD],
        first_login: 0,
    });
    state.player_name = Some("Tester".to_string());
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let typed = Arc::new(TestTransport::default());
    let transport: Arc<dyn Transport> = typed.clone();
    let db_pool = Some(Arc::new(pool.clone()));

    let _ = handle_on_client_ready(
        addr,
        [0u8; 32],
        &connected,
        &None,
        &transport,
        &entity_to_addr,
        &db_pool,
    )
    .await;
    let sent = typed.filter_to(addr);
    cleanup().await;

    let mut expected = CraftingState::new();
    expected.discipline_ids = vec![78];
    expected.expertise.insert(78, 33);
    expected.racial_paradigm_levels = HashMap::from([(1, 5), (2, 2), (3, 1), (4, 1), (5, 1)]);
    expected.blueprint_ids = vec![25];
    expected.applied_science_points = 2;
    let found = (0..sent.len() as u32 + 8).any(|seq| {
        let options = CraftingOptions::default();
        let (packets, _) = build_login_bundle(entity_id, Some(&expected), Some(&options)).finalize(
            FLAG_RELIABLE | FLAG_ON_CHANNEL,
            seq,
            |p| crate::mercury::encrypt_packet(p, &[0u8; 32], EncryptionVersion::V1),
        );
        packets.len() == 1 && sent.contains(&packets[0])
    });
    assert!(
        found,
        "no packet carries the stored crafting state and options; {} packets sent",
        sent.len()
    );
}
