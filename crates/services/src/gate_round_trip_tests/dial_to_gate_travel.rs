//! Gate travel's cell->base handoff, end to end: the cell's
//! `handle_dial_gate` emits a `CellToBaseMsg::GateTravel`, and the base's
//! `handle_gate_travel` turns it into the destination's pending world entry.
//!
//! Was `base::world_entry::gate_travel::tests::
//! dial_gate_to_handle_gate_travel_round_trips_destination_state`, which moved
//! to `cimmeria-base-world-entry` (wave B3 of
//! docs/architecture/services-crate-split.md) without this test: it drives the
//! cell's dial handler (`cimmeria-cell-interactions` since wave C4), which
//! that crate cannot reach. `make_state`, `make_socket` and
//! `stub_pending_ready` are copies of that file's fixtures.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use crate::base::world_entry::handle_gate_travel;
use crate::base::{ConnectedClientState, PendingClientReadyInfo};
use crate::test_support::TestTransport;

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

async fn make_socket() -> Arc<dyn Transport> {
    Arc::new(TestTransport::new())
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
            address_origin: 18,
            // Unpinned: this fixture asserts the traveller lands on the gate
            // row, which is what an unpinned gate must keep doing.
            arrival: None,
            event_set_id: None,
        },
    );
    mgr.create_entity(ENTITY_ID, "Agnos", [10.0; 3], [0.0; 3])
        .unwrap();
    mgr.connect_entity(ENTITY_ID);
    // `handle_dial_gate` refuses an address the player does not hold
    // (CAT-O-01), so the round-trip fixture has to grant one — this test is
    // about the cell→base message shape, not the address book.
    mgr.get_entity_mut(ENTITY_ID)
        .expect("traveller")
        .known_stargates = vec![TARGET_GATE];

    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(16);
    // No `REGION_FLAG_Stargate` region is registered for Agnos, so the
    // dial takes the CA10 fallback and travels in one call — which is
    // what this round-trip wants to exercise. The arm-then-cross path is
    // covered in `cell::gate_travel::tests`.
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    handle_dial_gate(ENTITY_ID, TARGET_GATE, 0, &tx, &mut mgr, &engine).await;

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
            destination_space_id,
        } => {
            // Stargate dial-travel must NOT carry a ring id — that field is
            // reserved for `Effect::TeleportCrossWorld`.
            assert_eq!(
                destination_ring_id, None,
                "stargate dial-gate must leave destination_ring_id=None",
            );
            // Stargate travel resolves by world name: no exact-instance
            // targeting (that is GM `.goto <player>` only, packet P45).
            assert_eq!(
                destination_space_id, None,
                "stargate dial-gate must leave destination_space_id=None",
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
    let transport = make_socket().await;
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
        None, // ... and no exact-instance targeting
        &transport,
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
