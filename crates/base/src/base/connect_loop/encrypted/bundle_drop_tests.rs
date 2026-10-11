//! TG-MER-06: every message the inbound bundle walk drops leaves a row
//! with a `reason` from the closed set and its level pinned.
//!
//! Each case delivers a real encrypted datagram through
//! [`handle_encrypted_datagram`], so a call site reverted to its old TRACE
//! row (or to no row) fails its case. Each case uses its own port: the drop
//! throttle is keyed by session address and reason.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL, FLAG_RELIABLE};
use cimmeria_mercury::test_transport::TestTransport;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::mercury::types::WorldEntryInfo;
use tokio::sync::mpsc;
use tracing::Level;

use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{test_default_connected_client_state, LogCapture};

use super::handle_encrypted_datagram;

const PLAYER_EID: u32 = 7;
const DROP_MESSAGE: &str = "Inbound client message dropped";

/// Where the session stands when the bundle arrives.
#[derive(Debug, Clone, Copy)]
enum Phase {
    CharacterSelect,
    /// Player entity created, `mapLoaded` not yet received.
    Loading,
    InWorld,
}

/// A word-length message: `[id][u16 len][payload]`.
fn word(id: u8, payload: &[u8]) -> Vec<u8> {
    let mut m = vec![id];
    m.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    m.extend_from_slice(payload);
    m
}

/// Deliver `body` as one reliable packet from a fresh session at `port`.
/// The cell channel is open, its receiver held, so a forwarded message is
/// not a closed-channel drop.
async fn deliver(port: u16, phase: Phase, body: &[u8]) {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut state: ConnectedClientState = test_default_connected_client_state();
    if !matches!(phase, Phase::CharacterSelect) {
        state.player_entity_id = Some(PLAYER_EID);
    }
    if matches!(phase, Phase::Loading) {
        state.pending_map_loaded = Some(WorldEntryInfo {
            player_entity_id: PLAYER_EID,
            space_id: 0x0001_0042,
            pos: [0.0; 3],
            rot: [0.0; 3],
            world_name: "Castle_CellBlock".to_string(),
            class_id: 0x02,
            world_stargates: Vec::new(),
        });
    }
    state.channel = Mutex::new(crate::base::login::new_client_channel(addr));
    let key = state.key;
    let pending_acks = Arc::clone(&state.pending_acks);
    let enc = state.enc.clone();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let (cell_tx, _cell_rx) = mpsc::channel::<BaseToCellMsg>(16);

    let flags = FLAG_ON_CHANNEL | FLAG_HAS_SEQUENCE | FLAG_RELIABLE;
    let plain = build_outgoing(flags, body, Some(0), &[], None);
    let datagram = enc.encrypt(&plain).unwrap();
    handle_encrypted_datagram(
        &transport,
        addr,
        &datagram,
        enc,
        key,
        0,
        &pending_acks,
        &connected,
        &None,
        &None,
        &Arc::new(Mutex::new(EntityManager::new())),
        &Some(cell_tx),
        &Arc::new(Mutex::new(HashMap::new())),
    )
    .await
    .unwrap();
}

/// One case per `reason`: the bundle that triggers it, and the level its
/// row must carry.
#[tokio::test]
async fn bundle_walk_drops_have_reasons() {
    let entity_prefix = PLAYER_EID.to_le_bytes();
    let movement = {
        let mut m = vec![0x03];
        m.extend_from_slice(&[0; 40]);
        m
    };
    let cases: Vec<(&str, Level, u16, Phase, Vec<u8>)> = vec![
        // A length prefix of 16 with no bytes behind it.
        (
            "bundle_truncated",
            Level::WARN,
            47_601,
            Phase::CharacterSelect,
            vec![0xC8, 0x10, 0x00],
        ),
        // VEHICLE_ACK: framed, but the walk has no arm for it.
        (
            "unhandled_msg_id",
            Level::WARN,
            47_602,
            Phase::CharacterSelect,
            {
                let mut m = vec![0x0A];
                m.extend_from_slice(&[0; 8]);
                m
            },
        ),
        (
            "unhandled_account_method",
            Level::WARN,
            47_603,
            Phase::CharacterSelect,
            word(0xC8, &[]),
        ),
        (
            "no_player_entity",
            Level::DEBUG,
            47_604,
            Phase::CharacterSelect,
            movement,
        ),
        (
            "before_world_entry",
            Level::DEBUG,
            47_605,
            Phase::CharacterSelect,
            word(0x85, &entity_prefix),
        ),
        (
            "map_loaded_pending",
            Level::DEBUG,
            47_606,
            Phase::Loading,
            word(0x85, &entity_prefix),
        ),
        (
            "malformed_args",
            Level::WARN,
            47_607,
            Phase::InWorld,
            word(0x85, &[0, 0]),
        ),
        (
            "subslot_missing_index",
            Level::WARN,
            47_608,
            Phase::InWorld,
            word(0xBD, &entity_prefix),
        ),
    ];

    let mut failures = Vec::new();
    for (reason, level, port, phase, body) in cases {
        let capture = LogCapture::install();
        deliver(port, phase, &body).await;
        match capture.find_event(level, DROP_MESSAGE, reason) {
            Some(row) if row.has_field("event", "inbound_message_dropped") => {}
            Some(_) => failures.push(format!("{reason}: row has no event field")),
            None => failures.push(format!(
                "{reason}: no {level} row; saw {:#?}",
                capture
                    .all()
                    .iter()
                    .filter(|c| c.fields.contains_key("reason"))
                    .collect::<Vec<_>>()
            )),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A truncation names where in the bundle it happened.
#[tokio::test]
async fn bundle_truncated_row_carries_offset_and_body_len() {
    let capture = LogCapture::install();
    // A whole VIEWPORT_ACK, then a base method cut short.
    let mut body = vec![0x09];
    body.extend_from_slice(&[0; 8]);
    body.extend_from_slice(&[0xC8, 0x10, 0x00]);
    deliver(47_609, Phase::CharacterSelect, &body).await;
    let row = capture
        .find_event(Level::WARN, DROP_MESSAGE, "bundle_truncated")
        .expect("truncation row");
    assert!(row.has_field("offset", "9"), "{row:#?}");
    assert!(row.has_field("body_len", "12"), "{row:#?}");
}

/// A cell method held for `mapLoaded` drops only itself: the next message
/// in the same bundle is still walked, and carries the method pair.
#[tokio::test]
async fn map_loaded_pending_drops_only_its_message() {
    let capture = LogCapture::install();
    let mut body = word(0x85, &PLAYER_EID.to_le_bytes());
    body.push(0x0A);
    body.extend_from_slice(&[0; 8]);
    deliver(47_610, Phase::Loading, &body).await;
    let held = capture
        .find_event(Level::DEBUG, DROP_MESSAGE, "map_loaded_pending")
        .expect("held row");
    assert!(held.has_field("method_index", "5"), "{held:#?}");
    assert!(
        capture
            .find_event(Level::WARN, DROP_MESSAGE, "unhandled_msg_id")
            .is_some(),
        "the walk stopped after the held message"
    );
}
