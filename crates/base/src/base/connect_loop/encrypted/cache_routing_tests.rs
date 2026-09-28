//! `0xC0`/`0xC1` routing by phase (#840), and world entry held behind a
//! cooked-data resync.
//!
//! At character select `0xC0`/`0xC1` are `versionInfoRequest` /
//! `elementDataRequest`. In-world they are `SGWPlayer.chatJoin` /
//! `chatLeave`. The encrypted loop used to send both phases to the cache
//! handlers, so the client's login rejoin of its default user channels
//! (`channel-chat`, `channel-roleplay`, `channel-alliance`) was read as a
//! request for category 12 or 16 at version `0x00680063` ("ch") and
//! answered with an InvalidateAll that emptied category 16 on every login
//! (colo SigNoz, 2026-09-28).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL, FLAG_RELIABLE};
use cimmeria_mercury::test_transport::TestTransport;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::mercury::write_wstring;
use tracing::Level;

use crate::base::resources::ResourceCache;
use crate::base::{cooked_sync, ConnectedClientState};
use crate::test_support::{test_default_connected_client_state, LogCapture};

use super::handle_encrypted_datagram;

const WITNESS_ID: u32 = 7;
const BASEMSG_ON_VERSION_INFO: u8 = 0x80;
const WORLD_INFO: u32 = 12;
const STARGATES: u32 = 13;

fn committed_cache() -> Arc<ResourceCache> {
    static CACHE: OnceLock<Arc<ResourceCache>> = OnceLock::new();
    Arc::clone(CACHE.get_or_init(|| {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
        Arc::new(ResourceCache::load_all(dir).expect("committed PAKs load"))
    }))
}

struct Rig {
    addr: SocketAddr,
    key: [u8; 32],
    sent: Arc<TestTransport>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pending_acks: Arc<Mutex<Vec<u32>>>,
    entity_manager: Arc<Mutex<EntityManager>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    next_seq: u32,
}

/// A session at character select, or in-world when `in_world`.
fn rig(port: u16, in_world: bool) -> Rig {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut state = test_default_connected_client_state();
    if in_world {
        state.player_entity_id = Some(WITNESS_ID);
    }
    state.channel = Mutex::new(crate::base::login::new_client_channel(addr));
    let key = state.key;
    let pending_acks = Arc::clone(&state.pending_acks);
    let sent = Arc::new(TestTransport::new());
    Rig {
        addr,
        key,
        transport: sent.clone(),
        sent,
        connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        pending_acks,
        entity_manager: Arc::new(Mutex::new(EntityManager::new())),
        entity_to_addr: Arc::new(Mutex::new(HashMap::new())),
        next_seq: 0,
    }
}

impl Rig {
    /// Deliver one reliable client packet carrying message `msg_id`.
    async fn deliver(&mut self, msg_id: u8, payload: &[u8]) {
        let mut body = vec![msg_id];
        body.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        body.extend_from_slice(payload);
        let flags = FLAG_ON_CHANNEL | FLAG_HAS_SEQUENCE | FLAG_RELIABLE;
        let plain = build_outgoing(flags, &body, Some(self.next_seq), &[], None);
        self.next_seq += 1;
        let enc = self.connected.lock().unwrap()[&self.addr].enc.clone();
        let datagram = enc.encrypt(&plain).unwrap();
        handle_encrypted_datagram(
            &self.transport,
            self.addr,
            &datagram,
            enc,
            self.key,
            0,
            &self.pending_acks,
            &self.connected,
            &None,
            &Some(committed_cache()),
            &self.entity_manager,
            &None,
            &self.entity_to_addr,
        )
        .await
        .unwrap();
    }

    /// Message ids of everything the server has sent, decrypted.
    fn sent_msg_ids(&self) -> Vec<u8> {
        let enc = self.connected.lock().unwrap()[&self.addr].enc.clone();
        self.sent
            .filter_to(self.addr)
            .iter()
            .map(|pkt| enc.decrypt(pkt).unwrap()[1])
            .collect()
    }

    fn ack_all(&self) {
        let clients = self.connected.lock().unwrap();
        let mut ch = clients[&self.addr].channel.lock().unwrap();
        let seqs: Vec<u32> = ch
            .tx_window
            .iter()
            .chain(ch.unsent_packets.iter())
            .map(|e| e.packet.sequence)
            .collect();
        ch.process_ack_footer(&seqs);
    }

