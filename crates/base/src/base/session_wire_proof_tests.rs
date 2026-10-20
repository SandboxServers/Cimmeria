//! Regression fixtures for Base's immutable send tickets and proof coverage.

use super::*;
use crate::test_support::test_default_connected_client_state;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

struct Wire {
    started: Notify,
    release: Notify,
    pause: AtomicBool,
    sent: Mutex<Vec<SocketAddr>>,
}

impl Wire {
    fn new() -> Self {
        Self {
            started: Notify::new(),
            release: Notify::new(),
            pause: AtomicBool::new(false),
            sent: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl Transport for Wire {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        if self.pause.load(Ordering::SeqCst) && bytes == b"paused" {
            self.started.notify_one();
            self.release.notified().await;
        }
        self.sent.lock().unwrap().push(addr);
        match bytes {
            b"failed" => Err(io::Error::other("fake send failure")),
            b"partial" => Ok(bytes.len() - 1),
            _ => Ok(bytes.len()),
        }
    }

    fn try_send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.sent.lock().unwrap().push(addr);
        Ok(bytes.len())
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
}

#[async_trait]
impl BidirectionalTransport for Wire {
    async fn recv_from(&self, _buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        Err(io::Error::other("unused fake recv"))
    }
}

fn peer(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn connected() -> Arc<Connected> {
    Arc::new(Mutex::new(HashMap::new()))
}

fn install(
    connected: &Arc<Connected>,
    peer: SocketAddr,
    seed: cimmeria_base_session::base::SessionOriginSeed,
) {
    let mut session = test_default_connected_client_state();
    session.extensions.insert(seed);
    connected.lock().unwrap().insert(peer, session);
}

#[path = "session_wire_proof_tests/observer.rs"]
mod observer;
#[path = "session_wire_proof_tests/service.rs"]
mod service;
