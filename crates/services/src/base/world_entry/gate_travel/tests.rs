//! Tests for the gate-travel cell↔base handoff seam.
//!
//! What's pinned:
//!   * cell-side `handle_dial_gate` emits a CellToBaseMsg::GateTravel
//!     with the right target world / position fields — fed verbatim
//!     into `handle_gate_travel`, the destination state lands in
//!     ConnectedClientState's pending_world_entry.
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
use super::*;
use crate::base::PendingClientReadyInfo;
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

fn make_state() -> ConnectedClientState {
    ConnectedClientState {
        enc: MercuryEncryption::from_session_key([0xCDu8; 32]),
        key: [0xCDu8; 32],
        account_id: 0xAABB,
        access_level: 0,
        char_list_sent: true,
        world_entry_sent: true, // post-playCharacter
        pending_player_entity_id: Some(42),
        player_entity_id: Some(42),
        next_seq: Arc::new(AtomicU32::new(10)),
        next_seq_unreliable: Arc::new(AtomicU32::new(0)),
        pending_acks: Arc::new(Mutex::new(Vec::new())),
        last_recv: Arc::new(Mutex::new(Instant::now())),
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
        player_name: Some("Tester".to_string()),
        player_level: Some(5),
        player_archetype: Some(1),
        world_name: Some("Agnos".to_string()),
        player_xp: Some(0),
        player_training_points: Some(0),
        active_player_id: Some(7),
        pending_destination_ring_id: None,
        channel: Mutex::new(cimmeria_mercury::channel::Channel::new(
            "127.0.0.1:9999".parse().unwrap(),
        )),
    }
}

async fn make_socket() -> Arc<UdpSocket> {
    Arc::new(UdpSocket::bind("127.0.0.1:0").await.expect("bind UDP"))
}

