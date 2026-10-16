//! Send-origin regressions for the session layer's owned outbound tickets.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_mercury::channel_bundle::ChannelBundle;
use cimmeria_mercury::packet::{Bytes, Packet};
use cimmeria_mercury::transport::{SendOrigin, Transport};
use tokio::sync::Notify;

use super::cooked_sync::{self, SyncJob};
use super::helpers::{
    collect_pending_retransmits_for_session, send_bundle_to_witness_reliable,
    send_reliable_to_addr, send_to_witness,
};
use super::{ConnectedClientState, SessionOriginSeed};
use crate::test_support::test_default_connected_client_state;

struct OriginTransport {
    sent: Mutex<Vec<(Option<String>, Vec<u8>)>>,
    pause_next: AtomicBool,
    entered: Notify,
    release: Notify,
}

impl OriginTransport {
    fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
            pause_next: AtomicBool::new(false),
            entered: Notify::new(),
            release: Notify::new(),
        }
    }

    async fn record(&self, bytes: &[u8], origin: Option<String>) -> std::io::Result<usize> {
        if self.pause_next.swap(false, Ordering::SeqCst) {
            self.entered.notify_one();
            self.release.notified().await;
        }
        self.sent.lock().unwrap().push((origin, bytes.to_vec()));
        Ok(bytes.len())
    }
}

impl Transport for OriginTransport {
    fn send_to<'a, 'b, 'f>(
        &'a self,
        bytes: &'b [u8],
        _addr: SocketAddr,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<usize>> + Send + 'f>>
    where
        'a: 'f,
        'b: 'f,
        Self: 'f,
    {
        Box::pin(self.record(bytes, None))
    }

