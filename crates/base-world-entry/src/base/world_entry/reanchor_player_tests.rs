//! Tests for [`super`]: the reanchor packet layout, sequencing and sends.

use super::*;
use crate::mercury::SGWGMPLAYER_CLASS_ID;

fn sample_info(entity_id: u32) -> WorldEntryInfo {
    WorldEntryInfo {
        player_entity_id: entity_id,
        space_id: 0x0001_0042,
        pos: [10.0, 20.0, 30.0],
        rot: [0.5, 1.5, 2.5],
        world_name: String::new(),
        class_id: SGWPLAYER_CLASS_ID,
        world_stargates: Vec::new(),
    }
}

/// Pin the wire layout of the CREATE_BASE_PLAYER + enter_world_body burst.
///
/// This is the load-bearing un-ragdoll primitive. If anyone refactors
/// `build_enter_world_body`, changes the `BASEMSG_CREATE_BASE_PLAYER`
/// constant, or "cleans up" the `propertyCount = 0` byte, this test
/// catches it before the pawn-recreate hook silently breaks.
#[test]
fn build_reanchor_burst_body_pins_create_base_player_then_enter_world_body() {
    let entity_id: u32 = 0x1234_5678;
    let info = sample_info(entity_id);

    let body = build_reanchor_burst_body(entity_id, &info);

    // CREATE_BASE_PLAYER header: [0x05][len=6 LE][entity_id LE][class=0x02][propCount=0]
    assert_eq!(
        body[0], BASEMSG_CREATE_BASE_PLAYER,
        "first byte must be CREATE_BASE_PLAYER (0x05)"
    );
    assert_eq!(
        &body[1..3],
        &6u16.to_le_bytes(),
        "length field must be 6 (entity_id 4 + class 1 + propCount 1)"
    );
    assert_eq!(
        &body[3..7],
        &entity_id.to_le_bytes(),
        "entity_id must be little-endian u32"
    );
    assert_eq!(
        body[7], info.class_id,
        "class_id byte must be the info's (login) class"
    );
    assert_eq!(body[8], 0x00, "propertyCount byte must be 0");

    // Tail must be byte-identical to build_enter_world_body so any future
    // refactor of that function (Y/Z swap fixes, viewport id changes,
    // forced-position flags) flows through Reanchor unchanged.
    assert_eq!(
        &body[9..],
        build_enter_world_body(&info, None).as_slice(),
        "tail must equal build_enter_world_body(info, None) verbatim — Reanchor and gate-travel must stay in lockstep on space/viewport/position"
    );
}

/// A GM's reanchor re-creates SGWGmPlayer (0x03), the class it logged in
/// with. Hard-coding SGWPlayer (0x02) demoted a GM on every respawn; the
/// "0x03 shifts method indices" reason for it was disproved
/// (`play_character.rs`).
#[test]
fn build_reanchor_burst_body_writes_the_login_class() {
    let info = WorldEntryInfo {
        class_id: SGWGMPLAYER_CLASS_ID,
        ..sample_info(99)
    };
    let body = build_reanchor_burst_body(99, &info);
    assert_eq!(
        body[7], SGWGMPLAYER_CLASS_ID,
        "the burst must carry the login class, not a hard-coded SGWPlayer"
    );
}

/// Fan-out byte test: a GM session (login class 0x03 cached on the
/// connected state) gets a reanchor burst that re-creates SGWGmPlayer,
/// byte for byte. Fails on the old handler, which always sent 0x02.
#[tokio::test]
async fn reanchor_keeps_the_gm_class_for_a_gm_session() {
    use crate::test_support::{test_default_connected_client_state, TestTransport};

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let entity_id = 0x4322u32;
    let space_id = 0x0001_0042u32;
    let position = [1.0f32, 2.0, 3.0];
    let addr: SocketAddr = "127.0.0.1:40201".parse().unwrap();

    let mut gm = test_default_connected_client_state();
    gm.player_class_id = Some(SGWGMPLAYER_CLASS_ID);
    let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
        Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::from([(addr, gm)])));

    handle_reanchor_player(
        entity_id,
        space_id,
        position,
        [0.0; 3],
        &dyn_transport,
        &connected,
        &entity_to_addr,
    )
    .await
    .expect("reanchor must succeed");

    let sent = transport.drain();
    assert_eq!(sent.len(), 1, "burst only (no cached appearance)");
    let info = WorldEntryInfo {
        player_entity_id: entity_id,
        space_id,
        pos: position,
        rot: [0.0; 3],
        world_name: String::new(),
        class_id: SGWGMPLAYER_CLASS_ID,
        world_stargates: Vec::new(),
    };
    // Built by hand, not through `build_reanchor_burst_body`, so the
    // expectation cannot inherit a class byte the handler got wrong.
    let mut body = vec![BASEMSG_CREATE_BASE_PLAYER];
    body.extend_from_slice(&6u16.to_le_bytes());
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.push(SGWGMPLAYER_CLASS_ID);
    body.push(0x00);
    body.extend_from_slice(&build_enter_world_body(&info, None));
    let plaintext = build_outgoing(REPLY_FLAGS, &body, Some(0), &[], None);
    let expected = encrypt_packet(&plaintext, &[0u8; 32], EncryptionVersion::V1);
    assert_eq!(
        sent[0].1, expected,
        "a GM's reanchor must re-create SGWGmPlayer (0x03), the login class"
    );
}

