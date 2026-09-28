//! Tests for the full-category resync (#840).
//!
//! The rig drives the real `handle_version_info_request` and resync task
//! against the committed PAKs, over a `TestTransport`. A simulated client
//! acks whatever is outstanding on the session's channel, and
//! [`ClientModel`] replays every packet the server sent through the
//! client's cache semantics (`ServerSource<N>::onVersionInfo` `0x00441630`,
//! the proxy-data handler `0x0043dad0`) to show what the client ends up
//! holding.

mod decision;
mod misses;
mod ordering;
mod pacing;
mod resync;
mod robustness;
mod telemetry;

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::transport::Transport;

use super::super::cooked_data::handle_version_info_request;
use super::super::resources::ResourceCache;
use super::super::ConnectedClientState;
use crate::mercury::{FRAG_FIRST, FRAG_FIRST_AND_LAST, FRAG_LAST};
use crate::test_support::{test_default_connected_client_state, TestTransport};

/// Wire ids (crate-private in `cimmeria-wire`).
pub(super) const BASEMSG_ON_VERSION_INFO: u8 = 0x80;
pub(super) const BASEMSG_RESOURCE_FRAGMENT: u8 = 0x36;
/// Extended entity-method marker, and `onVersionInfo`'s sub-index on
/// SGWPlayer (client method 96 minus idbase 61).
const EXTENDED_ENTITY_METHOD: u8 = 0xBD;
const PLAYER_ON_VERSION_INFO_SUB_INDEX: u8 = 35;
/// The fixture session's key (`test_default_connected_client_state`).
const KEY: [u8; 32] = [0u8; 32];

pub(super) fn committed_cache() -> Arc<ResourceCache> {
    static CACHE: OnceLock<Arc<ResourceCache>> = OnceLock::new();
    Arc::clone(CACHE.get_or_init(|| {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
        Arc::new(ResourceCache::load_all(dir).expect("committed PAKs load"))
    }))
}

pub(super) fn server_version(category_id: u32) -> u32 {
    committed_cache().category(category_id).unwrap().metadata
}

/// One connected session. Every test uses its own port: the resync
/// registry is keyed by address and shared across the test binary.
pub(super) struct Rig {
    pub(super) addr: SocketAddr,
    pub(super) sent: Arc<TestTransport>,
    pub(super) transport: Arc<dyn Transport>,
    pub(super) connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub(super) cache: Arc<ResourceCache>,
}

pub(super) fn rig(port: u16) -> Rig {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let sent = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = sent.clone();
    Rig {
        addr,
        sent,
        transport,
        connected: Arc::new(Mutex::new(HashMap::from([(
            addr,
            test_default_connected_client_state(),
        )]))),
        cache: committed_cache(),
    }
}

impl Rig {
    /// Answer one `versionInfoRequest` the way the receive loop does.
    pub(super) async fn request(&self, category_id: u32, client_version: u32) {
        let mut payload = category_id.to_le_bytes().to_vec();
        payload.extend_from_slice(&client_version.to_le_bytes());
        handle_version_info_request(
            &self.transport,
            self.addr,
            KEY,
            &payload,
            &self.connected,
            &Some(Arc::clone(&self.cache)),
        )
        .await
        .expect("versionInfoRequest handled");
    }

    /// Reliable packets outstanding on the session (TX window plus the
    /// deferred-send queue).
    pub(super) fn in_flight(&self) -> usize {
        let clients = self.connected.lock().unwrap();
        let Some(state) = clients.get(&self.addr) else {
            return 0;
        };
        let ch = state.channel.lock().unwrap();
        ch.tx_window.len() + ch.unsent_packets.len()
    }

    /// The simulated client acks everything outstanding. Returns how many
    /// packets were outstanding just before.
    pub(super) fn ack_all(&self) -> usize {
        let clients = self.connected.lock().unwrap();
        let Some(state) = clients.get(&self.addr) else {
            return 0;
        };
        let mut ch = state.channel.lock().unwrap();
        let seqs: Vec<u32> = ch
            .tx_window
            .iter()
            .chain(ch.unsent_packets.iter())
            .map(|e| e.packet.sequence)
            .collect();
        ch.process_ack_footer(&seqs);
        seqs.len()
    }