/// Cross-service round-trip: cell-side `handle_dial_gate` emits a
/// `CellToBaseMsg::GateTravel` with the captured destination fields,
/// and `handle_gate_travel` propagates them verbatim into
/// ConnectedClientState's pending_world_entry. Pins the entire
/// cell→base→cell handoff at the message-shape level — a regression
/// in either side of the pair would surface here.
#[tokio::test]
async fn dial_gate_to_handle_gate_travel_round_trips_destination_state() {
    use crate::cell::gate_travel::handle_dial_gate;
    use crate::cell::messages::CellToBaseMsg;
    use crate::cell::space_manager::SpaceManager;
    use crate::cell::spawner::StargateEntry;

    // ── Cell side: drive handle_dial_gate ───────────────────────
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" />
        <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
    </Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Agnos" />
        <Space WorldName="Castle" />
    </Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();

    const ENTITY_ID: u32 = 42;
    const TARGET_GATE: i32 = 2;
    const TARGET_X: f32 = 761.677;
    const TARGET_Y: f32 = 63.466;
    const TARGET_Z: f32 = 551.716;
    const TARGET_YAW: f32 = 2.152;

    mgr.stargates.insert(
        TARGET_GATE,
        StargateEntry {
            world_name: "Castle".to_string(),
            x: TARGET_X,
            y: TARGET_Y,
            z: TARGET_Z,
            yaw: TARGET_YAW,
        },
    );
    mgr.create_entity(ENTITY_ID, "Agnos", [10.0; 3], [0.0; 3])
        .unwrap();
    mgr.connect_entity(ENTITY_ID);

    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(16);
    handle_dial_gate(ENTITY_ID, TARGET_GATE, 0, &tx, &mut mgr).await;

    // Cell entity destroyed (post-handoff cleanup).
    assert!(
        mgr.get_entity(ENTITY_ID).is_none(),
        "cell entity must be destroyed before the GateTravel message lands at base"
    );

    // Capture the GateTravel message — this is the cell→base contract.
    let captured = match rx.try_recv().expect("GateTravel emitted") {
        CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name,
            position,
            rotation,
            destination_ring_id,
        } => {
            // Stargate dial-travel must NOT carry a ring id — that field is
            // reserved for `Effect::TeleportCrossWorld`.
            assert_eq!(
                destination_ring_id, None,
                "stargate dial-gate must leave destination_ring_id=None",
            );
            (entity_id, target_world_name, position, rotation)
        }
        other => panic!("expected GateTravel, got {other:?}"),
    };
    assert_eq!(captured.0, ENTITY_ID, "entity_id round-trips");
    assert_eq!(captured.1, "Castle", "target world from stargate cache");
    assert!((captured.2[0] - TARGET_X).abs() < 0.01);
    assert!((captured.2[1] - TARGET_Y).abs() < 0.01);
    assert!((captured.2[2] - TARGET_Z).abs() < 0.01);
    assert_eq!(
        captured.3,
        [0.0, 0.0, TARGET_YAW],
        "rotation = [0, 0, yaw] from stargate"
    );

    // ── Base side: feed the captured fields into handle_gate_travel ─
    let socket = make_socket().await;
    let addr: SocketAddr = "127.0.0.1:55700".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::new()));
    connected.lock().unwrap().insert(addr, make_state());
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(ENTITY_ID, addr);
        m
    }));
    // No cell_tx — handle_gate_travel falls back to resolve_space_id_fallback.
    // No db_pool — the persist UPDATE branch is skipped.
    handle_gate_travel(
        captured.0,
        &captured.1,
        captured.2,
        captured.3,
        None, // stargate dial-travel has no cross-world ring carry-through
        &socket,
        &connected,
        &entity_to_addr,
        &None,
        &None,
    )
    .await
    .expect("base-side gate travel completes");

    // Pending world entry now reflects the destination — the next
    // ENABLE_ENTITIES from the client will drive the create-player
    // wire flow against this entry.
    let map = connected.lock().unwrap();
    let c = map.get(&addr).unwrap();
    let entry = c
        .pending_world_entry
        .as_ref()
        .expect("pending_world_entry must be populated post-gate-travel");
    assert_eq!(entry.player_entity_id, ENTITY_ID);
    assert_eq!(entry.world_name, "Castle");
    assert_eq!(entry.pos, captured.2);
    assert_eq!(entry.rot, captured.3);
    assert_eq!(c.pending_player_entity_id, Some(ENTITY_ID));
    // pending_client_ready cleared so the next ENABLE_ENTITIES
    // doesn't observe a stale ready-state from the old world.
    assert!(c.pending_client_ready.is_none());
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

    let socket = make_socket().await;
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
        &socket,
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
    // pending_world_entry MUST NOT be set (we aborted before
    // reaching the populate-state block).
    let map = connected.lock().unwrap();
    assert!(
        map.get(&addr).unwrap().pending_world_entry.is_none(),
        "fail-closed abort must NOT populate pending_world_entry — \
         a populated entry here means the wrong-character corruption \
         window is still open"
    );
}

/// Stronger pin (live-DB): when `active_player_id` is missing, the
/// persist branch must NOT issue an UPDATE — both characters on a
/// multi-character account keep their existing world_location and
/// pos_x/pos_y/pos_z columns. A regression that fell back to
/// "lowest player_id for the account" would write the destination
/// onto the wrong character's row, surfacing here as a column drift.
#[tokio::test]
async fn gate_travel_persist_branch_is_a_no_op_when_active_player_id_missing() {
    use crate::test_support::require_db_or_skip;

    let pool = require_db_or_skip!();
    // Sentinel slot 0x7000_0D00 — distinct from prior slots:
    //   0x100 grant_cash, 0x200 move_inventory, 0x300 grant_item,
    //   0x400 missions, 0x500 mail, 0x700 vendor/paid_repair,
    //   0x900 vendor/buyback, 0xB00 inventory/ammo, 0xC00 vendor/data.
    // Picking 0xD00 avoids collision with paid_repair which also uses 0x700.
    const TEST_ACCOUNT: i32 = 0x7000_0D00;
    const CHAR_A: i32 = 0x7000_0D01;
    const CHAR_B: i32 = 0x7000_0D02;

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
    let socket = make_socket().await;
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
        &socket,
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
    let socket = make_socket().await;
    let connected = Arc::new(Mutex::new(HashMap::new()));
    // entity_to_addr is empty — the entity is genuinely unknown.
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));

    let result = handle_gate_travel(
        999,
        "Castle",
        [0.0; 3],
        [0.0; 3],
        None,
        &socket,
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
    let socket = make_socket().await;
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
        &socket,
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
