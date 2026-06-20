//! Witness-broadcast arm fan-out byte tests.
//!
//! `WitnessEntityMethod` and `EntityInvisible` fan a single packet out
//! to exactly one address — the witness's. The bug shapes these tests
//! pin:
//! 1. **Wrong-recipient routing** — a regression that swapped
//!    `witness_id` for `entity_id` (the observee) would send the
//!    packet to the wrong client. With both addresses present in
//!    `entity_to_addr`, a swap would show up as a
//!    `send_count_to(observee_addr) > 0` failure.
//! 2. **Wire-payload drift** — the captured packet's bytes must match
//!    `build_entity_method_packet` / `build_entity_invisible` with
//!    the same inputs. A regression to a wrong builder, wrong
//!    `idbase`, swapped `entity_id` for `witness_id` in the body, or
//!    an off-by-one in args framing would no longer match.

use cimmeria_mercury::channel_bundle::IDBASE_NPC_DEFAULT;

use super::super::*;
use super::test_default_connected_client_state;
use crate::mercury::{build_entity_invisible, build_entity_method_packet};
use crate::test_support::TestTransport;

/// `WitnessEntityMethod` routes the method packet to exactly the
/// witness's address — never to the observee entity's address, never
/// to any other session. Witness-cardinality regression guard.
#[tokio::test]
async fn witness_entity_method_routes_one_packet_to_witness_addr_only() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 900u32;
    let observee_id = 901u32; // the ghost entity the method is called on
    let witness_addr: SocketAddr = "127.0.0.1:55900".parse().unwrap();
    let observee_addr: SocketAddr = "127.0.0.1:55901".parse().unwrap();

    let connected = Arc::new(Mutex::new(HashMap::from([
        (witness_addr, test_default_connected_client_state()),
        (observee_addr, test_default_connected_client_state()),
    ])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (witness_id, witness_addr),
        (observee_id, observee_addr),
    ])));

    handle_cell_message(
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id: observee_id,
            method_index: 0x20,
            args: vec![0xDE, 0xAD],
            entity_is_player: false,
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

    assert_eq!(
        typed_transport.send_count_to(witness_addr),
        1,
        "exactly one packet to the witness"
    );
    assert_eq!(
        typed_transport.send_count_to(observee_addr),
        0,
        "observee must NOT receive — only the witness does"
    );
    assert_eq!(typed_transport.len(), 1, "no traffic to any other address");

    // Byte-exact payload check: reconstruct the expected wire bytes
    // from the same `build_entity_method_packet` the handler uses.
    // `test_default_connected_client_state` initialises the session at
    // next_seq=0 with the all-zero key and no pending acks — those are
    // the seq/acks the witness session passes to the builder. A
    // regression that swaps `entity_id` for `witness_id` in the body,
    // picks a wrong idbase, or pads args incorrectly would diverge.
    let sent = typed_transport.filter_to(witness_addr);
    assert_eq!(sent.len(), 1, "exactly one packet recorded for filter_to");
    let expected = build_entity_method_packet(
        &[0u8; 32],
        0,
        &[],
        observee_id,
        0x20,
        IDBASE_NPC_DEFAULT,
        &[0xDE, 0xAD],
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    assert_eq!(
        sent[0], expected,
        "witness packet bytes must match build_entity_method_packet \
         with the observee entity_id, the supplied method_index, \
         IDBASE_NPC_DEFAULT, and the supplied args"
    );
}

/// `EntityInvisible` routes the visibility-hide packet to exactly the
/// witness's address. Same fan-out shape as `WitnessEntityMethod`,
/// different wire bytes — pin the routing so a regression that swaps
/// the witness_id for entity_id (a wrong-recipient bug class) trips.
#[tokio::test]
async fn entity_invisible_routes_one_packet_to_witness_addr_only() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 910u32;
    let observee_id = 911u32;
    let witness_addr: SocketAddr = "127.0.0.1:55910".parse().unwrap();
    let observee_addr: SocketAddr = "127.0.0.1:55911".parse().unwrap();

    let connected = Arc::new(Mutex::new(HashMap::from([
        (witness_addr, test_default_connected_client_state()),
        (observee_addr, test_default_connected_client_state()),
    ])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (witness_id, witness_addr),
        (observee_id, observee_addr),
    ])));

    handle_cell_message(
        CellToBaseMsg::EntityInvisible {
            witness_id,
            entity_id: observee_id,
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

    assert_eq!(typed_transport.send_count_to(witness_addr), 1);
    assert_eq!(typed_transport.send_count_to(observee_addr), 0);
    assert_eq!(typed_transport.len(), 1);

    // Byte-exact payload check, same shape as the WitnessEntityMethod
    // test above. Catches a refactor that called the wrong builder
    // (e.g. `build_entity_leave` instead of `build_entity_invisible`,
    // which has different body bytes despite both being 1-entity AoI
    // events) or that swapped `entity_id`/`witness_id` in the body.
    let sent = typed_transport.filter_to(witness_addr);
    assert_eq!(sent.len(), 1, "exactly one packet recorded for filter_to");
    let expected = build_entity_invisible(
        &[0u8; 32],
        0,
        &[],
        observee_id,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    assert_eq!(
        sent[0], expected,
        "witness packet bytes must match build_entity_invisible \
         with the observee entity_id"
    );
}

/// idbase selection regression guard: `WitnessEntityMethod` for a player
/// ghost entity must encode with `IDBASE_SGW_PLAYER` (61), not
/// `IDBASE_NPC_DEFAULT` (62). Method indices ≥61 encode differently under
/// each idbase; wrong selection corrupts the wire byte.
///
/// Also verifies the NPC path is unchanged — NPC observees still use
/// `IDBASE_NPC_DEFAULT` (no regression).
#[tokio::test]
async fn witness_entity_method_player_ghost_uses_idbase_61_npc_uses_62() {
    use crate::mercury::build_entity_method_packet;
    use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;

    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();

    // High-index method to make the encoding difference visible.
    // Method 61 is the first index that encodes differently between
    // IDBASE_SGW_PLAYER (61 → extended) and IDBASE_NPC_DEFAULT (61 → direct).
    let method_index: u16 = 61;
    let args: Vec<u8> = vec![0xAB];

    let witness_id = 1000u32;
    let player_entity_id = 2000u32;
    let npc_entity_id = 2001u32;
    let witness_addr: SocketAddr = "127.0.0.1:56000".parse().unwrap();
    let player_addr: SocketAddr = "127.0.0.1:56001".parse().unwrap(); // player ghost has addr too

    let connected = Arc::new(Mutex::new(HashMap::from([
        (witness_addr, test_default_connected_client_state()),
        (player_addr, test_default_connected_client_state()),
    ])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (witness_id, witness_addr),
        (player_entity_id, player_addr),
    ])));

    // ── Player ghost (entity_is_player = true) ──
    handle_cell_message(
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id: player_entity_id,
            method_index,
            args: args.clone(),
            entity_is_player: true,
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

    let sent = typed_transport.drain();
    assert_eq!(sent.len(), 1);
    let expected_player = build_entity_method_packet(
        &[0u8; 32],
        0,
        &[],
        player_entity_id,
        method_index,
        IDBASE_SGW_PLAYER,
        &args,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    assert_eq!(
        sent[0].1, expected_player,
        "player ghost method 61 must encode with IDBASE_SGW_PLAYER (61), \
         not IDBASE_NPC_DEFAULT (62)"
    );

    // ── NPC ghost (entity_is_player = false) — no regression ──
    // Reset the witness session to next_seq=0 so the second send's wire bytes
    // are comparable against a fresh `build_entity_method_packet(seq=0)`. The
    // first send above advanced the witness session's sequence number; without
    // this reset the NPC packet would carry seq 1 and the byte-exact compare
    // would fail for sequence reasons unrelated to the idbase under test.
    {
        let mut guard = connected.lock().unwrap();
        guard.insert(witness_addr, test_default_connected_client_state());
    }

    handle_cell_message(
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id: npc_entity_id,
            method_index,
            args: args.clone(),
            entity_is_player: false,
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

    let sent_npc = typed_transport.drain();
    assert_eq!(sent_npc.len(), 1);
    let expected_npc = build_entity_method_packet(
        &[0u8; 32],
        0,
        &[],
        npc_entity_id,
        method_index,
        IDBASE_NPC_DEFAULT,
        &args,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    assert_eq!(
        sent_npc[0].1, expected_npc,
        "NPC ghost method 61 must still encode with IDBASE_NPC_DEFAULT (62) — no regression"
    );
}
