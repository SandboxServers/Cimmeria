//! Direct world-entry sends preserve owner fences and explicit proof gaps.

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use cimmeria_base_session::base::SessionOriginSeed;
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::instrumentation::wire_fingerprint;
use cimmeria_mercury::transport::{SendOrigin, Transport};
use tokio::sync::Notify;

use super::super::ConnectedClientState;

struct Wire {
    calls: AtomicUsize,
    entered: Notify,
    release: Notify,
    pause_first: bool,
    fail_on: Option<usize>,
    rows: Mutex<Vec<(Option<String>, String)>>,
}

impl Wire {
    fn new(pause_first: bool, fail_on: Option<usize>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            entered: Notify::new(),
            release: Notify::new(),
            pause_first,
            fail_on,
            rows: Mutex::new(Vec::new()),
        }
    }

    async fn record(&self, bytes: &[u8], origin: Option<&SendOrigin>) -> io::Result<usize> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if call == 1 && self.pause_first {
            self.entered.notify_one();
            self.release.notified().await;
        }
        if self.fail_on == Some(call) {
            return Err(io::Error::other("synthetic send failure"));
        }
        self.rows.lock().unwrap().push((
            origin.map(|origin| origin.session_generation.to_string()),
            wire_fingerprint(bytes),
        ));
        Ok(bytes.len())
    }
}

#[async_trait]
impl Transport for Wire {
    async fn send_to(&self, bytes: &[u8], _addr: SocketAddr) -> io::Result<usize> {
        self.record(bytes, None).await
    }