    fn world_entry_sent(&self) -> bool {
        self.connected.lock().unwrap()[&self.addr].world_entry_sent
    }
}

fn chat_join_payload(channel: &str) -> Vec<u8> {
    let mut p = Vec::new();
    write_wstring(&mut p, channel);
    write_wstring(&mut p, "");
    p
}

fn version_request(category_id: u32, version: u32) -> Vec<u8> {
    let mut p = category_id.to_le_bytes().to_vec();
    p.extend_from_slice(&version.to_le_bytes());
    p
}

fn saw_version_reply(capture: &crate::test_support::LogCaptureGuard) -> bool {
    capture
        .all()
        .iter()
        .any(|e| e.message_contains("Responding to versionInfoRequest"))
}

/// In-world `0xC0` is `chatJoin`: the client's login rejoin of
/// `channel-chat` reaches the chat handler and no cache reply goes out.
#[tokio::test]
async fn in_world_0xc0_is_chat_join() {
    let mut rig = rig(47_501, true);
    let capture = LogCapture::install();
    rig.deliver(0xC0, &chat_join_payload("channel-chat")).await;

    assert!(
        capture.all().iter().any(|e| e.message_contains("chatJoin")
            && e.has_field("channel_name", "\"channel-chat\"")
            || e.message_contains("chatJoin") && e.has_field("channel_name", "channel-chat")),
        "chatJoin handler not reached; saw {:#?}",
        capture.all()
    );
    assert!(
        !saw_version_reply(&capture),
        "chatJoin was read as a versionInfoRequest"
    );
    assert!(!rig.sent_msg_ids().contains(&BASEMSG_ON_VERSION_INFO));
    assert!(!cooked_sync::is_syncing(&rig.connected, rig.addr));
}

/// In-world `0xC1` is `chatLeave`.
#[tokio::test]
async fn in_world_0xc1_is_chat_leave() {
    let mut rig = rig(47_502, true);
    let capture = LogCapture::install();
    rig.deliver(0xC1, &[3]).await;
    assert!(
        capture
            .all()
            .iter()
            .any(|e| e.message_contains("chatLeave") && e.has_field("channel_id", "3")),
        "chatLeave handler not reached; saw {:#?}",
        capture.all()
    );
}

/// At character select `0xC0` is still `versionInfoRequest`: an up-to-date
/// category gets its `onVersionInfo`. Guards against gating too much.
#[tokio::test]
async fn pre_world_0xc0_is_a_version_request() {
    let mut rig = rig(47_503, false);
    let capture = LogCapture::install();
    let served = committed_cache().category(STARGATES).unwrap().metadata;
    rig.deliver(0xC0, &version_request(STARGATES, served)).await;
    assert!(saw_version_reply(&capture));
    assert!(rig.sent_msg_ids().contains(&BASEMSG_ON_VERSION_INFO));
}

/// `playCharacter` during a resync is held, then runs once the resync is
/// pushed and acked.
#[tokio::test]
async fn play_character_waits_for_the_resync() {
    let mut rig = rig(47_504, false);
    let capture = LogCapture::install();
    rig.deliver(0xC0, &version_request(WORLD_INFO, 5959)).await;
    assert!(cooked_sync::is_syncing(&rig.connected, rig.addr));

    rig.deliver(0xC4, &71i32.to_le_bytes()).await;
    assert!(
        !rig.world_entry_sent(),
        "world entry must wait for the resync"
    );
    assert!(capture
        .all()
        .iter()
        .any(|e| e.level == Level::INFO && e.has_field("event", "cooked_data.world_entry_held")));

    let done = async {
        while cooked_sync::is_syncing(&rig.connected, rig.addr) || !rig.world_entry_sent() {
            rig.ack_all();
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(30), done)
        .await
        .expect("held playCharacter never ran");
    assert!(rig.world_entry_sent());
}

/// No resync running: `playCharacter` goes straight through.
#[tokio::test]
async fn play_character_is_not_held_without_a_resync() {
    let mut rig = rig(47_505, false);
    rig.deliver(0xC4, &71i32.to_le_bytes()).await;
    assert!(rig.world_entry_sent());
}
