//! NA34 regression guard: two players in a shared world must see each
//! other regardless of arrival order.
//!
//! `docs/architecture/player-ghost-aoi-cascade.md`'s "Known gaps" section
//! flags that arrival order B — A gates in while B is already standing
//! there, so the deferred-AoI path drives at least one of the two
//! introductions — was never exercised end to end: every existing test in
//! this crate drives ONE witness through EITHER the standalone path
//! (`aoi_defer_gate`) or the deferred/flush path (`deferred_flush`'s own
//! tests), never both directions of the SAME pair of sessions at once. This
//! test drives both `entity_to_addr` entries and both `ConnectedClientState`
//! sessions together, mirroring what two real clients in Castle look like:
//!
//! - **A** is already fully loaded (`pending_client_ready: None`) — the
//!   witness set the periodic AoI tick or the connect-time immediate compute
//!   would already have driven to `aoi::entered_aoi`'s standalone path.
//! - **B** is mid-load (`pending_client_ready: Some(..)`) — its `EnteredAoI`
//!   for A is still buffered until `onClientReady` flushes it.
//!
//! Both directions must land the OBSERVEE's player-ghost cascade (name,
//! appearance, level, live combat state) on the OBSERVER's wire, not the
//! bare NPC-shaped cascade — that's the byte-level assertion a
//! `#578`/#737-style regression would trip.

use super::super::*;
use crate::cell::messages::PlayerAoIData;
use crate::mercury::{
    compose_create_entity_cascade_body, compose_player_ghost_cascade_body, PlayerGhostCascade,
};
use crate::test_support::{test_default_connected_client_state, TestTransport};
use cimmeria_mercury::encryption::MercuryEncryption;

const PLAYER_A: u32 = 900;
const PLAYER_B: u32 = 901;

fn addr_a() -> SocketAddr {
    "127.0.0.1:55910".parse().unwrap()
}

fn addr_b() -> SocketAddr {
    "127.0.0.1:55911".parse().unwrap()
}

/// An in-world session: named, levelled, with a cached appearance — the
/// state `play_character` + `map_loaded` leave behind for a real player.
fn named_session(name: &str, level: i32, archetype: i32) -> ConnectedClientState {
    let mut s = test_default_connected_client_state();
    s.player_name = Some(name.to_string());
    s.player_level = Some(level);
    s.player_archetype = Some(archetype);
    s.player_alignment = Some(1);
    s.cached_appearance_args = Some(vec![0x11, 0x22]);
    s.cached_tint_args = Some(vec![0; 12]);
    s
}

fn live_state(target_id: i32) -> PlayerAoIData {
    PlayerAoIData {
        state_field: 0,
        target_id,
        ammo_type_id: 0,
        ..PlayerAoIData::default()
    }
}

/// Decrypt a test-transport packet (all-zero session key) down to its
/// message body: strip the flags byte and the 4-byte sequence footer.
fn body_of(pkt: &[u8]) -> Vec<u8> {
    let pt = MercuryEncryption::from_session_key([0u8; 32])
        .decrypt(pkt)
        .unwrap();
    pt[1..pt.len() - 4].to_vec()
}

/// Two sessions sharing one `connected` / `entity_to_addr` map, exactly as
/// the real `BaseService` holds every connected client: A ready, B still
/// mid-load.
fn two_player_world() -> (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let a = named_session("Lomiada", 9, 4);
    let mut b = named_session("Harset", 5, 2);
    b.pending_client_ready = Some(crate::base::PendingClientReadyInfo {
        entity_id: PLAYER_B,
        player_id: 2,
        world_name: "Castle".into(),
        appearance_args: Vec::new(),
        tint_args: Vec::new(),
        first_login: 0,
    });

    let connected = Arc::new(Mutex::new(HashMap::from([(addr_a(), a), (addr_b(), b)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (PLAYER_A, addr_a()),
        (PLAYER_B, addr_b()),
    ])));
    (connected, entity_to_addr)
}

