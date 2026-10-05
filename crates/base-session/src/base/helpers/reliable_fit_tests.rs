//! Size guards for [`super::send_to_witness_reliable`]: no reliable datagram
//! it sends is larger than the client's 1472-byte receive buffer, whatever the
//! body and however many ACKs are pending.
//!
//! The 2026-10-05 lab stall: an NPC's `createOnClient` cascade (BeingAppearance
//! with a long component list, tint, flags, level, faction, state and the two
//! 180-byte stat arrays) went out as one packet with 10 piggybacked ACKs, came
//! to 1504 encrypted bytes, never reached the client, and wedged its reliable
//! stream (`tx_hole_stall seq=2744 wire_len=1504`).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::consts::PACKET_MAX_SIZE;
use cimmeria_mercury::encryption::{EncryptionVersion, MercuryEncryption};
use cimmeria_mercury::packet::{parse_incoming, FLAG_FRAGMENTED};
use cimmeria_mercury::transport::Transport;

use super::{send_to_witness_reliable, ConnectedClientState, WitnessSendOutcome};
use crate::cell::messages::NpcAoIData;
use crate::mercury::{
    build_create_entity_cascade, compose_create_entity_cascade_body, SGWMOB_CLASS_ID,
};
use crate::test_support::TestTransport;

const WITNESS: u32 = 700;
const NPC: u32 = 100_221;

/// Entity template 221, "Petbe (hostile)", the longest humanoid appearance in
/// the seed (`db/resources/Entities/Seed/entity_templates.sql`): 14
/// components, 918 bytes of `BeingAppearance` arguments, event set 570.
fn petbe_hostile() -> NpcAoIData {
    let components = [
        "AR_J_Praxis.AR_JM_PB1_PH100",
        "AR_J_Praxis.AR_JM_PG1_PG100PB100",
        "AR_J_Praxis.AR_JM_PH1_PH100",
        "AR_J_Praxis.AR_JM_PL1_PL101",
        "AR_J_Praxis.AR_JM_PT1_PT100PT101PC100PS100",
        "BS_JaffaMale.BS_JM_Boots_00",
        "BS_JaffaMale.BS_JM_FaceHair_01",
        "BS_JaffaMale.BS_JM_FacePaint_01",
        "BS_JaffaMale.BS_JM_Hair_00",
        "BS_JaffaMale.BS_JM_Hands_00",
        "BS_JaffaMale.BS_JM_Head_08",
        "BS_JaffaMale.BS_JM_Legs_00",
        "BS_JaffaMale.BS_JM_Torso_00",
        "WP-Jaffa.WP_Staff_Plasma_4A",
    ];
    NpcAoIData {
        name_id: Some(7586),
        faction: 10,
        alignment: 0,
        event_set_id: Some(570),
        body_set: Some("BS_JaffaMale.BS_JaffaMale".to_string()),
        components: components.iter().map(|c| c.to_string()).collect(),
        ..NpcAoIData::default()
    }
}

/// A gallery-sized lineup: Petbe wearing a second set of armour pieces, so the
/// cascade body alone is past what one datagram can hold.
fn oversize_lineup() -> NpcAoIData {
    let mut npc = petbe_hostile();
    let extra: Vec<String> = npc.components.iter().map(|c| format!("{c}_Alt")).collect();
    npc.components.extend(extra);
    npc.speaker_id = Some(4711);
    npc
}

struct Harness {
    tt: Arc<TestTransport>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    addr: SocketAddr,
    version: EncryptionVersion,
}

const VERSIONS: [EncryptionVersion; 2] = [EncryptionVersion::V1, EncryptionVersion::V2];

/// One connected witness on wire `version`, with `pending` ACKs waiting to
/// ride a packet.
fn harness(version: EncryptionVersion, pending: std::ops::Range<u32>) -> Harness {
    let tt = Arc::new(TestTransport::default());
    let transport: Arc<dyn Transport> = tt.clone();
    let addr: SocketAddr = "127.0.0.1:55900".parse().unwrap();
    let mut state = crate::test_support::test_default_connected_client_state();
    state.enc_version = version;
    state.enc = MercuryEncryption::from_session_key_versioned(state.key, version);
    state.pending_acks.lock().unwrap().extend(pending);
    Harness {
        tt,
        transport,
        connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        entity_to_addr: Arc::new(Mutex::new(HashMap::from([(WITNESS, addr)]))),
        addr,
        version,
    }
}

