//! Who gets a connection task on the public minigame port, and for how long.
//!
//! The port is internet-facing and every peer is anonymous until its ticket
//! checks out, so two limits sit in front of the SmartFox handshake:
//!
//! - a **connection cap** — past [`ConnectionLimits::max_connections`] a new
//!   socket is closed on accept instead of getting a task and a buffer;
//! - a **handshake deadline** — enforced in `handshake.rs`, it drops a
//!   socket that has not finished verChk + login in time.
//!
//! Neither existed in the original server (`deprecated/cpp/src/baseapp/
//! minigame_connection.cpp` reads without a timeout and never caps). Real
//! demand is at most one socket per online player: a session is one per
//! entity and its room is `maxu='1'`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::TcpListener;
use tokio::sync::{mpsc, Semaphore};

use super::handle_connection;
use crate::cell::messages::CellToBaseMsg;
use crate::minigame::session::SessionRegistry;

/// Minimum gap between two "at the connection cap" warnings. WARN events
/// are forwarded to Discord, so a flood must not post once per socket.
const CAP_WARN_INTERVAL: Duration = Duration::from_secs(60);

/// Per-listener limits on anonymous connections.
#[derive(Debug, Clone, Copy)]
pub(super) struct ConnectionLimits {
    /// Most connections served at once. The next one is closed on accept.
    pub max_connections: usize,
    /// How long a connection has to finish verChk + login, from accept.
    pub handshake_timeout: Duration,
}

/// Accept connections forever, spawning one task per admitted socket.
pub(super) async fn serve(
    listener: TcpListener,
    registry: SessionRegistry,
    result_tx: mpsc::Sender<CellToBaseMsg>,
    external_port: u16,
    limits: ConnectionLimits,
) {
    let slots = Arc::new(Semaphore::new(limits.max_connections));
    let mut last_cap_warn: Option<Instant> = None;
    let mut refused_since_warn: u64 = 0;

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(e) => {
                tracing::warn!(error = %e, "Minigame accept error");
                continue;
            }
        };

        // Taken before the spawn and moved into the task, so the slot is
        // released on every exit path of the connection, panics included.
        let Ok(slot) = Arc::clone(&slots).try_acquire_owned() else {
            drop(stream);
            refused_since_warn += 1;
            if last_cap_warn.is_none_or(|at| at.elapsed() >= CAP_WARN_INTERVAL) {
                tracing::warn!(
                    %peer,
                    max_connections = limits.max_connections,
                    refused = refused_since_warn,
                    reason = "connection_cap",
                    "Minigame connection refused: server is at its connection cap"
                );
                last_cap_warn = Some(Instant::now());
                refused_since_warn = 0;
            } else {
                tracing::debug!(%peer, reason = "connection_cap", "Minigame connection refused");
            }
            continue;
        };

        tracing::debug!(peer = %peer, "Minigame connection accepted");
        let registry = registry.clone();
        let result_tx = result_tx.clone();
        tokio::spawn(async move {
            handle_connection(
                stream,
                peer,
                registry,
                result_tx,
                external_port,
                limits.handshake_timeout,
            )
            .await;
            drop(slot);
        });
    }
}