/// Arrival order B (the documented worst case): A is already standing in
/// the space and ready; B is mid-load. The cell fires BOTH introductions —
/// "B entered A's AoI" and "A entered B's AoI" — in the same tick, exactly
/// as `SpaceManager::compute_aoi_changes` does for every player in
/// `space.players` on one pass. A's introduction of B must reach A's wire
/// immediately (standalone path); B's introduction of A must reach B's wire
/// only after `onClientReady` flushes the deferred buffer. Both bodies must
/// carry the OBSERVEE's real identity, not the bare cascade.
#[tokio::test]
async fn both_arrival_directions_deliver_the_observee_identity() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let (connected, entity_to_addr) = two_player_world();

    // 1. The cell tells A's witness set that B entered A's AoI. A is
    //    already ready, so this must land on A's wire right now.
    handle_cell_message(
        CellToBaseMsg::EnteredAoI {
            witness_id: PLAYER_A,
            entity_id: PLAYER_B,
            space_id: 0x0001_0042,
            class_id: 0x02,
            position: [10.0, 0.0, 10.0],
            direction: [0.0; 3],
            level: 1, // cell-side level is never authoritative for players
            npc_data: None,
            player_data: Some(live_state(0)),
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    // 2. The SAME tick tells B's witness set that A entered B's AoI. B is
    //    still mid-load, so this must buffer — nothing on B's wire yet.
    handle_cell_message(
        CellToBaseMsg::EnteredAoI {
            witness_id: PLAYER_B,
            entity_id: PLAYER_A,
            space_id: 0x0001_0042,
            class_id: 0x02,
            position: [11.0, 0.0, 11.0],
            direction: [0.0; 3],
            level: 1,
            npc_data: None,
            player_data: Some(live_state(0)),
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    // A already has B's cascade: 2 packets (create_base + player-ghost
    // cascade), both addressed to A, carrying B's ("Harset") identity.
    let sent_to_a = typed_transport.filter_to(addr_a());
    assert_eq!(
        sent_to_a.len(),
        2,
        "A must receive B's create + cascade immediately (standalone path)"
    );
    let expected_b_ghost = compose_player_ghost_cascade_body(
        PLAYER_B,
        &PlayerGhostCascade {
            name: "Harset",
            level: 5,
            archetype: 2,
            alignment: 1,
            appearance_args: Some(&[0x11, 0x22]),
            tint_args: Some(&[0; 12]),
            live: &live_state(0),
        },
    );
    assert_eq!(
        body_of(&sent_to_a[1]),
        expected_b_ghost,
        "A's cascade for B must carry B's real identity, not the bare cascade"
    );
    assert_ne!(
        expected_b_ghost,
        compose_create_entity_cascade_body(PLAYER_B, 0x02, 1, None),
        "test invariant: the ghost cascade differs from the bare one"
    );

    // B has nothing yet — its introduction of A is still buffered.
    assert!(
        typed_transport.filter_to(addr_b()).is_empty(),
        "B is pre-onClientReady: A's introduction must be buffered, not sent"
    );
    {
        let clients = connected.lock().unwrap();
        let buf = &clients.get(&addr_b()).unwrap().deferred_aoi_msgs;
        assert_eq!(buf.len(), 1, "B's buffer must hold exactly A's EnteredAoI");
    }

    // 3. B finishes loading and signals onClientReady: flush the buffer.
    flush_deferred_aoi(
        PLAYER_B,
        addr_b(),
        "on_client_ready",
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let sent_to_b = typed_transport.filter_to(addr_b());
    assert_eq!(
        sent_to_b.len(),
        2,
        "flushing B's buffer must deliver A's create + cascade (phase-1 + phase-2 bundles)"
    );
    let expected_a_ghost = compose_player_ghost_cascade_body(
        PLAYER_A,
        &PlayerGhostCascade {
            name: "Lomiada",
            level: 9,
            archetype: 4,
            alignment: 1,
            appearance_args: Some(&[0x11, 0x22]),
            tint_args: Some(&[0; 12]),
            live: &live_state(0),
        },
    );
    assert_eq!(
        body_of(&sent_to_b[1]),
        expected_a_ghost,
        "B's cascade for A must carry A's real identity, not the bare cascade, \
         even though it travelled through the deferred-AoI buffer"
    );

    // Neither client received anything addressed to the other.
    assert_eq!(
        typed_transport.len(),
        4,
        "exactly 4 packets total: 2 to A, 2 to B — no cross-talk"
    );
}
