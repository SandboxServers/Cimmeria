//! Direct world-entry sends retain the session that built their ciphertext.

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use cimmeria_base_session::base::SessionOriginSeed;
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
    assert_eq!(wire.rows.lock().unwrap()[0].0.as_deref(), Some("A"));
    assert!(connected.lock().unwrap()[&peer()]
        .channel
        .lock()
        .unwrap()
        .tx_window
        .is_empty());
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
    assert!(rows.iter().all(|row| row.0.as_deref() == Some("A")));
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
            "{family} lost an origin-aware send"
        );
    }
    for (family, source) in [
        (
            "base_character_create",
            include_str!("../../../../base/src/base/character_create/mod.rs"),
        ),
        (
            "base_dispatch",
            include_str!("../../../../base/src/base/dispatch/session.rs"),
        ),
        (
            "base_eviction",
            include_str!("../../../../base/src/base/login/eviction.rs"),
        ),
    ] {
        assert!(
            source.contains("send_to_with_origin("),
            "{family} lost its origin-aware send"
        );
    }
    let login = include_str!("../../../../base/src/base/login/mod.rs");
    assert!(
        login.contains("pub(crate) use eviction::handle_log_off;"),
        "account logoff must retain its teardown handler"
    );
    let eviction = include_str!("../../../../base/src/base/login/eviction.rs");
    assert!(
        eviction.contains("send_direct(transport, &pkt, addr, origin.as_ref())"),
        "account logoff lost its retained ticket"
    );
    let dispatch = include_str!("../../../../base/src/base/dispatch/session.rs");
    assert_eq!(
        dispatch.matches("send_direct(transport, &pkt").count(),
        2,
        "both logoff forms need their original ticket"
    );
}