async fn send_cascade(h: &Harness, npc: &NpcAoIData) -> WitnessSendOutcome {
    send_to_witness_reliable(
        &h.transport,
        &h.connected,
        &h.entity_to_addr,
        WITNESS,
        |key, version, seq, acks| {
            build_create_entity_cascade(
                key,
                seq,
                acks,
                NPC,
                SGWMOB_CLASS_ID,
                42,
                Some(npc),
                version,
            )
        },
    )
    .await
}

/// Every datagram sent, decrypted with the session key and parsed.
fn sent_packets(h: &Harness) -> Vec<(usize, cimmeria_mercury::packet::ParsedPacket)> {
    let enc = MercuryEncryption::from_session_key_versioned([0u8; 32], h.version);
    h.tt.filter_to(h.addr)
        .into_iter()
        .map(|wire| {
            let plaintext = enc.decrypt(&wire).expect("server packet decrypts");
            (wire.len(), parse_incoming(&plaintext).expect("parses"))
        })
        .collect()
}

fn assert_all_fit(packets: &[(usize, cimmeria_mercury::packet::ParsedPacket)]) {
    for (i, (len, _)) in packets.iter().enumerate() {
        assert!(
            *len <= PACKET_MAX_SIZE,
            "datagram {i} is {len} bytes, over the client's {PACKET_MAX_SIZE}-byte buffer"
        );
    }
}

fn pending_acks(h: &Harness) -> Vec<u32> {
    let clients = h.connected.lock().unwrap();
    let pending = clients[&h.addr].pending_acks.lock().unwrap().clone();
    pending
}

/// The lab stall exactly: template 221's cascade (1427-byte body) with a long
/// ACK backlog. Before the fix it took the full 10-ACK data budget and came
/// to 1504 bytes; now it takes only the ACKs that fit beside this body and
/// leaves the rest pending, in one packet of at most 1472 bytes.
#[tokio::test]
async fn npc_cascade_with_an_ack_backlog_stays_within_the_client_buffer() {
    let npc = petbe_hostile();
    let body = compose_create_entity_cascade_body(NPC, SGWMOB_CLASS_ID, 42, Some(&npc));
    assert_eq!(body.len(), 1427, "template 221's cascade body");

    let h = harness(EncryptionVersion::V1, 100..140);
    let outcome = send_cascade(&h, &npc).await;
    assert!(outcome.is_sent(), "{outcome:?}");

    let packets = sent_packets(&h);
    assert_all_fit(&packets);
    assert_eq!(packets.len(), 1, "a body that fits stays one packet");
    let (_, packet) = &packets[0];
    assert_eq!(&packet.body[..], &body[..], "the cascade arrives intact");
    assert_eq!(packet.seq_id, Some(0));
    // The ACKs that rode the packet are the oldest ones; the rest wait.
    let n = packet.acks.len();
    assert!(n < 10, "fewer than the generic budget: {n}");
    assert_eq!(packet.acks, (100..100 + n as u32).collect::<Vec<_>>());
    assert_eq!(pending_acks(&h), (100 + n as u32..140).collect::<Vec<_>>());
}

