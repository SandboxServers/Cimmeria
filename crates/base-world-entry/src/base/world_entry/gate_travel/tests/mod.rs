//! Tests for the gate-travel cell↔base handoff seam.
//!
//! What's pinned:
//!   * The cell→base half of the seam, cell-side `handle_dial_gate`'s
//!     `CellToBaseMsg::GateTravel` fed verbatim into `handle_gate_travel`,
//!     drives the cell's dial handler, so it is
//!     `gate_round_trip_tests::dial_to_gate_travel` in `cimmeria-services`.
//!   * Active-player-id fail-closed guard: missing `active_player_id`
//!     refuses to persist the destination (otherwise a fallback would
//!     corrupt the wrong character on multi-character accounts).
//!   * Missing entity_to_addr / connected entries surface as Err
//!     (not silently dropped).
//!
//! Out of scope: multi-shard handoff (the codebase has no live-shard
//! registry today — `orchestrator_shards.rs` is a read-only loader).
//! These tests cover the cell→base→cell handoff within a single shard,
//! which is the production behavior.
//!
//! [`transfer`] holds the GM cross-instance transfer half of the same seam:
//! exact-instance targeting end to end, the validate-before-`CreateEntity`
//! ordering, and mid-transfer disconnect recovery. It reuses [`make_state`]
//! and [`make_socket`] from here.
use super::*;

mod crafting_options;
mod crafting_queue;
mod space_fallback;
mod transfer;
mod world_name;
use crate::base::PendingClientReadyInfo;
use crate::test_support::TestTransport;
use cimmeria_mercury::encryption::MercuryEncryption;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::time::Instant;

/// Stub PendingClientReadyInfo for fixture seeding. Used to make the
/// `pending_client_ready.is_none()` post-condition a real regression
/// guard: if the fixture starts with `Some(...)` and the assertion
/// later requires `None`, then a regression that stops clearing the
/// field surfaces as a failed assertion.
fn stub_pending_ready() -> PendingClientReadyInfo {
    PendingClientReadyInfo {
        entity_id: 0,
        player_id: 0,
        world_name: "Stale".to_string(),
        appearance_args: vec![0xAB],
        tint_args: vec![0xCD],
        first_login: 0,
    }
}

pub(super) fn make_state() -> ConnectedClientState {
    ConnectedClientState {
        enc: MercuryEncryption::from_session_key([0xCDu8; 32]),
        key: [0xCDu8; 32],
        enc_version: cimmeria_mercury::encryption::EncryptionVersion::V1,
        account_id: 0xAABB,
        account_name: Some("testacct".into()),
        access_level: 0,
        dnd_message: None,
        afk_message: None,
        ignore: Default::default(),
        char_list_sent: true,
        world_entry_sent: true, // post-playCharacter
        pending_player_entity_id: Some(42),
        player_entity_id: Some(42),
        next_seq: Arc::new(AtomicU32::new(10)),
        next_seq_unreliable: Arc::new(AtomicU32::new(0)),
        pending_acks: Arc::new(Mutex::new(Vec::new())),
        last_recv: Arc::new(Mutex::new(Instant::now())),
        connected_at: Instant::now(),
        account_entity_id: 1,
        next_data_id: 0,
        pending_world_entry: None,
        pending_player_load_data: None,
        pending_map_loaded: None,
        // Seeded with Some(...) so a regression that stops clearing
        // it surfaces as a failed assertion in the round-trip test.
        // Without seeding, asserting None would be a no-op.
        pending_client_ready: Some(stub_pending_ready()),
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
        player_name: Some("Tester".to_string()),
        player_level: Some(5),
        player_archetype: Some(1),
        player_alignment: None,
        world_name: Some("Agnos".to_string()),
        player_xp: Some(0),
        player_training_points: Some(0),
        active_player_id: Some(7),
        pending_destination_ring_id: None,
        channel: Mutex::new(cimmeria_mercury::channel::Channel::new(
            "127.0.0.1:9999".parse().unwrap(),
        )),
        crafting_options: Default::default(),
    }
}

pub(super) async fn make_socket() -> Arc<dyn Transport> {
    Arc::new(TestTransport::new())
}