    async fn send_to_with_origin(
        &self,
        bytes: &[u8],
        _addr: SocketAddr,
        origin: &SendOrigin,
    ) -> io::Result<usize> {
        self.record(bytes, Some(origin)).await
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
}

fn peer() -> SocketAddr {
    "127.0.0.1:55631".parse().unwrap()
}

fn session(generation: &str, long_appearance: bool) -> ConnectedClientState {
    let mut client = crate::test_support::test_default_connected_client_state();
    client.extensions.insert(SessionOriginSeed {
        server_boot_id: Arc::from("boot"),
        session_generation: Arc::from(generation),
        peer: peer(),
        connected_at: Instant::now(),
    });
    client.pending_map_loaded = Some(crate::mercury::types::WorldEntryInfo {
        player_entity_id: 4242,
        space_id: 0x0001_0042,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "Agnos".to_string(),
        class_id: 2,
        world_stargates: Vec::new(),
    });
    if long_appearance {
        let mut data = super::methods::default_player_load_data();
        data.components = (0..30)
            .map(|i| format!("AR_J_Praxis.AR_JM_PT1_PT100PT101PC100PS{i:03}"))
            .collect();
        client.pending_player_load_data = Some(data);
    }
    client
}

async fn enter_world(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    super::map_loaded::handle_map_loaded(
        transport,
        peer(),
        [0; 32],
        connected,
        &None,
        &Arc::new(Mutex::new(HashMap::new())),
        &None,
    )
    .await
}

#[tokio::test]
async fn direct_send_origin_old_world_entry_cannot_register_in_new_peer_window() {
    let connected = Arc::new(Mutex::new(HashMap::from([(peer(), session("A", false))])));
    let wire = Arc::new(Wire::new(true, Some(2)));
    let transport: Arc<dyn Transport> = wire.clone();
    let clients = Arc::clone(&connected);
    let task = tokio::spawn(async move { enter_world(&transport, &clients).await });
    wire.entered.notified().await;
    connected
        .lock()
        .unwrap()
        .insert(peer(), session("B", false));
    wire.release.notify_one();
    assert!(task.await.unwrap().is_err());
    assert_eq!(wire.rows.lock().unwrap()[0].0, None);
    assert!(connected.lock().unwrap()[&peer()]
        .channel
        .lock()
        .unwrap()
        .tx_window
        .is_empty());
}

#[tokio::test]
async fn direct_send_origin_same_key_replacement_before_first_lock_is_unproven() {
    let a = session("A", false);
    let old_key = a.key;
    let connected = Arc::new(Mutex::new(HashMap::from([(peer(), a)])));
    connected
        .lock()
        .unwrap()
        .insert(peer(), session("B", false));
    let wire = Arc::new(Wire::new(false, None));
    let transport: Arc<dyn Transport> = wire.clone();
    assert_eq!(old_key, connected.lock().unwrap()[&peer()].key);
    enter_world(&transport, &connected).await.unwrap();
    assert!(!wire.rows.lock().unwrap().is_empty());
    assert!(wire.rows.lock().unwrap().iter().all(|row| row.0.is_none()));
    let clients = connected.lock().unwrap();
    let channel = clients[&peer()].channel.lock().unwrap();
    assert!(channel
        .tx_window
        .iter()
        .all(|entry| entry.send_kind == Some("direct_unproven")));
}

#[tokio::test]
async fn direct_send_origin_create_player_helper_does_not_prove_replaced_ingress() {
    let a = session("A", false);
    let old_key = a.key;
    let mut b = session("B", false);
    b.pending_player_entity_id = Some(4242);
    b.pending_world_entry = b.pending_map_loaded.take();
    let connected = Arc::new(Mutex::new(HashMap::from([(peer(), a)])));
    connected.lock().unwrap().insert(peer(), b);
    let wire = Arc::new(Wire::new(false, None));
    let transport: Arc<dyn Transport> = wire.clone();
    super::enable_entities::handle_enable_entities(
        &transport,
        peer(),
        old_key,
        1,
        &connected,
        &None,
        &Arc::new(Mutex::new(EntityManager::new())),
        &None,
        &Arc::new(Mutex::new(HashMap::new())),
    )
    .await
    .unwrap();
    assert!(!wire.rows.lock().unwrap().is_empty());
    assert!(wire.rows.lock().unwrap().iter().all(|row| row.0.is_none()));
    let clients = connected.lock().unwrap();
    let channel = clients[&peer()].channel.lock().unwrap();
    assert!(channel
        .tx_window
        .iter()
        .all(|entry| entry.send_kind == Some("create_player_unproven")));
}

#[tokio::test]
async fn direct_send_origin_map_loaded_does_not_commit_a_state_into_b() {
    let connected = Arc::new(Mutex::new(HashMap::from([(peer(), session("A", false))])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let wire = Arc::new(Wire::new(true, None));
    let transport: Arc<dyn Transport> = wire.clone();
    let clients = Arc::clone(&connected);
    let addresses = Arc::clone(&entity_to_addr);
    let task = tokio::spawn(async move {
        super::map_loaded::handle_map_loaded(
            &transport,
            peer(),
            [0; 32],
            &clients,
            &None,
            &addresses,
            &None,
        )
        .await
    });
    wire.entered.notified().await;
    connected
        .lock()
        .unwrap()
        .insert(peer(), session("B", false));
    wire.release.notify_one();
    task.await.unwrap().unwrap();
    assert!(entity_to_addr.lock().unwrap().is_empty());
    assert!(connected.lock().unwrap()[&peer()]
        .pending_client_ready
        .is_none());
}

#[tokio::test]
async fn direct_send_origin_char_list_does_not_mark_b_after_a_send() {
    let mut a = session("A", false);
    a.pending_map_loaded = None;
    let connected = Arc::new(Mutex::new(HashMap::from([(peer(), a)])));
    let wire = Arc::new(Wire::new(true, None));
    let transport: Arc<dyn Transport> = wire.clone();
    let clients = Arc::clone(&connected);
    let task = tokio::spawn(async move {
        super::enable_entities::handle_enable_entities(
            &transport,
            peer(),
            [0; 32],
            1,
            &clients,
            &None,
            &Arc::new(Mutex::new(EntityManager::new())),
            &None,
            &Arc::new(Mutex::new(HashMap::new())),
        )
        .await
    });
    wire.entered.notified().await;
    let mut b = session("B", false);
    b.pending_map_loaded = None;
    connected.lock().unwrap().insert(peer(), b);
    wire.release.notify_one();
    task.await.unwrap().unwrap();
    assert!(!connected.lock().unwrap()[&peer()].char_list_sent);
}

#[tokio::test]
async fn direct_send_origin_fragments_share_a_and_have_distinct_fingerprints() {
    let connected = Arc::new(Mutex::new(HashMap::from([(peer(), session("A", true))])));
    let wire = Arc::new(Wire::new(false, None));
    let transport: Arc<dyn Transport> = wire.clone();
    enter_world(&transport, &connected).await.unwrap();
    let rows = wire.rows.lock().unwrap();
    assert!(
        rows.len() >= 4,
        "long appearance must fragment both bundles"
    );
    assert!(rows.iter().all(|row| row.0.is_none()));
    let unique: std::collections::HashSet<_> = rows.iter().map(|row| &row.1).collect();
    assert_eq!(
        unique.len(),
        rows.len(),
        "every fragment is a distinct wire datagram"
    );
}

#[tokio::test]
async fn direct_send_origin_failed_send_records_no_successful_row() {
    let connected = Arc::new(Mutex::new(HashMap::from([(peer(), session("A", false))])));
    let wire = Arc::new(Wire::new(false, Some(1)));
    let transport: Arc<dyn Transport> = wire.clone();
    assert!(enter_world(&transport, &connected).await.is_err());
    assert!(wire.rows.lock().unwrap().is_empty());
    assert!(connected.lock().unwrap()[&peer()]
        .channel
        .lock()
        .unwrap()
        .tx_window
        .is_empty());
}

#[tokio::test]
async fn direct_send_origin_content_owner_wrapper_rejects_b_ticket_for_a_content() {
    let a = session("A", false);
    let a_seq = Arc::clone(&a.next_seq);
    let a_origin = a.wire_send_origin();
    let connected = Arc::new(Mutex::new(HashMap::from([(peer(), a)])));
    let wire = Arc::new(Wire::new(false, None));
    let inner: Arc<dyn Transport> = wire.clone();
    let guarded = super::content_owner_transport(&inner, &connected, &a_seq, a_origin.as_ref());
    connected
        .lock()
        .unwrap()
        .insert(peer(), session("B", false));
    let b_origin = connected.lock().unwrap()[&peer()]
        .wire_send_origin()
        .unwrap();
    assert!(guarded
        .send_to_with_origin(b"A content", peer(), &b_origin)
        .await
        .is_err());
    assert!(wire.rows.lock().unwrap().is_empty());
}

#[tokio::test]
async fn direct_send_origin_reanchor_stays_unproven_without_cell_generation() {
    let connected = Arc::new(Mutex::new(HashMap::from([(peer(), session("A", false))])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(4242, peer())])));
    let wire = Arc::new(Wire::new(false, None));
    let transport: Arc<dyn Transport> = wire.clone();
    super::reanchor_player::handle_reanchor_player(
        4242,
        0x0001_0042,
        [0.0; 3],
        [0.0; 3],
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await
    .unwrap();
    assert_eq!(wire.rows.lock().unwrap().len(), 1);
    assert!(wire.rows.lock().unwrap().iter().all(|row| row.0.is_none()));
    {
        let clients = connected.lock().unwrap();
        let mut channel = clients[&peer()].channel.lock().unwrap();
        for entry in &mut channel.tx_window {
            assert_eq!(entry.send_kind, Some("direct_unproven"));
            entry.last_sent = Instant::now() - Duration::from_secs(30);
        }
    }
    let retransmits =
        cimmeria_base_session::base::helpers::collect_pending_retransmits(&connected, peer());
    assert!(!retransmits.is_empty());
    assert!(retransmits.iter().all(|send| send.origin.is_none()));
}

#[test]
fn direct_send_origin_registration_retains_send_site_and_kind() {
    let a = session("A", false);
    let seq = Arc::clone(&a.next_seq);
    let origin = a.wire_send_origin();
    let connected = Arc::new(Mutex::new(HashMap::from([(peer(), a)])));
    super::register_unproven_direct_reliable(
        &connected,
        peer(),
        10,
        &[0xA1; 48],
        &seq,
        origin.as_ref(),
    );
    let clients = connected.lock().unwrap();
    let channel = clients[&peer()].channel.lock().unwrap();
    let entry = channel.tx_window.front().unwrap();
    assert_eq!(entry.raw_bytes.as_ref(), &[0xA1; 48]);
    assert_eq!(entry.send_kind, Some("direct_unproven"));
    assert!(entry
        .send_site
        .as_ref()
        .unwrap()
        .contains("direct_send_origin_tests.rs"));
}

#[test]
fn direct_send_origin_family_census() {
    for (family, source, expected) in [
        ("character", include_str!("../character/mod.rs"), 2),
        ("enable_entities", include_str!("enable_entities.rs"), 1),
        ("map_loaded", include_str!("map_loaded.rs"), 2),
        ("reanchor_player", include_str!("reanchor_player.rs"), 1),
        ("play_character", include_str!("play_character/mod.rs"), 1),
        ("gate_travel", include_str!("gate_travel/mod.rs"), 1),
    ] {
        assert_eq!(
            source.matches("send_direct(").count(),
            expected,
            "{family} lost a direct send conversion"
        );
    }
    let create = include_str!("../../../../base/src/base/character_create/mod.rs");
    assert!(create.contains("transport.send_to(&pkt, addr).await?"));
    let login = include_str!("../../../../base/src/base/login/mod.rs");
    assert!(
        login.contains("pub(crate) use eviction::handle_log_off;"),
        "account logoff must retain its teardown handler"
    );
    let eviction = include_str!("../../../../base/src/base/login/eviction.rs");
    assert!(eviction.contains("send_direct(transport, &pkt, old.addr, origin.as_ref())"));
    assert!(eviction.contains("send_direct(transport, &pkt, addr, None)"));
    let dispatch = include_str!("../../../../base/src/base/dispatch/session.rs");
    assert_eq!(
        dispatch
            .matches("send_direct(transport, &pkt, addr, None)")
            .count(),
        2,
        "both ingress logoff forms must stay unproven"
    );
    let map = include_str!("map_loaded.rs");
    assert!(map.contains("same_direct_owner(c, &next_seq, origin.as_ref())"));
    let enable = include_str!("enable_entities.rs");
    assert!(enable.contains("content_owner_transport(transport, connected, &next_seq"));
    assert!(enable.contains("same_direct_owner(c, &list_next_seq, list_origin.as_ref())"));
    let reanchor = include_str!("reanchor_player.rs");
    assert!(reanchor.contains("send_direct(transport, pkt, addr, None)"));
    let character = include_str!("../character/mod.rs");
    assert!(character.contains("Arc::ptr_eq(&c.next_seq, expected_seq)"));
    assert!(create.contains("Arc::ptr_eq(&c.next_seq, &expected_seq)"));
    let gate = include_str!("gate_travel/mod.rs");
    assert!(gate.contains("send_direct(transport, &pkt, addr, None)"));
    let fit = include_str!("../../../../base-session/src/base/helpers/reliable_fit.rs");
    assert!(fit.contains("send_reliable_to_addr_for_owner"));
    // The retransmit scan moved out of reliable_fit.rs (hard line cap).
    let scan = include_str!("../../../../base-session/src/base/helpers/retransmit_scan.rs");
    assert!(scan.contains("kind.ends_with(\"_unproven\")"));
}