/// A cascade whose body alone cannot fit one datagram goes out as a
/// fragmented bundle: contiguous reliable sequence numbers, every datagram at
/// most 1472 bytes, and the fragments reassemble to the exact cascade body.
#[tokio::test]
async fn oversize_cascade_is_fragmented_and_reassembles_under_v1_and_v2() {
    let npc = oversize_lineup();
    let body = compose_create_entity_cascade_body(NPC, SGWMOB_CLASS_ID, 42, Some(&npc));
    assert!(
        body.len() > 1500,
        "the body alone is past one datagram: {}",
        body.len()
    );

    for version in VERSIONS {
        let h = harness(version, 100..140);
        let outcome = send_cascade(&h, &npc).await;
        let WitnessSendOutcome::Sent { seq, .. } = outcome else {
            panic!("expected Sent, got {outcome:?}");
        };
        assert_eq!(seq, 0, "the fragments start at the next reliable seq");

        let packets = sent_packets(&h);
        assert_all_fit(&packets);
        assert!(packets.len() >= 2, "fragmented: {}", packets.len());
        let last = packets.len() as u32 - 1;
        let mut reassembled = Vec::new();
        for (i, (_, p)) in packets.iter().enumerate() {
            assert_ne!(p.flags & FLAG_FRAGMENTED, 0, "fragment {i} is flagged");
            assert_eq!(p.seq_id, Some(i as u32), "contiguous seqs");
            assert_eq!((p.frag_begin, p.frag_end), (Some(0), Some(last)));
            reassembled.extend_from_slice(&p.body);
        }
        assert_eq!(reassembled, body, "the fragments reassemble to the cascade");
        // The next reliable send continues after the fragments: no gap, no reuse.
        let next = h.connected.lock().unwrap()[&h.addr]
            .next_seq
            .load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(next, last + 1);
    }
}

/// A single entity method too big for one datagram (a long feedback line, an
/// appearance broadcast) takes the same path: nothing over the buffer.
#[tokio::test]
async fn oversize_single_method_is_fragmented() {
    let args = vec![0x41u8; 1600];
    let h = harness(EncryptionVersion::V1, 1..30);
    let outcome = send_to_witness_reliable(
        &h.transport,
        &h.connected,
        &h.entity_to_addr,
        WITNESS,
        |key, version, seq, acks| {
            crate::mercury::build_entity_method_packet(
                key,
                seq,
                acks,
                NPC,
                crate::mercury::method_idx::BEING_APPEARANCE,
                cimmeria_mercury::channel_bundle::IDBASE_NPC_DEFAULT,
                &args,
                version,
            )
        },
    )
    .await;
    assert!(outcome.is_sent(), "{outcome:?}");
    let packets = sent_packets(&h);
    assert_all_fit(&packets);
    assert_eq!(packets.len(), 2);
    let total: usize = packets.iter().map(|(_, p)| p.body.len()).sum();
    assert_eq!(total, 3 + 4 + args.len(), "header, entity id and args");
}

/// Under wire v2 (17-byte prefix, so 17 bytes less room) template 221's
/// cascade is 1473 bytes with no ACKs at all: the body alone is past the
/// buffer, so it goes out as fragments that reassemble to the cascade.
#[tokio::test]
async fn npc_cascade_under_v2_is_fragmented_even_without_acks() {
    let npc = petbe_hostile();
    let body = compose_create_entity_cascade_body(NPC, SGWMOB_CLASS_ID, 42, Some(&npc));
    let no_acks =
        cimmeria_mercury::packet::encrypted_len(1 + body.len() + 4, EncryptionVersion::V2);
    assert_eq!(no_acks, 1473, "template 221 under v2, no ACKs");

    for pending in [0..0, 100..140] {
        let h = harness(EncryptionVersion::V2, pending.clone());
        let outcome = send_cascade(&h, &npc).await;
        let WitnessSendOutcome::Sent {
            seq,
            bytes,
            packets,
            ..
        } = outcome
        else {
            panic!("expected Sent, got {outcome:?}");
        };
        let sent = sent_packets(&h);
        assert_all_fit(&sent);
        assert_eq!(seq, 0);
        assert_eq!(packets, 2, "{pending:?}");
        assert_eq!(sent.len(), 2, "{pending:?}");
        assert_eq!(
            bytes,
            h.tt.filter_to(h.addr).iter().map(Vec::len).sum::<usize>()
        );
        let reassembled: Vec<u8> = sent.iter().flat_map(|(_, p)| p.body.to_vec()).collect();
        assert_eq!(reassembled, body);
        // ACKs ride the first fragment only, within the generic budget.
        assert!(sent[0].1.acks.len() <= 2, "v2 data budget is 2");
        assert!(sent[1].1.acks.is_empty());
    }
}