/// Active-player-id fail-closed guard (unit-level). Without
/// `active_player_id` cached on ConnectedClientState (set during
/// playCharacter), gate travel must REFUSE to persist — otherwise a
/// fallback like "lowest player_id for the account" would silently
/// corrupt a DIFFERENT character's row on multi-character accounts.
/// The handler logs and returns Ok (no UDP packet sent).
///
/// This test drives the persist branch by passing a non-None
/// db_pool. Without the cached id, the persist guard returns early
/// BEFORE the UPDATE runs — so we don't actually need a working DB
/// (the function exits before issuing the query). The
/// `gate_travel_persist_branch_is_a_no_op_when_active_player_id_missing`
/// live-DB sibling test below pins the stronger property that the
/// real DB rows are unchanged.
///
/// The abort also ends the session: by the time this handler runs the cell
/// has already removed the entity from its origin space, so leaving the
/// client connected would strand it bound to an entity that is in no space at
/// all. See `transfer::aborted_transfer_ends_the_session_rather_than_stranding_an_unspaced_client`.
#[tokio::test]
async fn gate_travel_without_active_player_id_aborts_before_persist() {
    // Build a non-connectable PgPool. The test hits the
    // active_player_id guard first and never reaches the SQL, so
    // we never need to connect. connect_lazy returns Ok regardless
    // of whether the URL is reachable; expect-on-build catches a
    // genuine misconfiguration (URL syntax error) loudly.
    let lazy_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_millis(1))
        .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
        .expect("connect_lazy must succeed for any well-formed URL");

    let transport = make_socket().await;
    let addr: SocketAddr = "127.0.0.1:55701".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let mut state = make_state();
    // The guard: clear active_player_id. Without this cached, the
    // persist branch must abort before issuing the UPDATE.
    state.active_player_id = None;
    connected.lock().unwrap().insert(addr, state);
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(42u32, addr);
        m
    }));

    let result = handle_gate_travel(
        42,
        "Castle",
        [10.0, 20.0, 30.0],
        [0.0; 3],
        None,
        None,
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &Some(Arc::new(lazy_pool)),
    )
    .await;
    // The guard returns Ok — the failure was logged, not propagated.
    assert!(
        result.is_ok(),
        "gate travel returns Ok on the fail-closed abort path"
    );
    // The session is gone, so `pending_world_entry` cannot have been
    // populated — a surviving session here would mean either that the
    // populate-state block ran (the wrong-character corruption window is
    // still open) or that the abort left a client stranded on an entity that
    // is in no space.
    assert!(
        connected.lock().unwrap().get(&addr).is_none(),
        "fail-closed abort must end the session: the cell already tore the \
         entity out of its origin space, so a surviving session is a client \
         bound to an entity that exists nowhere"
    );
}

/// Stronger pin (live-DB): when `active_player_id` is missing, the
/// persist branch must NOT issue an UPDATE — both characters on a
/// multi-character account keep their existing world_location and
/// pos_x/pos_y/pos_z columns. A regression that fell back to
/// "lowest player_id for the account" would write the destination
/// onto the wrong character's row, surfacing here as a column drift.
#[tokio::test]
async fn live_db_gate_travel_persist_branch_is_a_no_op_when_active_player_id_missing() {
    use crate::test_support::require_db_or_skip;

    let pool = require_db_or_skip!();
    // Sentinel slot 0x7000_8500. It was 0x7000_0D00, which
    // `player_load/meta.rs` owns (#800); the workspace sentinel lint in
    // `cimmeria-test-support` now keeps every sentinel value in one file.
    const TEST_ACCOUNT: i32 = 0x7000_8500;
    const CHAR_A: i32 = 0x7000_8501;
    const CHAR_B: i32 = 0x7000_8502;

    // Cleanup before + after to keep concurrent tests honest.
    async fn cleanup(p: &PgPool) {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE account_id = $1")
            .bind(TEST_ACCOUNT)
            .execute(p)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(TEST_ACCOUNT)
            .execute(p)
            .await;
    }
    cleanup(&pool).await;

    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(TEST_ACCOUNT)
        .bind(format!("gate-test-{TEST_ACCOUNT}"))
        .execute(&pool)
        .await
        .expect("insert account");

    // Two characters with distinct, recognisable world + position so a
    // wrong-character UPDATE would produce a clearly different value.
    for (pid, world, x) in [(CHAR_A, "Agnos", 100.0f32), (CHAR_B, "CombatSim", 200.0f32)] {
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', $4, 'BS_HumanMale.BS_HumanMale', \
                       $5, 0.0, 0.0, 0, 0)",
        )
        .bind(TEST_ACCOUNT)
        .bind(pid)
        .bind(format!("gate-test-{pid}"))
        .bind(world)
        .bind(x)
        .execute(&pool)
        .await
        .expect("insert player");
    }

    // Wire the gate-travel call: no active_player_id, real DB pool
    // pointing at the seeded rows.
    let transport = make_socket().await;
    let addr: SocketAddr = "127.0.0.1:55720".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let mut state = make_state();
    // Match the seeded account_id so a fallback "lowest player_id
    // for the account" would actually touch our rows if it ran.
    state.account_id = TEST_ACCOUNT as u32;
    state.active_player_id = None;
    connected.lock().unwrap().insert(addr, state);
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(42u32, addr);
        m
    }));

    let _ = handle_gate_travel(
        42,
        "Castle", // destination world that doesn't match either seeded row
        [999.0, 999.0, 999.0],
        [0.0; 3],
        None,
        None,
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &Some(Arc::new(pool.clone())),
    )
    .await;

    // Verify both rows kept their seeded values. Any column drift
    // here means the persist branch ran and targeted the wrong
    // character.
    for (pid, expected_world, expected_x) in
        [(CHAR_A, "Agnos", 100.0f32), (CHAR_B, "CombatSim", 200.0f32)]
    {
        let (world, pos_x): (String, f32) = sqlx::query_as(
            "SELECT world_location, pos_x FROM sgw_player \
             WHERE player_id = $1 AND account_id = $2",
        )
        .bind(pid)
        .bind(TEST_ACCOUNT)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            world, expected_world,
            "char {pid}'s world_location must be unchanged when fail-closed guard fires"
        );
        assert!(
            (pos_x - expected_x).abs() < 0.01,
            "char {pid}'s pos_x must be unchanged when fail-closed guard fires (got {pos_x}, expected {expected_x})"
        );
    }

    cleanup(&pool).await;
}

