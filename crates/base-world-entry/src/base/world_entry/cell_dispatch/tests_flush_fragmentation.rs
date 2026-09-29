//! The deferred-AoI flush must reach the client whole (#838).
//!
//! The phase-2 cascade of a 24-NPC world entry is one ~18 KB reliable bundle
//! in 15 fragments. The client's bundle loop demands every message header lie
//! inside one packet and abandons the rest of the bundle at the first that does
//! not, so NPCs after that point never got appearance and never rendered.
//! These tests run the flush's real wire output through the client model.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::client_model::{unpack_packet_bodies, unpack_reliable_stream, ClientUnpack};
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{parse_incoming, FLAG_FRAGMENTED, FRAGMENT_BODY_SIZE};
use cimmeria_mercury::transport::Transport;

use crate::base::deferred_aoi::DeferredAoiMsg;
use crate::cell::messages::NpcAoIData;
use crate::mercury::{compose_create_entity_base_body, compose_create_entity_cascade_body};
use crate::test_support::{test_default_connected_client_state, TestTransport};

const WITNESS: u32 = 200;
const NPC_COUNT: u32 = 24;
const CLASS_ID: u8 = 1;

/// A humanoid NPC whose component names are `pad` bytes longer, which shifts
/// where every later fragment boundary falls.
fn npc(i: u32, pad: usize) -> NpcAoIData {
    NpcAoIData {
        name_id: Some(1000 + i as i32),
        faction: 10,
        body_set: Some("BS_HumanMale.BS_HumanMale".into()),
        components: vec![
            format!("BS_HumanMale.BS_HM_Hands_{}", "0".repeat(2 + pad)),
            format!(
                "BS_HumanMale.BS_HM_Torso_{}",
                "0".repeat(2 + (pad + i as usize) % 7)
            ),
            "BS_HumanMale.BS_HM_Legs_00".into(),
        ],
        ..NpcAoIData::default()
    }
}

fn entity_id(i: u32) -> u32 {
    1000 + i
}

/// Concatenated phase-2 body for the fixture, exactly as the flush composes it.
fn cascade_body(pad: usize) -> Vec<u8> {
    let mut body = Vec::new();
    for i in 0..NPC_COUNT {
        body.extend(compose_create_entity_cascade_body(
            entity_id(i),
            CLASS_ID,
            1,
            Some(&npc(i, pad)),
        ));
    }
    body
}

/// The first `pad` whose phase-2 body a raw 1300-byte split would break: the
/// client model aborts on it, having dispatched only a prefix.
fn pad_where_raw_split_breaks_the_client() -> usize {
    (0..64)
        .find(|&pad| {
            let body = cascade_body(pad);
            let raw: Vec<&[u8]> = body.chunks(FRAGMENT_BODY_SIZE).collect();
            unpack_packet_bodies(&raw).abort.is_some()
        })
        .expect("some fixture variant must put a header at a raw cut")
}

/// Flush 24 buffered introductions and return the decrypted wire packets in
/// send order.
async fn flush(pad: usize) -> Vec<Vec<u8>> {
    let addr: SocketAddr = "127.0.0.1:54601".parse().unwrap();
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let mut state = test_default_connected_client_state();
    for i in 0..NPC_COUNT {
        state.deferred_aoi_msgs.push(DeferredAoiMsg::EnteredAoI {
            entity_id: entity_id(i),
            class_id: CLASS_ID,
            position: [i as f32 * 3.0, 0.0, 0.0],
            direction: [0.0; 3],
            level: 1,
            npc_data: Some(Box::new(npc(i, pad))),
            player_data: None,
        });
    }
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(WITNESS, addr)])));

    super::deferred_flush::flush_deferred_aoi(
        WITNESS,
        addr,
        "on_client_ready",
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let session = MercuryEncryption::from_session_key([0u8; 32]);
    typed
        .filter_to(addr)
        .iter()
        .map(|raw| session.decrypt(raw).unwrap())
        .collect()
}

fn messages_of(unpack: &[ClientUnpack]) -> Vec<(u8, Vec<u8>)> {
    unpack.iter().flat_map(|u| u.messages.clone()).collect()
}

/// What the client must dispatch: phase 1 (each NPC's create + avatar update)
/// then phase 2 (the cascades), as the flush composes them.
fn expected_messages(pad: usize) -> Vec<(u8, Vec<u8>)> {
    let mut expected = Vec::new();
    for i in 0..NPC_COUNT {
        let base = compose_create_entity_base_body(
            entity_id(i),
            CLASS_ID,
            [i as f32 * 3.0, 0.0, 0.0],
            [0.0; 3],
        );
        expected.extend(unpack_packet_bodies(&[&base]).messages);
    }
    let phase2 = cascade_body(pad);
    expected.extend(unpack_packet_bodies(&[&phase2]).messages);
    expected
}

/// Every message the flush composes reaches the client model, in order, for a
/// fixture where a raw 1300-byte split would put a header at a cut. Removing
/// the header guard in `plan_fragments` fails this.
#[tokio::test]
async fn flush_of_24_npcs_reaches_the_client_whole_when_a_raw_cut_would_split_a_header() {
    let pad = pad_where_raw_split_breaks_the_client();
    let packets = flush(pad).await;

    assert!(
        packets
            .iter()
            .any(|p| parse_incoming(p).unwrap().flags & FLAG_FRAGMENTED != 0),
        "the fixture must exercise the fragmented cascade bundle"
    );

    let bundles = unpack_reliable_stream(&packets);
    for (n, b) in bundles.iter().enumerate() {
        assert_eq!(b.abort, None, "bundle {n} is abandoned by the client");
    }
    assert_eq!(
        messages_of(&bundles),
        expected_messages(pad),
        "every create and every cascade message must be dispatched, in order"
    );
}

/// The guarantee holds as the cut moves with the data.
#[tokio::test]
async fn flush_reaches_the_client_whole_for_every_cascade_size() {
    for pad in [0usize, 3, 7, 11, 19, 30] {
        let packets = flush(pad).await;
        let bundles = unpack_reliable_stream(&packets);
        assert!(
            bundles.iter().all(|b| b.abort.is_none()),
            "pad {pad}: a bundle is abandoned by the client"
        );
        assert_eq!(messages_of(&bundles), expected_messages(pad), "pad {pad}");
    }
}