/// The measuring build is the only one that logs. An NPC with no appearance
/// data makes the cascade builder WARN `aoi.cascade_appearance_missing`;
/// before the real build ran silent, every such send logged it twice.
#[tokio::test]
async fn builder_warnings_fire_once_per_send() {
    let capture = crate::test_support::LogCapture::install();
    let npc = NpcAoIData::default();
    let h = harness(EncryptionVersion::V1, 1..5);
    assert!(send_cascade(&h, &npc).await.is_sent());
    let warns = capture
        .all()
        .into_iter()
        .filter(|e| e.level == tracing::Level::WARN && e.message_contains("no appearance data"))
        .count();
    assert_eq!(warns, 1, "one appearance-missing WARN per send");
}

/// A builder whose body grows with its ACK list (so the measured size
/// understates the real one): the real build is checked, and when it is past
/// the buffer the packet is rebuilt at the same seq without ACKs, which the
/// measurement proved fits. The ACKs go back to the front of the queue.
#[tokio::test]
async fn real_build_past_the_buffer_is_rebuilt_without_acks() {
    let capture = crate::test_support::LogCapture::install();
    let h = harness(EncryptionVersion::V1, 100..140);
    let outcome = send_to_witness_reliable(
        &h.transport,
        &h.connected,
        &h.entity_to_addr,
        WITNESS,
        |key, version, seq, acks| {
            let args = vec![0x41u8; 1380 + 40 * acks.len()];
            crate::mercury::build_entity_method_packet(
                key,
                seq,
                acks,
                NPC,
                crate::mercury::method_idx::BEING_APPEARANCE,
                cimmeria_mercury::channel_bundle::IDBASE_NPC_DEFAULT,
                &args,
                version,
            )
        },
    )
    .await;
    assert!(outcome.is_sent(), "{outcome:?}");
    let sent = sent_packets(&h);
    assert_all_fit(&sent);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].1.seq_id, Some(0));
    assert!(sent[0].1.acks.is_empty(), "rebuilt without ACKs");
    assert_eq!(
        pending_acks(&h),
        (100..140).collect::<Vec<_>>(),
        "ACKs requeued in order"
    );
    assert!(capture
        .find_message(tracing::Level::WARN, "rebuilt without them")
        .is_some_and(|e| e.has_field("event", "ack_overflow_rebuilt")));
}

/// A builder that frames bigger after it was measured, even without ACKs:
/// the reserved seq is handed back (nobody took the next one) and the body
/// goes out as fragments from that same seq, so the stream has no gap.
#[tokio::test]
async fn packet_growing_after_reserve_is_fragmented_on_the_same_seq() {
    let h = harness(EncryptionVersion::V1, 0..0);
    let calls = std::cell::Cell::new(0usize);
    let outcome = send_to_witness_reliable(
        &h.transport,
        &h.connected,
        &h.entity_to_addr,
        WITNESS,
        |key, version, seq, acks| {
            calls.set(calls.get() + 1);
            let len = if calls.get() == 1 { 100 } else { 2000 };
            crate::mercury::build_entity_method_packet(
                key,
                seq,
                acks,
                NPC,
                crate::mercury::method_idx::BEING_APPEARANCE,
                cimmeria_mercury::channel_bundle::IDBASE_NPC_DEFAULT,
                &vec![0x42u8; len],
                version,
            )
        },
    )
    .await;
    let WitnessSendOutcome::Sent { seq, packets, .. } = outcome else {
        panic!("expected Sent, got {outcome:?}");
    };
    let sent = sent_packets(&h);
    assert_all_fit(&sent);
    assert_eq!((seq, packets, sent.len()), (0, 2, 2));
    for (i, (_, p)) in sent.iter().enumerate() {
        assert_eq!(p.seq_id, Some(i as u32));
    }
    let next = h.connected.lock().unwrap()[&h.addr]
        .next_seq
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(next, 2, "no seq burned");
}
