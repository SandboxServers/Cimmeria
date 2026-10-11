//! Lab chaos flush modes (CH-07): fan-out byte tests on the deferred-AoI
//! flush's real wire output, with a registry lent to the test's thread.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_base_session::base::lab_chaos::ChaosRefused;
use cimmeria_base_session::base::lab_chaos::FlushChaos;
use cimmeria_base_session::base::lab_chaos::LabChaos;
use cimmeria_base_session::base::lab_chaos::SHIFT_LEN_MAX;
use cimmeria_mercury::client_model::{unpack_packet_bodies, unpack_reliable_stream};
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{parse_incoming, FLAG_FRAGMENTED};
use cimmeria_mercury::transport::Transport;

use super::flush_chaos::{apply, filler, lend_registry, shift_for_cursor, sweep_len, CursorFamily};
use super::intro_bundles::Phase1Shape;
use crate::base::deferred_aoi::DeferredAoiMsg;
use crate::mercury::compose_create_entity_base_body;
use crate::test_support::{test_default_connected_client_state, TestTransport};

const WITNESS: u32 = 200;
const RECORD: usize = 37;
const CREATE_ENTITY: u8 = 0x09;

fn addr() -> SocketAddr {
    "127.0.0.1:54701".parse().unwrap()
}

fn entity_id(i: u32) -> u32 {
    1000 + i
}

fn position(i: u32) -> [f32; 3] {
    [i as f32 * 10.0, 0.0, 0.0]
}

/// `n` NPC introductions, as the 28-NPC burst test buffers them.
fn burst(n: u32) -> Vec<DeferredAoiMsg> {
    (0..n)
        .map(|i| DeferredAoiMsg::EnteredAoI {
            entity_id: entity_id(i),
            class_id: 1,
            position: position(i),
            direction: [0.0; 3],
            level: 1,
            npc_data: None,
            player_data: None,
        })
        .collect()
}

/// Phase-1 records `from..to`, concatenated as the flush composes them.
fn records(from: u32, to: u32) -> Vec<u8> {
    (from..to)
        .flat_map(|i| compose_create_entity_base_body(entity_id(i), 1, position(i), [0.0; 3]))
        .collect()
}

/// A registry lent for the test's life, with `mode` set for `entity`.
fn chaos(entity: u32, mode: FlushChaos) -> &'static LabChaos {
    let registry: &'static LabChaos = Box::leak(Box::new(LabChaos::new()));
    registry
        .set(
            addr(),
            entity,
            "flush test".into(),
            None,
            Some(mode),
            60,
            Instant::now(),
        )
        .expect("a valid set");
    registry
}

/// Flush `buffered` on a fresh session with `registry` lent, and return the
/// decrypted wire packets in send order.
async fn flush(buffered: Vec<DeferredAoiMsg>, registry: Option<&'static LabChaos>) -> Vec<Vec<u8>> {
    lend_registry(registry);
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let mut state = test_default_connected_client_state();
    state.deferred_aoi_msgs = buffered;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr(), state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(WITNESS, addr())])));
    super::deferred_flush::flush_deferred_aoi(
        WITNESS,
        addr(),
        "on_client_ready",
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    lend_registry(None);
    let session = MercuryEncryption::from_session_key([0u8; 32]);
    typed
        .filter_to(addr())
        .iter()
        .map(|raw| session.decrypt(raw).unwrap())
        .collect()
}

fn body(packet: &[u8]) -> Vec<u8> {
    parse_incoming(packet).unwrap().body.to_vec()
}

fn fragmented(packet: &[u8]) -> bool {
    parse_incoming(packet).unwrap().flags & FLAG_FRAGMENTED != 0
}

/// Entity ids of the phase-1 creates the client dispatches, in order.
fn create_order(packets: &[Vec<u8>]) -> Vec<u32> {
    unpack_reliable_stream(packets)
        .iter()
        .flat_map(|b| b.messages.clone())
        .filter(|(id, _)| *id == CREATE_ENTITY)
        .map(|(_, payload)| u32::from_le_bytes(payload[..4].try_into().unwrap()))
        .collect()
}

/// Unset chaos changes no byte: no registry, an empty one, one with chaos for
/// another address, and one with chaos left by the last character on this
/// address (cleared by the flush) all send the same bytes, and the phase-1
/// packet is exactly the records with nothing ahead. Fails if `apply` shapes
/// a flush it should leave alone.
#[tokio::test]
async fn flush_without_chaos_is_byte_identical() {
    let baseline = flush(burst(28), None).await;
    assert_eq!(body(&baseline[0]), records(0, 28), "phase 1: records only");

    let empty: &'static LabChaos = Box::leak(Box::new(LabChaos::new()));
    assert_eq!(flush(burst(28), Some(empty)).await, baseline, "empty");

    let other_addr: &'static LabChaos = Box::leak(Box::new(LabChaos::new()));
    other_addr
        .set(
            "127.0.0.1:1".parse().unwrap(),
            WITNESS,
            "flush:pack".into(),
            None,
            Some(FlushChaos::Shift { len: 5 }),
            60,
            Instant::now(),
        )
        .unwrap();
    assert_eq!(flush(burst(28), Some(other_addr)).await, baseline, "other");

    let last_character = chaos(WITNESS + 1, FlushChaos::Shift { len: 5 });
    assert_eq!(
        flush(burst(28), Some(last_character)).await,
        baseline,
        "chaos set for the previous character on this address"
    );
    assert_eq!(
        last_character.flush_for(addr(), Instant::now()),
        None,
        "the stale entry is cleared, so it cannot reach a later flush"
    );
}