    /// Ack `n` of the oldest outstanding packets.
    pub(super) fn ack_oldest(&self, n: usize) {
        let clients = self.connected.lock().unwrap();
        let state = &clients[&self.addr];
        let mut ch = state.channel.lock().unwrap();
        let seqs: Vec<u32> = ch
            .tx_window
            .iter()
            .chain(ch.unsent_packets.iter())
            .take(n)
            .map(|e| e.packet.sequence)
            .collect();
        ch.process_ack_footer(&seqs);
    }

    pub(super) fn syncing(&self) -> bool {
        super::is_syncing(&self.connected, self.addr)
    }

    /// The client asks for one entry (`elementDataRequest`).
    pub(super) fn miss(&self, category_id: u32, key: u32) -> super::MissOutcome {
        super::serve_miss(test_context(self), category_id, key)
    }

    /// Put the session in the world as player entity `eid`.
    pub(super) fn enter_world(&self, eid: u32) {
        self.connected
            .lock()
            .unwrap()
            .get_mut(&self.addr)
            .unwrap()
            .player_entity_id = Some(eid);
    }

    /// Run the simulated client until every queued resync has finished and
    /// everything is acked. Returns the most packets ever outstanding.
    pub(super) async fn pump_until_idle(&self) -> usize {
        let mut max_in_flight = 0;
        let run = async {
            loop {
                tokio::task::yield_now().await;
                let outstanding = self.ack_all();
                max_in_flight = max_in_flight.max(outstanding);
                if !self.syncing() && outstanding == 0 {
                    // One more turn so a task finishing its last send is seen.
                    tokio::task::yield_now().await;
                    if self.in_flight() == 0 && !self.syncing() {
                        break;
                    }
                }
            }
        };
        tokio::time::timeout(Duration::from_secs(300), run)
            .await
            .expect("resync did not finish");
        max_in_flight
    }

    /// Give the task `turns` scheduler turns without acking anything.
    pub(super) async fn idle_turns(&self, turns: usize) {
        for _ in 0..turns {
            tokio::task::yield_now().await;
        }
    }

    /// Every packet sent so far, decrypted, in send order; clears the log.
    pub(super) fn take_plaintexts(&self) -> Vec<Vec<u8>> {
        let enc = MercuryEncryption::from_session_key(KEY);
        self.sent
            .drain()
            .into_iter()
            .filter(|(a, _)| *a == self.addr)
            .map(|(_, pkt)| enc.decrypt(&pkt).expect("decrypt"))
            .collect()
    }
}

/// Test contexts poll with a zero interval (a scheduler yield), so the
/// simulated client's acks interleave with the push without real sleeps.
pub(super) fn test_context(rig: &Rig) -> super::SyncContext {
    super::SyncContext {
        poll: Duration::ZERO,
        ..super::context(&rig.transport, rig.addr, KEY, &rig.connected, &rig.cache)
    }
}

fn u32_at(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}

/// A decoded `onVersionInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VersionInfo {
    pub(super) category: u32,
    pub(super) version: u32,
    pub(super) required: u32,
    pub(super) invalidate_all: bool,
    pub(super) keys: Vec<u32>,
}

fn version_info_args(a: &[u8]) -> VersionInfo {
    let count = u32_at(a, 13) as usize;
    VersionInfo {
        category: u32_at(a, 0),
        version: u32_at(a, 4),
        required: u32_at(a, 8),
        invalidate_all: a[12] != 0,
        keys: (0..count).map(|i| u32_at(a, 17 + i * 4)).collect(),
    }
}

/// One packet the server sent, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Sent {
    VersionInfo(VersionInfo),
    Fragment {
        data_id: u16,
        flags: u8,
        head: Option<(u32, u32)>,
        body: Vec<u8>,
    },
    Other(u8),
}