const TEST_KEY: [u8; 32] = [0x42; 32];

/// Cached appearance + tint → 3 packets (burst + BeingAppearance + onEntityTint).
#[test]
fn build_reanchor_packets_emits_three_when_both_cached() {
    let info = sample_info(0x1234);
    let appearance = vec![0xAA, 0xBB];
    let tint = vec![0xCC, 0xDD];

    let pkts = build_reanchor_packets(
        &TEST_KEY,
        100,
        0x1234,
        &info,
        Some(&appearance),
        Some(&tint),
        &[],
        EncryptionVersion::V1,
    );

    assert_eq!(
        pkts.len(),
        3,
        "with cached appearance + tint, must return 3 packets"
    );
}

/// No cache → 1 packet (burst only). The pawn will render blank, but
/// emitting a partial replay would be worse — body without tint or
/// vice versa flickers visibly.
#[test]
fn build_reanchor_packets_emits_one_when_neither_cached() {
    let info = sample_info(0x1234);
    let pkts = build_reanchor_packets(
        &TEST_KEY,
        100,
        0x1234,
        &info,
        None,
        None,
        &[],
        EncryptionVersion::V1,
    );

    assert_eq!(
        pkts.len(),
        1,
        "without cache, must return only the burst packet"
    );
}

/// Partial cache → 1 packet. Pinning the "all-or-nothing" rule for
/// the replay so a future bug where only one of appearance/tint is
/// populated doesn't silently emit a half-replay.
#[test]
fn build_reanchor_packets_emits_one_when_only_one_side_cached() {
    let info = sample_info(0x1234);
    let appearance = vec![0xAA];
    let tint = vec![0xBB];

    let only_appearance = build_reanchor_packets(
        &TEST_KEY,
        100,
        0x1234,
        &info,
        Some(&appearance),
        None,
        &[],
        EncryptionVersion::V1,
    );
    let only_tint = build_reanchor_packets(
        &TEST_KEY,
        100,
        0x1234,
        &info,
        None,
        Some(&tint),
        &[],
        EncryptionVersion::V1,
    );

    assert_eq!(
        only_appearance.len(),
        1,
        "appearance-only cache must not emit appearance packet alone"
    );
    assert_eq!(
        only_tint.len(),
        1,
        "tint-only cache must not emit tint packet alone"
    );
}

/// Acks attach to the burst packet only. Replay packets must use
/// empty acks — duplicating acks would re-acknowledge already-acked
/// sequence IDs and (depending on the protocol layer) could be
/// rejected as malformed by the client.
#[test]
fn build_reanchor_packets_attaches_acks_to_burst_only() {
    let info = sample_info(0x1234);
    let appearance = vec![0xAA];
    let tint = vec![0xBB];

    let no_acks = build_reanchor_packets(
        &TEST_KEY,
        100,
        0x1234,
        &info,
        Some(&appearance),
        Some(&tint),
        &[],
        EncryptionVersion::V1,
    );
    let with_acks = build_reanchor_packets(
        &TEST_KEY,
        100,
        0x1234,
        &info,
        Some(&appearance),
        Some(&tint),
        &[42, 43],
        EncryptionVersion::V1,
    );

    assert_ne!(
        no_acks[0], with_acks[0],
        "adding acks must change the burst packet"
    );
    assert_eq!(
        no_acks[1], with_acks[1],
        "BeingAppearance packet must not include acks (was identical with vs without acks)"
    );
    assert_eq!(
        no_acks[2], with_acks[2],
        "onEntityTint packet must not include acks (was identical with vs without acks)"
    );
}

/// Sequence IDs are consecutive: base, base+1, base+2. Verified by
/// reconstructing what each replay packet would look like with an
/// explicit seq ID and asserting equality. This catches any future
/// "simplification" that hardcodes the same seq for all packets, or
/// that increments by the wrong stride.
#[test]
fn build_reanchor_packets_uses_consecutive_seqs() {
    use crate::mercury::{build_player_entity_method_packet, method_idx};

    let info = sample_info(0x1234);
    let appearance = vec![0xAA, 0xBB];
    let tint = vec![0xCC, 0xDD];
    let base_seq = 500u32;

    let pkts = build_reanchor_packets(
        &TEST_KEY,
        base_seq,
        0x1234,
        &info,
        Some(&appearance),
        Some(&tint),
        &[],
        EncryptionVersion::V1,
    );

    let expected_appearance = build_player_entity_method_packet(
        &TEST_KEY,
        base_seq + 1,
        &[],
        0x1234,
        method_idx::BEING_APPEARANCE,
        &appearance,
        EncryptionVersion::V1,
    );
    let expected_tint = build_player_entity_method_packet(
        &TEST_KEY,
        base_seq + 2,
        &[],
        0x1234,
        method_idx::ON_ENTITY_TINT,
        &tint,
        EncryptionVersion::V1,
    );

    assert_eq!(
        pkts[1], expected_appearance,
        "packet[1] must be BeingAppearance with seq=base_seq+1"
    );
    assert_eq!(
        pkts[2], expected_tint,
        "packet[2] must be onEntityTint with seq=base_seq+2"
    );
}