/// Pack: 24 records go in one unfragmented packet, 888 body bytes (the last
/// record ends at client cursor 889). With 40 introductions, which the
/// ordinary flush fragments, the first 34 still go in one packet. Fails if
/// Pack is ignored.
#[tokio::test]
async fn pack_puts_records_in_one_packet() {
    let packed = flush(
        burst(24),
        Some(chaos(WITNESS, FlushChaos::Pack { records: 24 })),
    )
    .await;
    assert!(!fragmented(&packed[0]));
    assert_eq!(body(&packed[0]), records(0, 24));
    assert_eq!(body(&packed[0]).len(), 24 * RECORD);

    let ordinary = flush(burst(40), None).await;
    assert!(fragmented(&ordinary[0]), "40 records overflow one packet");

    let packed = flush(
        burst(40),
        Some(chaos(WITNESS, FlushChaos::Pack { records: 34 })),
    )
    .await;
    assert!(!fragmented(&packed[0]));
    assert_eq!(body(&packed[0]), records(0, 34));
    assert!(!fragmented(&packed[1]));
    assert_eq!(body(&packed[1]), records(34, 40), "the rest, still phase 1");
    assert_eq!(
        create_order(&packed),
        (0..40).map(entity_id).collect::<Vec<_>>()
    );
}

/// Shift by 5: one 5-byte `voiceData` ahead of the records, so record `j`'s
/// create sits at body offset `5 + 37j`; the client consumes the filler and
/// every record. Fails if the filler is missing, misplaced or malformed.
#[tokio::test]
async fn shift_moves_every_cursor_by_len() {
    let shifted = flush(
        burst(28),
        Some(chaos(WITNESS, FlushChaos::Shift { len: 5 })),
    )
    .await;
    let first = body(&shifted[0]);
    assert!(!fragmented(&shifted[0]));
    assert_eq!(&first[..5], &[0x33, 2, 0, 0, 0]);
    assert_eq!(&first[5..], records(0, 28).as_slice());
    for j in 0..28 {
        assert_eq!(first[5 + RECORD * j], CREATE_ENTITY, "record {j}");
    }
    let unpacked = unpack_packet_bodies(&[&first]);
    assert_eq!(unpacked.abort, None);
    assert_eq!(unpacked.messages[0], (0x33, vec![0, 0]));
    assert_eq!(unpacked.messages.len(), 1 + 2 * 28);
}

/// The class-1 residue: a create at cursor 224 needs `L = 1`, so it uses
/// `L = 38` (two fillers) on record 5, and the flush's bytes put record 5's
/// CREATE_ENTITY exactly there. Fails if the helper or the 38-byte filler
/// drifts.
#[tokio::test]
async fn shift_38_lands_class_1_residue() {
    assert_eq!(
        shift_for_cursor(224, CursorFamily::Create, 24),
        Some((38, 5))
    );
    let shifted = flush(
        burst(24),
        Some(chaos(WITNESS, FlushChaos::Shift { len: 38 })),
    )
    .await;
    let first = body(&shifted[0]);
    let at = 224 - 1;
    assert_eq!(at, 38 + RECORD * 5);
    assert_eq!(first[at], CREATE_ENTITY);
    assert_eq!(&first[at + 3..at + 7], &entity_id(5).to_le_bytes());
    let unpacked = unpack_packet_bodies(&[&first]);
    assert_eq!(unpacked.abort, None);
    assert_eq!(unpacked.messages[0], (0x02, vec![10]));
    assert_eq!(unpacked.messages[1].0, 0x33);
}

#[test]
fn shift_for_cursor_matches_the_design_cases() {
    // The observed #1341 cursor: an avatar update, no filler, record 20.
    assert_eq!(
        shift_for_cursor(752, CursorFamily::Avatar, 24),
        Some((0, 20))
    );
    assert_eq!(shift_for_cursor(8, CursorFamily::Create, 24), Some((7, 0)));
    assert_eq!(
        shift_for_cursor(8, CursorFamily::Avatar, 24),
        None,
        "R < 12"
    );
    assert_eq!(
        shift_for_cursor(224, CursorFamily::Create, 5),
        None,
        "no record 5"
    );
    assert_eq!(
        shift_for_cursor(1500, CursorFamily::Create, 255),
        None,
        "past one packet"
    );
    for r in (8..1270).step_by(8) {
        for family in [CursorFamily::Create, CursorFamily::Avatar] {
            let Some((len, j)) = shift_for_cursor(r, family, 34) else {
                continue;
            };
            assert_ne!(len, 1, "R {r}");
            assert!(len <= SHIFT_LEN_MAX, "R {r}");
            let start = if family == CursorFamily::Create {
                1
            } else {
                12
            };
            assert_eq!(u32::from(len) + start + 37 * j, r, "R {r} {family:?}");
        }
    }
}