pub(super) fn decode(pt: &[u8]) -> Sent {
    let len = u16::from_le_bytes([pt[2], pt[3]]) as usize;
    let p = &pt[4..4 + len];
    match pt[1] {
        // Account client method 0: [account id][args].
        BASEMSG_ON_VERSION_INFO => Sent::VersionInfo(version_info_args(&p[4..])),
        // SGWPlayer client method 96, in-world: [player id][96 - 61][args].
        EXTENDED_ENTITY_METHOD if p[4] == PLAYER_ON_VERSION_INFO_SUB_INDEX => {
            Sent::VersionInfo(version_info_args(&p[5..]))
        }
        BASEMSG_RESOURCE_FRAGMENT => {
            let data_id = u16::from_le_bytes([p[0], p[1]]);
            let flags = p[3];
            if matches!(flags, FRAG_FIRST | FRAG_FIRST_AND_LAST) {
                Sent::Fragment {
                    data_id,
                    flags,
                    head: Some((u32_at(p, 5), u32_at(p, 9))),
                    body: p[13..].to_vec(),
                }
            } else {
                Sent::Fragment {
                    data_id,
                    flags,
                    head: None,
                    body: p[4..].to_vec(),
                }
            }
        }
        other => Sent::Other(other),
    }
}

/// What the client holds for one category.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CategoryState {
    pub(super) version: u32,
    pub(super) required_updates: u32,
    pub(super) entries: BTreeMap<u32, Vec<u8>>,
}

/// The client's cache, as the RE'd handlers treat it.
#[derive(Debug, Default)]
pub(super) struct ClientModel {
    pub(super) categories: HashMap<u32, CategoryState>,
    partial: HashMap<u16, (u32, u32, Vec<u8>)>,
    /// Entries written per category, counting repeats.
    pub(super) writes: HashMap<u32, u32>,
}

impl ClientModel {
    /// A client holding `category` at `version` with `entries`.
    pub(super) fn holding(category: u32, version: u32, entries: BTreeMap<u32, Vec<u8>>) -> Self {
        let mut m = Self::default();
        m.categories.insert(
            category,
            CategoryState {
                version,
                required_updates: 0,
                entries,
            },
        );
        m
    }

    /// Whether the client would send `elementDataRequest` for `category`
    /// now: every per-category request function tests
    /// `*(int *)(this + 0x48) == 0` (`0x00cfe060`, `0x00d20150`, ...).
    pub(super) fn requests_misses(&self, category: u32) -> bool {
        self.categories
            .get(&category)
            // Before any onVersionInfo the constructor's LONG_MAX blocks them.
            .is_some_and(|c| c.required_updates == 0)
    }

    pub(super) fn apply_all(&mut self, plaintexts: &[Vec<u8>]) {
        for pt in plaintexts {
            self.apply(&decode(pt));
        }
    }

    pub(super) fn apply(&mut self, sent: &Sent) {
        match sent {
            Sent::VersionInfo(v) => {
                let cat = self.categories.entry(v.category).or_default();
                cat.required_updates = v.required;
                if v.invalidate_all {
                    cat.entries.clear();
                } else {
                    for k in &v.keys {
                        cat.entries.remove(k);
                    }
                }
                cat.version = v.version;
            }
            Sent::Fragment {
                data_id,
                flags,
                head,
                body,
            } => {
                if let Some((category, element)) = head {
                    self.partial
                        .insert(*data_id, (*category, *element, body.clone()));
                } else if let Some(p) = self.partial.get_mut(data_id) {
                    p.2.extend_from_slice(body);
                }
                if matches!(*flags, FRAG_LAST | FRAG_FIRST_AND_LAST) {
                    let (category, element, xml) =
                        self.partial.remove(data_id).expect("transfer started");
                    let cat = self.categories.entry(category).or_default();
                    // `0x0043dad0`: `if (*(int *)(this + 0x48) != 0) { ... + -1; }`.
                    // Guarded, so it stops at 0; it never wraps or goes
                    // negative.
                    if cat.required_updates != 0 {
                        cat.required_updates -= 1;
                    }
                    cat.entries.insert(element, xml);
                    *self.writes.entry(category).or_default() += 1;
                }
            }
            Sent::Other(_) => {}
        }
    }
}

/// The server's entries for `category`, sorted.
pub(super) fn server_entries(category_id: u32) -> BTreeMap<u32, Vec<u8>> {
    committed_cache()
        .category(category_id)
        .unwrap()
        .elements
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect()
}