/// Missing entity_to_addr entry: the cell told us about an entity_id
/// we don't know. handle_gate_travel must surface this as an Err so
/// the caller can log and skip — silently no-op'ing would mean the
/// cell tore down the entity but the client never gets RESET_ENTITIES,
/// stranding the player on the OLD world's avatar.
#[tokio::test]
async fn gate_travel_with_unknown_entity_id_returns_err() {
    let transport = make_socket().await;
    let connected = Arc::new(Mutex::new(HashMap::new()));
    // entity_to_addr is empty — the entity is genuinely unknown.
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));

    let result = handle_gate_travel(
        999,
        "Castle",
        [0.0; 3],
        [0.0; 3],
        None,
        None,
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
    )
    .await;
    assert!(
        result.is_err(),
        "unknown entity_id must surface as Err so the caller logs it"
    );
}

/// Missing connected entry: addr exists in entity_to_addr but the
/// connection state was torn down between the cell-side gate dial
/// and the base-side processing. Must Err — same rationale as the
/// unknown-entity case.
#[tokio::test]
async fn gate_travel_with_torn_down_connected_state_returns_err() {
    let transport = make_socket().await;
    let addr: SocketAddr = "127.0.0.1:55702".parse().unwrap();
    // entity_to_addr has the entity, but `connected` does not.
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(42u32, addr);
        m
    }));

    let result = handle_gate_travel(
        42,
        "Castle",
        [0.0; 3],
        [0.0; 3],
        None,
        None,
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
    )
    .await;
    assert!(
        result.is_err(),
        "torn-down connection state must surface as Err"
    );
}

/// SS-00: the last-resort abandon removes the session outright, so the
/// character no longer resolves in the online name index and nobody else's
/// listing is touched. Regression shape: an abandon that only cancels the
/// session (and leaves it in `connected`) keeps a ghost name tells and duel
/// challenges would resolve to.
#[tokio::test]
async fn abandoned_unspaced_session_leaves_no_player_index_listing() {
    use cimmeria_base_session::base::player_index::{lookup_online, NameLookup, OnlinePlayer};

    let addr: SocketAddr = "127.0.0.1:55690".parse().unwrap();
    let other: SocketAddr = "127.0.0.1:55691".parse().unwrap();
    let mut leaving = make_state();
    leaving.player_name = Some("Lomiada".to_string());
    leaving.active_player_id = Some(7);
    leaving.player_entity_id = Some(42);
    leaving.listed_online = true;
    let mut staying = make_state();
    staying.player_name = Some("Teal".to_string());
    staying.active_player_id = Some(8);
    staying.player_entity_id = Some(43);
    staying.listed_online = true;
    let connected = Arc::new(Mutex::new(HashMap::from([
        (addr, leaving),
        (other, staying),
    ])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(42u32, addr), (43u32, other)])));
    assert_eq!(
        lookup_online(&connected, "Lomiada"),
        NameLookup::Found(OnlinePlayer { addr, player_id: 7 })
    );

    let capture = crate::test_support::LogCapture::install();
    abandon_unspaced_session(addr, 42, &connected, &entity_to_addr, &None).await;

    assert_eq!(lookup_online(&connected, "Lomiada"), NameLookup::NotFound);
    assert!(
        capture.all().iter().any(|c| c.target == "online_index"
            && c.has_field("event", "online_index.remove")
            && c.has_field("path", "gate_travel_abandon")),
        "the abandon logs online_index.remove path=gate_travel_abandon"
    );
    assert_eq!(
        lookup_online(&connected, "Teal"),
        NameLookup::Found(OnlinePlayer {
            addr: other,
            player_id: 8
        })
    );
}
