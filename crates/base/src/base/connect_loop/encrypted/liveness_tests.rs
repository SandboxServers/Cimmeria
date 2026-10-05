//! Guards for `liveness`: only a datagram that brings something new
//! refreshes the session's inactivity clock.
//!
//! PR #1246 review, finding 2: the refresh ran straight after decryption,
//! so replaying one captured datagram (it passes the HMAC) kept a session
//! registered forever. A sniffer, or a squatter on a spoofed address, could
//! hold a dead session up. These drive real encrypted datagrams through
//! `handle_encrypted_datagram` and watch `last_recv`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL, FLAG_RELIABLE};
use cimmeria_mercury::test_transport::TestTransport;
use cimmeria_mercury::transport::Transport;

use crate::base::ConnectedClientState;
use crate::test_support::test_default_connected_client_state;

use super::handle_encrypted_datagram;

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;

struct Rig {
    addr: SocketAddr,
    connected: Connected,
    last_recv: Arc<Mutex<Instant>>,
    long_ago: Instant,
}

fn rig(port: u16) -> Rig {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut state = test_default_connected_client_state();
    state.channel = Mutex::new(crate::base::login::new_client_channel(addr));
    let last_recv = Arc::clone(&state.last_recv);
    Rig {
        addr,
        connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        last_recv,
        long_ago: Instant::now()
            .checked_sub(Duration::from_secs(30))
            .expect("host uptime must exceed 30 s"),
    }
}

impl Rig {
    /// An encrypted client datagram carrying one ignored AUTHENTICATE.
    fn datagram(&self, reliable: bool, seq: u32) -> Vec<u8> {
        let body = [0x01, 0x04, 0x00, 0, 0, 0, 0];
        let mut flags = FLAG_ON_CHANNEL | FLAG_HAS_SEQUENCE;
        if reliable {
            flags |= FLAG_RELIABLE;
        }
        let plain = build_outgoing(flags, &body, Some(seq), &[], None);
        let enc = self.connected.lock().unwrap()[&self.addr].enc.clone();
        enc.encrypt(&plain).unwrap()
    }

    fn backdate(&self) {
        *self.last_recv.lock().unwrap() = self.long_ago;
    }

    fn refreshed(&self) -> bool {
        *self.last_recv.lock().unwrap() != self.long_ago
    }

    async fn deliver(&self, raw: &[u8]) {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let (enc, key, pending_acks) = {
            let map = self.connected.lock().unwrap();
            let s = &map[&self.addr];
            (s.enc.clone(), s.key, Arc::clone(&s.pending_acks))
        };
        handle_encrypted_datagram(
            &transport,
            self.addr,
            raw,
            enc,
            key,
            0,
            &pending_acks,
            &self.connected,
            &None,
            &None,
            &Arc::new(Mutex::new(EntityManager::new())),
            &None,
            &Arc::new(Mutex::new(HashMap::new())),
        )
        .await
        .unwrap();
    }
}

/// A new reliable packet refreshes the clock; the same datagram replayed
/// does not (the receive gate calls it a duplicate). Fails with the refresh
/// back before the gate.
#[tokio::test]
async fn a_replayed_reliable_datagram_does_not_keep_the_session_alive() {
    let rig = rig(52701);
    let packet = rig.datagram(true, 0);

    rig.backdate();
    rig.deliver(&packet).await;
    assert!(rig.refreshed(), "the first delivery is new traffic");

    rig.backdate();
    rig.deliver(&packet).await;
    rig.deliver(&packet).await;
    assert!(!rig.refreshed(), "a replay must not refresh last_recv");
}

/// The idle in-world client's traffic (unreliable, on its own advancing
/// counter) keeps the session alive; a replay of one of those datagrams
/// does not.
#[tokio::test]
async fn unreliable_traffic_refreshes_only_while_its_sequence_advances() {
    let rig = rig(52702);
    let first = rig.datagram(false, 40);

    rig.backdate();
    rig.deliver(&first).await;
    assert!(rig.refreshed(), "the first unreliable packet counts");

    rig.backdate();
    rig.deliver(&rig.datagram(false, 41)).await;
    assert!(rig.refreshed(), "an advancing sequence is a live client");

    rig.backdate();
    rig.deliver(&first).await;
    rig.deliver(&rig.datagram(false, 41)).await;
    assert!(
        !rig.refreshed(),
        "replayed unreliable datagrams must not refresh last_recv"
    );
}
