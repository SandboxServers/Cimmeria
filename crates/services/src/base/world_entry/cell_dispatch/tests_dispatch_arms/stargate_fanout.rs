//! Stargate gate-event fan-out byte test (CA10).
//!
//! The cell emits one `WitnessEntityMethod` per observer for
//! `Stargate_MakeGate` / `Stargate_CrossGate`; this pins what those
//! messages become on the wire once the base routes them. Sibling of
//! [`super::witness_broadcast`], but end-to-end from the cell emitter
//! rather than from a hand-built message: the cell-side fan-out
//! (`cell::gate_travel::sequences::send_gate_sequence`) produces the
//! messages, the base dispatcher routes them, and the assertions cover
//! both halves at once.
//!
//! Bug shapes pinned:
//! 1. **Amplification / omission** — exactly one packet per observer
//!    address and nothing to anyone else.
//! 2. **Wire drift** — each packet is byte-identical to
//!    `build_entity_method_packet` with `onSequence` (method 1), the
//!    dialer as the observee, `IDBASE_SGW_PLAYER`, and the 26-byte
//!    `onSequence` payload carrying the DB-resolved sequence id.

use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;

use super::super::*;
use super::test_default_connected_client_state;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::StargateEntry;
use crate::mercury::{build_entity_method_packet, method_idx::ON_SEQUENCE};
use crate::test_support::TestTransport;

/// Castle gate event set and the `Stargate_MakeGate` sequence it
/// resolves to (`event_sets_sequences` 10011 → `sequences` 10145/6100).
const EVENT_SET: i32 = 10011;
const EVENT_MAKE_GATE: i32 = 6100;
const SEQ_MAKE_GATE: i32 = 10145;

const DIALER: u32 = 700;
const WITNESS_A: u32 = 701;
const WITNESS_B: u32 = 702;

/// Three co-located players in one space, both non-dialers witnessing
/// the dialer, plus the stargate + sequence caches the emitter reads.
fn space_with_three_players() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();

    for eid in [DIALER, WITNESS_A, WITNESS_B] {
        mgr.create_entity(eid, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        let e = mgr.get_entity_mut(eid).unwrap();
        e.is_player = true;
        e.player_id = Some(eid as i32);
        mgr.connect_entity(eid);
    }
    for observer in [WITNESS_A, WITNESS_B] {
        mgr.get_entity_mut(observer)
            .unwrap()
            .witnesses
            .insert(cimmeria_common::EntityId(DIALER as i32));
    }

    mgr.stargates.insert(
        1,
        StargateEntry {
            world_name: "Agnos".to_string(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            event_set_id: Some(EVENT_SET),
        },
    );
    mgr.sequence_map
        .insert((EVENT_SET, EVENT_MAKE_GATE), SEQ_MAKE_GATE);
    mgr
}

#[tokio::test]
async fn make_gate_fans_out_one_packet_to_each_witness_and_the_dialer() {
    // ── Cell side: produce the real messages ──────────────────────
    let mgr = space_with_three_players();
    let (cell_tx, mut cell_rx) = tokio::sync::mpsc::channel(32);
    crate::cell::gate_travel::sequences::send_gate_sequence(
        DIALER,
        Some(EVENT_SET),
        EVENT_MAKE_GATE,
        &cell_tx,
        &mgr,
    )
    .await;
    drop(cell_tx);

    let mut emitted = Vec::new();
    while let Ok(msg) = cell_rx.try_recv() {
        emitted.push(msg);
    }
    assert_eq!(
        emitted.len(),
        3,
        "the cell must emit exactly one frame per observer (dialer + 2 \
         witnesses); got {}",
        emitted.len()
    );

    // ── Base side: route each one ─────────────────────────────────
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();

    let dialer_addr: SocketAddr = "127.0.0.1:57700".parse().unwrap();
    let a_addr: SocketAddr = "127.0.0.1:57701".parse().unwrap();
    let b_addr: SocketAddr = "127.0.0.1:57702".parse().unwrap();
    let uninvolved_addr: SocketAddr = "127.0.0.1:57799".parse().unwrap();

    let connected = Arc::new(Mutex::new(HashMap::from([
        (dialer_addr, test_default_connected_client_state()),
        (a_addr, test_default_connected_client_state()),
        (b_addr, test_default_connected_client_state()),
        (uninvolved_addr, test_default_connected_client_state()),
    ])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (DIALER, dialer_addr),
        (WITNESS_A, a_addr),
        (WITNESS_B, b_addr),
        (999u32, uninvolved_addr),
    ])));

    for msg in emitted {
        handle_cell_message(
            msg,
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
    }

    for (label, addr) in [
        ("dialer", dialer_addr),
        ("witness A", a_addr),
        ("witness B", b_addr),
    ] {
        assert_eq!(
            typed_transport.send_count_to(addr),
            1,
            "{label} must receive the gate sequence exactly once"
        );
    }
    assert_eq!(
        typed_transport.send_count_to(uninvolved_addr),
        0,
        "a player who is not witnessing the dialer must receive nothing"
    );
    assert_eq!(typed_transport.len(), 3, "no traffic to any other address");

    // Byte-exact: every session starts at next_seq=0 with the all-zero
    // key, so all three packets are the identical frame.
    let mut args = Vec::with_capacity(26);
    args.extend_from_slice(&SEQ_MAKE_GATE.to_le_bytes());
    args.extend_from_slice(&(DIALER as i32).to_le_bytes());
    args.extend_from_slice(&(DIALER as i32).to_le_bytes());
    args.push(1);
    args.extend_from_slice(&0.0f32.to_le_bytes());
    args.extend_from_slice(&0u32.to_le_bytes());
    args.push(3); // KISMET_VIEW_EventInvoker
    args.extend_from_slice(&0i32.to_le_bytes());

    let expected = build_entity_method_packet(
        &[0u8; 32],
        0,
        &[],
        DIALER,
        ON_SEQUENCE,
        IDBASE_SGW_PLAYER,
        &args,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    for (label, addr) in [
        ("dialer", dialer_addr),
        ("witness A", a_addr),
        ("witness B", b_addr),
    ] {
        let sent = typed_transport.filter_to(addr);
        assert_eq!(sent.len(), 1);
        assert_eq!(
            sent[0], expected,
            "{label}'s packet must be onSequence(10145) on the dialer's \
             ghost under IDBASE_SGW_PLAYER"
        );
    }
}
