//! Session-entry fence at the three send chokepoints: a send to the guarded
//! address is dropped when its session state no longer belongs to the entry,
//! before any sequence is reserved or ACK drained; every other send is
//! unchanged.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_SGW_PLAYER};
use cimmeria_mercury::transport::Transport;

use super::{
    scoped, send_bundle_to_witness_reliable, send_to_witness, send_to_witness_reliable,
    BundleSendOutcome, ConnectedClientState, EntryGuard, WitnessSendOutcome,
};
use crate::test_support::{test_default_connected_client_state, TestTransport};

const WITNESS: u32 = 900;
const OTHER_WITNESS: u32 = 901;

/// One session at `port` (the witness `WITNESS`), with two pending ACKs so the
/// tests can see whether a send drained them.
struct Harness {
    tt: Arc<TestTransport>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    addr: SocketAddr,
}

fn harness(port: u16) -> Harness {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let tt = Arc::new(TestTransport::default());
    let transport: Arc<dyn Transport> = tt.clone();
    let state = test_default_connected_client_state();
    state.pending_acks.lock().unwrap().extend([7, 8]);
    Harness {
        tt,
        transport,
        connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        entity_to_addr: Arc::new(Mutex::new(HashMap::from([(WITNESS, addr)]))),
        addr,
    }
}

async fn send_reliable(h: &Harness) -> WitnessSendOutcome {
    send_to_witness_reliable(
        &h.transport,
        &h.connected,
        &h.entity_to_addr,
        WITNESS,
        |_key, _version, _seq, _acks| vec![0xAA; 12],
    )
    .await
}

async fn send_unreliable(h: &Harness) -> WitnessSendOutcome {
    send_to_witness(
        &h.transport,
        &h.connected,
        &h.entity_to_addr,
        WITNESS,
        |_key, _version, _seq, _acks| vec![0xAA; 12],
    )
    .await
}

async fn send_bundle(h: &Harness) -> BundleSendOutcome {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(5, IDBASE_SGW_PLAYER, WITNESS, &[1, 2, 3]);
    send_bundle_to_witness_reliable(
        &h.transport,
        &h.connected,
        &h.entity_to_addr,
        WITNESS,
        bundle,
    )
    .await
}

/// `(next_seq, next_seq_unreliable, pending ACKs)` of the session at `h.addr`.
fn counters(h: &Harness) -> (u32, u32, Vec<u32>) {
    let guard = h.connected.lock().unwrap();
    let c = &guard[&h.addr];
    let acks = c.pending_acks.lock().unwrap().clone();
    (
        c.next_seq.load(Ordering::Relaxed),
        c.next_seq_unreliable.load(Ordering::Relaxed),
        acks,
    )
}

#[tokio::test]
async fn entry_guard_none_leaves_sends_unchanged() {
    let h = harness(56100);
    assert!(send_reliable(&h).await.is_sent());
    assert!(send_unreliable(&h).await.is_sent());
    assert!(send_bundle(&h).await.is_sent());
    assert_eq!(h.tt.filter_to(h.addr).len(), 3, "three datagrams");
    let (next, unreliable, acks) = counters(&h);
    assert_eq!(next, 2, "reliable and bundle each take one sequence");
    assert_eq!(unreliable, 1, "unreliable takes one unreliable sequence");
    assert!(acks.is_empty(), "sends drain the pending ACKs");
}

#[tokio::test]
async fn entry_guard_current_allows_all_three_send_paths() {
    let h = harness(56101);
    let (reliable, unreliable, bundle) = scoped(EntryGuard::new(h.addr, |_| true), async {
        (
            send_reliable(&h).await,
            send_unreliable(&h).await,
            send_bundle(&h).await,
        )
    })
    .await;
    assert!(reliable.is_sent(), "{reliable:?}");
    assert!(unreliable.is_sent(), "{unreliable:?}");
    assert!(bundle.is_sent(), "{bundle:?}");
    assert_eq!(h.tt.filter_to(h.addr).len(), 3, "three datagrams");
}

#[tokio::test]
async fn entry_guard_superseded_reliable_send_reserves_no_sequence_and_sends_nothing() {
    let h = harness(56102);
    let outcome = scoped(EntryGuard::new(h.addr, |_| false), send_reliable(&h)).await;
    assert!(
        matches!(outcome, WitnessSendOutcome::ClientDisconnected),
        "{outcome:?}"
    );
    assert!(h.tt.filter_to(h.addr).is_empty(), "nothing on the wire");
    assert_eq!(counters(&h), (0, 0, vec![7, 8]));
}

#[tokio::test]
async fn entry_guard_superseded_bundle_send_reserves_no_sequence_and_drains_no_acks() {
    let h = harness(56103);
    let outcome = scoped(EntryGuard::new(h.addr, |_| false), send_bundle(&h)).await;
    assert!(
        matches!(outcome, BundleSendOutcome::ClientDisconnected),
        "{outcome:?}"
    );
    assert!(h.tt.filter_to(h.addr).is_empty(), "nothing on the wire");
    assert_eq!(counters(&h), (0, 0, vec![7, 8]));
}

#[tokio::test]
async fn entry_guard_superseded_unreliable_send_takes_no_unreliable_sequence() {
    let h = harness(56104);
    let outcome = scoped(EntryGuard::new(h.addr, |_| false), send_unreliable(&h)).await;
    assert!(
        matches!(outcome, WitnessSendOutcome::ClientDisconnected),
        "{outcome:?}"
    );
    assert!(h.tt.filter_to(h.addr).is_empty(), "nothing on the wire");
    assert_eq!(counters(&h), (0, 0, vec![7, 8]));
}

#[tokio::test]
async fn entry_guard_other_address_is_not_affected() {
    let h = harness(56105);
    let other: SocketAddr = "127.0.0.1:56106".parse().unwrap();
    h.connected
        .lock()
        .unwrap()
        .insert(other, test_default_connected_client_state());
    h.entity_to_addr
        .lock()
        .unwrap()
        .insert(OTHER_WITNESS, other);

    // The guard names `h.addr` and rejects everything; the send goes to `other`.
    let outcome = scoped(EntryGuard::new(h.addr, |_| false), async {
        send_to_witness_reliable(
            &h.transport,
            &h.connected,
            &h.entity_to_addr,
            OTHER_WITNESS,
            |_key, _version, _seq, _acks| vec![0xAA; 12],
        )
        .await
    })
    .await;
    assert!(outcome.is_sent(), "{outcome:?}");
    assert_eq!(h.tt.filter_to(other).len(), 1, "other player's packet sent");
}

#[tokio::test]
async fn entry_guard_scope_ends_with_the_future() {
    let h = harness(56107);
    scoped(EntryGuard::new(h.addr, |_| false), async {}).await;
    assert!(send_reliable(&h).await.is_sent());
    assert_eq!(h.tt.filter_to(h.addr).len(), 1, "send after the scope");
}