/// Domain C (fan-out byte test): `handle_reanchor_player` for a client with
/// no cached appearance/tint emits exactly **one** packet — the reanchor
/// burst — to the owner's own addr, byte-exact, with **zero** witness
/// fan-out (reanchor is owner-only). Catches a regression that fans the
/// owner-only burst out to witnesses, or that emits a half-replay when no
/// cache is present.
#[tokio::test]
async fn reanchor_emits_single_burst_to_owner_only() {
    use crate::test_support::{test_default_connected_client_state, TestTransport};

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();

    let entity_id = 0x4321u32;
    let space_id = 0x0001_0042u32;
    let position = [10.0f32, 20.0, 30.0];
    let rotation = [0.5f32, 1.5, 2.5];
    let addr: SocketAddr = "127.0.0.1:40200".parse().unwrap();

    // Default state has no cached appearance/tint → burst-only (1 packet).
    let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
        Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> = Arc::new(Mutex::new(
        HashMap::from([(addr, test_default_connected_client_state())]),
    ));

    handle_reanchor_player(
        entity_id,
        space_id,
        position,
        rotation,
        &dyn_transport,
        &connected,
        &entity_to_addr,
    )
    .await
    .expect("reanchor must succeed");

    let sent = transport.drain();
    assert_eq!(
        sent.len(),
        1,
        "burst-only (no cache) ⇒ exactly one packet, no witness fan-out"
    );
    assert_eq!(
        sent[0].0, addr,
        "reanchor burst goes to the owner's own addr"
    );

    // base_seq = 0 (default next_seq), no acks, all-zero key, no replay.
    let info = WorldEntryInfo {
        player_entity_id: entity_id,
        space_id,
        pos: position,
        rot: rotation,
        world_name: String::new(),
        class_id: SGWPLAYER_CLASS_ID,
        world_stargates: Vec::new(),
    };
    let expected = build_reanchor_packets(
        &[0u8; 32],
        0,
        entity_id,
        &info,
        None,
        None,
        &[],
        EncryptionVersion::V1,
    );
    assert_eq!(expected.len(), 1, "test premise: burst-only");
    assert_eq!(sent[0].1, expected[0], "reanchor burst wire bytes (seq 0)");
}

/// A cached appearance too long for one datagram (a long equipment list)
/// is cut into fragments: every datagram fits the client's 1472-byte
/// buffer, the packet count is what `reanchor_seq_count` reserved, and the
/// sequence numbers run on without a gap into the tint packet. Before the
/// fix the appearance went out as one packet over the buffer, which the
/// client never receives, wedging its reliable stream.
#[test]
fn oversize_appearance_replay_is_fragmented_within_the_client_buffer() {
    use cimmeria_mercury::consts::PACKET_MAX_SIZE;
    use cimmeria_mercury::encryption::MercuryEncryption;
    use cimmeria_mercury::packet::parse_incoming;

    let entity_id = 0x1234;
    let appearance = vec![0x41; 1500];
    let tint = vec![0xCC; 12];
    let acks: Vec<u32> = (1..=10).collect();
    let pkts = build_reanchor_packets(
        &TEST_KEY,
        100,
        entity_id,
        &sample_info(entity_id),
        Some(&appearance),
        Some(&tint),
        &acks,
        EncryptionVersion::V1,
    );
    assert_eq!(
        pkts.len() as u32,
        reanchor_seq_count(entity_id, Some(&appearance), Some(&tint))
    );
    assert_eq!(pkts.len(), 4, "burst, two appearance fragments, tint");
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let mut appearance_body = Vec::new();
    for (i, wire) in pkts.iter().enumerate() {
        assert!(
            wire.len() <= PACKET_MAX_SIZE,
            "packet {i} is {} bytes, over the client's buffer",
            wire.len()
        );
        let p = parse_incoming(&enc.decrypt(wire).unwrap()).unwrap();
        assert_eq!(p.seq_id, Some(100 + i as u32), "contiguous seqs");
        if (1..=2).contains(&i) {
            appearance_body.extend_from_slice(&p.body);
        }
    }
    assert_eq!(
        appearance_body,
        appearance_replay_body(entity_id, &appearance),
        "the fragments reassemble to the BeingAppearance message"
    );
}
