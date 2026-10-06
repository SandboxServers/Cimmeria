//! World entry never sends a reliable datagram larger than the client's
//! 1472-byte receive buffer, however long the player's appearance is.
//!
//! The create-player packet and the enter-world bundle both carry the full
//! `BeingAppearance` (body set plus every component name), so their bodies are
//! data-sized. A datagram over the buffer never reaches the client and wedges
//! its reliable stream behind it (`mercury.tx_hole`), which here means a
//! player stuck on the loading screen.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::consts::PACKET_MAX_SIZE;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{parse_incoming, ParsedPacket, FLAG_FRAGMENTED};
use cimmeria_mercury::transport::Transport;

use super::super::ConnectedClientState;
use crate::mercury::types::{PlayerLoadData, WorldEntryInfo};
use crate::test_support::TestTransport;

const ENTITY: u32 = 4242;

fn entry() -> WorldEntryInfo {
    WorldEntryInfo {
        player_entity_id: ENTITY,
        space_id: 0x0001_0042,
        pos: [10.0, 20.0, 30.0],
        rot: [0.0; 3],
        world_name: "Agnos".to_string(),
        class_id: 2,
        world_stargates: Vec::new(),
    }
}

/// A player wearing far more named pieces than any seed outfit: about 1.9 KB
/// of `BeingAppearance` arguments, past what one datagram holds.
fn long_appearance() -> PlayerLoadData {
    let mut data = super::methods::default_player_load_data();
    data.components = (0..30)
        .map(|i| format!("AR_J_Praxis.AR_JM_PT1_PT100PT101PC100PS{i:03}"))
        .collect();
    data.weapon_visual = None;
    data
}

/// A session with a long ACK backlog, which is what pushed the 2026-10-05
/// cascade over the buffer.
fn session(client: ConnectedClientState) -> Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> {
    client.pending_acks.lock().unwrap().extend(1..60);
    Arc::new(Mutex::new(HashMap::from([(addr(), client)])))
}

fn addr() -> SocketAddr {
    "127.0.0.1:55610".parse().unwrap()
}

/// Every datagram sent, checked against the buffer, decrypted and parsed.
fn sent(tt: &TestTransport) -> Vec<ParsedPacket> {
    let enc = MercuryEncryption::from_session_key([0u8; 32]);
    tt.filter_to(addr())
        .into_iter()
        .enumerate()
        .map(|(i, wire)| {
            assert!(
                wire.len() <= PACKET_MAX_SIZE,
                "datagram {i} is {} bytes, over the client's {PACKET_MAX_SIZE}-byte buffer",
                wire.len()
            );
            parse_incoming(&enc.decrypt(&wire).unwrap()).unwrap()
        })
        .collect()
}

/// Reliable sequence numbers run on from 0 without a gap.
fn assert_contiguous(packets: &[ParsedPacket]) {
    for (i, p) in packets.iter().enumerate() {
        assert_eq!(p.seq_id, Some(i as u32), "packet {i}");
    }
}

/// CREATE_BASE_PLAYER + appearance pre-warm + onClientMapLoad, with an
/// appearance too long for one datagram: fragmented, and the world entry
/// still stages `pending_map_loaded`.
#[tokio::test]
async fn create_player_with_a_long_appearance_fits_the_client_buffer() {
    let tt = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = tt.clone();
    let mut client = crate::test_support::test_default_connected_client_state();
    client.pending_player_entity_id = Some(ENTITY);
    client.pending_world_entry = Some(entry());
    client.pending_player_load_data = Some(long_appearance());
    let connected = session(client);

    super::enable_entities::handle_enable_entities(
        &transport,
        addr(),
        [0u8; 32],
        1,
        &connected,
        &None,
        &Arc::new(Mutex::new(EntityManager::new())),
        &None,
        &Arc::new(Mutex::new(HashMap::new())),
    )
    .await
    .expect("create player sends");

    let packets = sent(&tt);
    assert!(packets.len() >= 2, "fragmented: {}", packets.len());
    assert!(packets.iter().all(|p| p.flags & FLAG_FRAGMENTED != 0));
    assert_contiguous(&packets);
    let clients = connected.lock().unwrap();
    assert!(clients[&addr()].pending_map_loaded.is_some());
}

/// The enter-world bundle (viewport, appearance, createCellPlayer,
/// forcedPosition) and the mapLoaded methods after it, with the same long
/// appearance: every datagram fits and the two bundles take one contiguous
/// run of sequence numbers.
#[tokio::test]
async fn enter_world_with_a_long_appearance_fits_the_client_buffer() {
    let tt = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = tt.clone();
    let mut client = crate::test_support::test_default_connected_client_state();
    client.pending_map_loaded = Some(entry());
    client.pending_player_load_data = Some(long_appearance());
    let connected = session(client);

    super::map_loaded::handle_map_loaded(
        &transport,
        addr(),
        [0u8; 32],
        &connected,
        &None,
        &Arc::new(Mutex::new(HashMap::new())),
        &None,
    )
    .await
    .expect("enter world sends");

    let packets = sent(&tt);
    assert_contiguous(&packets);
    // The enter-world bundle is its own fragment group, starting at seq 0.
    let enter = &packets[0];
    assert_ne!(enter.flags & FLAG_FRAGMENTED, 0, "enter world fragmented");
    let enter_end = enter.frag_end.unwrap();
    let body: Vec<u8> = packets[..=enter_end as usize]
        .iter()
        .flat_map(|p| p.body.iter().copied())
        .collect();
    let expected = crate::mercury::build_enter_world_body(&entry(), Some(&long_appearance()));
    assert_eq!(body, expected, "the enter-world fragments reassemble");
}