/// No filler is 1 byte: the registry refuses `Shift { len: 1 }`, `filler`
/// has none, and the sweep never draws 1 but draws every other length.
#[test]
fn shift_len_1_is_refused() {
    let registry = LabChaos::new();
    let refused = registry.set(
        addr(),
        WITNESS,
        "shift 1".into(),
        None,
        Some(FlushChaos::Shift { len: 1 }),
        60,
        Instant::now(),
    );
    assert!(
        matches!(refused, Err(ChaosRefused::BadKnob(_))),
        "{refused:?}"
    );
    assert_eq!(filler(1), None);
    assert_eq!(filler(SHIFT_LEN_MAX + 1), None);
    let mut seen = [false; SHIFT_LEN_MAX as usize + 1];
    for seed in [0u64, 7, 0xDEAD_BEEF] {
        for draw in 0..4000 {
            seen[usize::from(sweep_len(seed, draw))] = true;
        }
    }
    assert!(!seen[1], "the sweep drew 1");
    let missing: Vec<usize> = (0..seen.len()).filter(|&l| l != 1 && !seen[l]).collect();
    assert!(missing.is_empty(), "never drawn: {missing:?}");
}

/// Fillers by length: 2 is `02 0A`, 36 one 36-byte `voiceData`, 37 and 38
/// two messages. Every allowed length is exactly that long and the client
/// model consumes it whole.
#[test]
fn shift_filler_is_two_messages_above_36() {
    assert_eq!(filler(0), Some(Vec::new()));
    assert_eq!(filler(2), Some(vec![vec![0x02, 0x0A]]));
    let mut voice_36 = vec![0x33, 33, 0];
    voice_36.resize(36, 0);
    assert_eq!(filler(36), Some(vec![voice_36]));
    let mut voice_35 = vec![0x33, 32, 0];
    voice_35.resize(35, 0);
    assert_eq!(filler(37), Some(vec![vec![0x02, 0x0A], voice_35]));
    assert_eq!(filler(38).map(|f| f.len()), Some(2));
    for len in (0..=SHIFT_LEN_MAX).filter(|&l| l != 1) {
        let bytes: Vec<u8> = filler(len).unwrap().concat();
        assert_eq!(bytes.len(), usize::from(len), "len {len}");
        if !bytes.is_empty() {
            assert_eq!(unpack_packet_bodies(&[&bytes]).abort, None, "len {len}");
        }
    }
}

/// The same seed gives the same order twice, another seed another order,
/// and either way every record arrives whole. Fails if the shuffle is
/// unseeded or a no-op.
#[tokio::test]
async fn reorder_is_seeded() {
    let seven = flush(
        burst(12),
        Some(chaos(WITNESS, FlushChaos::Reorder { seed: 7 })),
    )
    .await;
    let again = flush(
        burst(12),
        Some(chaos(WITNESS, FlushChaos::Reorder { seed: 7 })),
    )
    .await;
    let eight = flush(
        burst(12),
        Some(chaos(WITNESS, FlushChaos::Reorder { seed: 8 })),
    )
    .await;
    assert_eq!(seven, again);
    let (a, b) = (create_order(&seven), create_order(&eight));
    assert_ne!(a, b);
    assert_ne!(
        a,
        (0..12).map(entity_id).collect::<Vec<_>>(),
        "not shuffled"
    );
    let mut sorted = a.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..12).map(entity_id).collect::<Vec<_>>());
    for bundle in unpack_reliable_stream(&seven) {
        assert_eq!(bundle.abort, None);
    }
}

#[test]
fn delay_returns_the_configured_wait() {
    lend_registry(Some(chaos(WITNESS, FlushChaos::Delay { ms: 1500 })));
    let mut segments = vec![burst(3)];
    let got = apply(addr(), WITNESS, None, &mut segments);
    let mut no_intros = vec![vec![DeferredAoiMsg::LeftAoI { entity_id: 7 }]];
    let quiet = apply(addr(), WITNESS, None, &mut no_intros);
    lend_registry(None);
    assert_eq!(
        got,
        (Some(Duration::from_millis(1500)), Phase1Shape::default())
    );
    assert_eq!(quiet, (None, Phase1Shape::default()), "no introductions");
}

/// The flush itself waits: on a paused clock the packets go out only once
/// the delay has passed.
#[tokio::test(start_paused = true)]
async fn delay_holds_the_flush_back() {
    let start = tokio::time::Instant::now();
    let sent = flush(
        burst(3),
        Some(chaos(WITNESS, FlushChaos::Delay { ms: 1500 })),
    )
    .await;
    assert!(!sent.is_empty());
    assert!(start.elapsed() >= Duration::from_millis(1500));
}