    fn send_to_with_origin<'a, 'b, 'c, 'f>(
        &'a self,
        bytes: &'b [u8],
        _addr: SocketAddr,
        origin: &'c SendOrigin,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<usize>> + Send + 'f>>
    where
        'a: 'f,
        'b: 'f,
        'c: 'f,
        Self: 'f,
    {
        Box::pin(self.record(bytes, Some(origin.session_generation.to_string())))
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok("127.0.0.1:1".parse().unwrap())
    }
}

fn seeded(addr: SocketAddr, generation: &str) -> ConnectedClientState {
    let mut state = test_default_connected_client_state();
    state.extensions.insert(SessionOriginSeed {
        server_boot_id: Arc::from("boot"),
        session_generation: Arc::from(generation),
        peer: addr,
        connected_at: state.connected_at,
    });
    state
}

#[tokio::test]
async fn send_origin_reliable_single_stays_a_across_same_peer_replacement() {
    let addr: SocketAddr = "127.0.0.1:54051".parse().unwrap();
    let clients = Arc::new(Mutex::new(HashMap::from([(addr, seeded(addr, "A"))])));
    let recorder = Arc::new(OriginTransport::new());
    recorder.pause_next.store(true, Ordering::SeqCst);
    let transport: Arc<dyn Transport> = recorder.clone();
    let clients_task = Arc::clone(&clients);
    let send = tokio::spawn(async move {
        send_reliable_to_addr(&transport, &clients_task, addr, 1, "proof", |_, _, _, _| {
            vec![0xA5; 48]
        })
        .await
    });
    tokio::time::timeout(Duration::from_secs(2), recorder.entered.notified())
        .await
        .unwrap();
    clients.lock().unwrap().insert(addr, seeded(addr, "B"));
    recorder.release.notify_one();
    let _ = send.await.unwrap();
    assert_eq!(
        recorder.sent.lock().unwrap()[0],
        (Some("A".into()), vec![0xA5; 48])
    );
    assert!(clients.lock().unwrap()[&addr]
        .channel
        .lock()
        .unwrap()
        .tx_window
        .is_empty());
}

#[tokio::test]
async fn send_origin_witness_single_and_bundle_use_the_recipient_session() {
    let addr: SocketAddr = "127.0.0.1:54052".parse().unwrap();
    let clients = Arc::new(Mutex::new(HashMap::from([(addr, seeded(addr, "A"))])));
    let mapping = Arc::new(Mutex::new(HashMap::from([(7, addr)])));
    let recorder = Arc::new(OriginTransport::new());
    let transport: Arc<dyn Transport> = recorder.clone();
    let _ = send_to_witness(&transport, &clients, &mapping, 7, |_, _, _, _| {
        vec![0x5A; 32]
    })
    .await;
    let mut bundle = ChannelBundle::new(true);
    bundle.append_raw_message(&[0x01, 0x00, 0x00]);
    let _ = send_bundle_to_witness_reliable(&transport, &clients, &mapping, 7, bundle).await;
    let sent = recorder.sent.lock().unwrap();
    assert_eq!(sent.len(), 2);
    assert!(sent
        .iter()
        .all(|(origin, _)| origin.as_deref() == Some("A")));
    assert_eq!(sent[0].1, vec![0x5A; 32]);
}

struct ManualClock {
    base: Instant,
    offset: Mutex<Duration>,
}

impl cimmeria_mercury::clock::Clock for ManualClock {
    fn now(&self) -> Instant {
        self.base + *self.offset.lock().unwrap()
    }
}

#[test]
fn send_origin_retransmit_bytes_and_origin_are_from_one_channel() {
    let addr: SocketAddr = "127.0.0.1:54053".parse().unwrap();
    let clock = Arc::new(ManualClock {
        base: Instant::now(),
        offset: Mutex::new(Duration::ZERO),
    });
    let wire = vec![0xC3; 48];
    let mut a = seeded(addr, "A");
    let cancelled_a = Arc::clone(&a.cancelled);
    let mut channel = cimmeria_mercury::channel::Channel::with_clock(addr, clock.clone());
    channel
        .register_sent_packet(
            Packet::new(Default::default(), 1, Bytes::new()),
            Bytes::from(wire.clone()),
        )
        .unwrap();
    a.channel = Mutex::new(channel);
    let clients = Arc::new(Mutex::new(HashMap::from([(addr, a)])));
    *clock.offset.lock().unwrap() += Duration::from_secs(5);
    let pending = collect_pending_retransmits_for_session(&clients, addr, Some(&cancelled_a));
    assert_eq!(pending.len(), 1);
    assert_eq!(&pending[0].bytes[..], &wire);
    assert_eq!(
        pending[0]
            .origin
            .as_ref()
            .unwrap()
            .session_generation
            .as_ref(),
        "A"
    );
    clients.lock().unwrap().insert(addr, seeded(addr, "B"));
    assert!(collect_pending_retransmits_for_session(&clients, addr, Some(&cancelled_a)).is_empty());
    assert_eq!(&pending[0].bytes[..], &wire);
    assert_eq!(
        pending[0]
            .origin
            .as_ref()
            .unwrap()
            .session_generation
            .as_ref(),
        "A"
    );
}

#[tokio::test]
async fn send_origin_old_tick_cannot_scan_replacement_channel() {
    let addr: SocketAddr = "127.0.0.1:54054".parse().unwrap();
    let clock = Arc::new(ManualClock {
        base: Instant::now(),
        offset: Mutex::new(Duration::ZERO),
    });
    let mut a = seeded(addr, "A");
    let mut channel = cimmeria_mercury::channel::Channel::with_clock(addr, clock.clone());
    channel
        .register_sent_packet(
            Packet::new(Default::default(), 1, Bytes::new()),
            Bytes::from(vec![0xA1; 48]),
        )
        .unwrap();
    a.channel = Mutex::new(channel);
    let cancelled_a = Arc::clone(&a.cancelled);
    let next = Arc::clone(&a.next_seq_unreliable);
    let pending = Arc::clone(&a.pending_acks);
    let last_recv = Arc::clone(&a.last_recv);
    let clients = Arc::new(Mutex::new(HashMap::from([(addr, a)])));
    let recorder = Arc::new(OriginTransport::new());
    recorder.pause_next.store(true, Ordering::SeqCst);
    let transport: Arc<dyn Transport> = recorder.clone();
    *clock.offset.lock().unwrap() += Duration::from_secs(5);

    // Constructing this future captures A before Tokio can poll it.
    let loop_future = super::tick_sync::run_tick_loop(
        transport,
        addr,
        [0; 32],
        Default::default(),
        next,
        pending,
        last_recv,
        cancelled_a,
        Arc::clone(&clients),
        Arc::new(Mutex::new(cimmeria_entity::manager::EntityManager::new())),
        None,
        Arc::new(Mutex::new(HashMap::new())),
        None,
    );
    let task = tokio::spawn(loop_future);
    tokio::time::timeout(Duration::from_secs(2), recorder.entered.notified())
        .await
        .unwrap();

    let mut b = seeded(addr, "B");
    let mut b_channel = cimmeria_mercury::channel::Channel::with_clock(addr, clock);
    b_channel
        .register_sent_packet(
            Packet::new(Default::default(), 1, Bytes::new()),
            Bytes::from(vec![0xB2; 48]),
        )
        .unwrap();
    b.channel = Mutex::new(b_channel);
    clients.lock().unwrap().insert(addr, b);
    recorder.release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    let sent = recorder.sent.lock().unwrap();
    assert_eq!(sent.len(), 1, "A must not retransmit B's cached packet");
    assert_eq!(sent[0].0.as_deref(), Some("A"));
    assert_ne!(sent[0].1, vec![0xB2; 48]);
}

#[tokio::test]
async fn send_origin_cooked_direct_and_paced_replies_are_tagged() {
    let addr: SocketAddr = "127.0.0.1:54055".parse().unwrap();
    let clients = Arc::new(Mutex::new(HashMap::from([(addr, seeded(addr, "A"))])));
    let recorder = Arc::new(OriginTransport::new());
    let transport: Arc<dyn Transport> = recorder.clone();
    let cache = Arc::new(
        super::resources::ResourceCache::load_all(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/cache"
        ))
        .unwrap(),
    );
    // No server data for this category: the direct response is a single packet.
    super::cooked_data::handle_version_info_request(
        &transport,
        addr,
        [0; 32],
        &[0xFF, 0xFF, 0xFF, 0xFF, 1, 0, 0, 0],
        &clients,
        &None,
    )
    .await
    .unwrap();
    let ctx = cooked_sync::context(&transport, addr, [0; 32], &clients, &cache);
    cooked_sync::start_resync(
        ctx,
        SyncJob {
            category_id: u32::MAX,
            client_version: 1,
            server_version: 2,
        },
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if recorder.sent.lock().unwrap().len() >= 3 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let sent = recorder.sent.lock().unwrap();
    assert!(sent
        .iter()
        .all(|(origin, bytes)| origin.as_deref() == Some("A") && !bytes.is_empty()));
}
