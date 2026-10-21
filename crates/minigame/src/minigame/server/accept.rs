//! The accept loop: bind, admit one connection per valid ticket through
//! `SessionRegistry::try_admit` (every other socket is closed before a byte
//! is read), and hand admitted ones to a connection task.

use std::net::SocketAddr;
use std::time::Duration;

use cimmeria_wire::cell::messages::CellToBaseMsg;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::Instant;

use super::handle_connection;
use super::limits::ListenerLimits;
use crate::minigame::session::{SessionRegistry, PENDING_SESSION_TTL, SWEEP_INTERVAL};

/// How often a throttled row (a scanner refusal, an accept error) may
/// be written. Occurrences in between are counted and reported on the next
/// row as `suppressed` (negative-logging Pattern D).
pub(super) const LOG_WINDOW: Duration = Duration::from_secs(60);

/// Silence before the first keepalive probe.
pub(super) const KEEPALIVE_TIME: Duration = Duration::from_secs(60);
/// Gap between unanswered probes.
pub(super) const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
/// Unanswered probes before the OS resets the socket. With the two above,
/// a vanished peer on an idle connection is dropped about 90 s after it last
/// sent anything. A connection with unacknowledged data in flight is
/// governed by TCP retransmission and `SEND_TIMEOUT` instead.
pub(super) const KEEPALIVE_RETRIES: u32 = 3;

/// Start the minigame TCP server with the default [`ListenerLimits`].
pub async fn run(
    addr: &str,
    port: u16,
    external_port: u16,
    registry: SessionRegistry,
    result_tx: mpsc::Sender<CellToBaseMsg>,
) {
    run_with_limits(
        addr,
        port,
        external_port,
        registry,
        result_tx,
        ListenerLimits::default(),
    )
    .await;
}

/// Start the minigame TCP server with explicit limits.
pub async fn run_with_limits(
    addr: &str,
    port: u16,
    external_port: u16,
    registry: SessionRegistry,
    result_tx: mpsc::Sender<CellToBaseMsg>,
    limits: ListenerLimits,
) {
    let listen_addr = format!("{addr}:{port}");
    let listener = match TcpListener::bind(&listen_addr).await {
        Ok(l) => {
            tracing::info!(
                addr = %listen_addr,
                handshake_timeout_s = limits.handshake_timeout.as_secs(),
                idle_timeout_s = limits.idle_timeout.as_secs(),
                "Minigame server listening",
            );
            l
        }
        Err(e) => {
            tracing::error!(addr = %listen_addr, error = %e, "Failed to bind minigame server");
            return;
        }
    };

    // Defect B4: a session whose SWF never connects has no connection task
    // to clean it up, so without this sweep it pins its entity id in the
    // registry until the player relogs.
    // The sweep also reports each expiry to the cell as result 3, so the
    // player's minigame hold (D-MG15) is released.
    registry.spawn_sweep_reporting(PENDING_SESSION_TTL, SWEEP_INTERVAL, result_tx.clone());

    serve(listener, external_port, registry, result_tx, limits).await;
}

/// The accept loop over an already-bound listener. Split from
/// [`run_with_limits`] so tests can bind an ephemeral port.
pub(super) async fn serve(
    listener: TcpListener,
    external_port: u16,
    registry: SessionRegistry,
    result_tx: mpsc::Sender<CellToBaseMsg>,
    limits: ListenerLimits,
) {
    let mut accept_error_log = LogThrottle::new(LOG_WINDOW);
    let mut unexpected_peer_log = LogThrottle::new(LOG_WINDOW);
    let mut no_free_ticket_log = LogThrottle::new(LOG_WINDOW);

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                // Before any byte is read: a scanner, or a second socket for
                // a ticket already in use, gets a closed socket and nothing
                // else. The guard is the ticket slot and lives exactly as
                // long as the connection task.
                let admission = match registry.try_admit(peer.ip()).await {
                    Ok(guard) => guard,
                    Err(refusal) => {
                        drop(stream);
                        let pending_sessions = refusal.pending_sessions;
                        match refusal.reason {
                            // With sessions registered, a real player may be
                            // the one refused (NAT, multi-WAN, a TTL sweep
                            // race), so that row is unthrottled. With none,
                            // it is most likely a scanner.
                            "unexpected_peer_with_sessions" => tracing::info!(
                                %peer,
                                reason = refusal.reason,
                                pending_sessions,
                                "Minigame connection refused: sessions are registered but none expects this address",
                            ),
                            "unexpected_peer" => {
                                if let Some(suppressed) = unexpected_peer_log.admit(Instant::now())
                                {
                                    tracing::info!(
                                        %peer,
                                        reason = refusal.reason,
                                        suppressed,
                                        "Minigame connection refused: no minigame session expects this address",
                                    );
                                }
                            }
                            // Every ticket for this address is in use or
                            // expired: a real player may be the one turned
                            // away (a reconnect racing its old socket), so
                            // INFO, but once a window since a peer who holds
                            // no ticket can repeat this at socket rate. The arm is a
                            // catch-all so a new reason is still logged, under its own
                            // name; the assert makes a typo in a reason string fail
                            // in tests instead of logging as `no_free_ticket`.
                            reason => {
                                debug_assert_eq!(
                                    reason, "no_free_ticket",
                                    "unknown refusal reason"
                                );
                                if let Some(suppressed) = no_free_ticket_log.admit(Instant::now()) {
                                    tracing::info!(
                                        %peer,
                                        reason,
                                        pending_sessions,
                                        suppressed,
                                        "Minigame connection refused: no free ticket for this address",
                                    );
                                }
                            }
                        }
                        continue;
                    }
                };
                enable_keepalive(&stream, peer);
                tracing::debug!(peer = %peer, "Minigame connection accepted");
                tokio::spawn(handle_connection(
                    stream,
                    peer,
                    admission,
                    registry.clone(),
                    result_tx.clone(),
                    external_port,
                    limits.clone(),
                ));
            }
            Err(e) => {
                // Out of descriptors keeps failing until a connection
                // closes, so this can repeat many times a second: WARN once
                // a window, with the count.
                if let Some(suppressed) = accept_error_log.admit(Instant::now()) {
                    tracing::warn!(
                        error = %e,
                        reason = "accept_error",
                        suppressed,
                        "Minigame accept error",
                    );
                }
                // Without a pause this loop spins.
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

/// Turn on TCP keepalive so a peer that vanished without a FIN or RST
/// (a pulled cable, a NAT that dropped its mapping) is noticed. Without it a
/// half-open socket keeps its session claimed until the idle timeout.
///
/// Probes start after [`KEEPALIVE_TIME`] of silence and repeat every
/// [`KEEPALIVE_INTERVAL`]; after [`KEEPALIVE_RETRIES`] unanswered probes the
/// OS resets the socket and the pending read fails. Failing to set it is
/// not fatal: the idle timeout still bounds the session.
pub(super) fn enable_keepalive(stream: &TcpStream, peer: SocketAddr) {
    let keepalive = socket2::TcpKeepalive::new()
        .with_time(KEEPALIVE_TIME)
        .with_interval(KEEPALIVE_INTERVAL)
        .with_retries(KEEPALIVE_RETRIES);
    if let Err(e) = socket2::SockRef::from(stream).set_tcp_keepalive(&keepalive) {
        tracing::debug!(%peer, error = %e, reason = "keepalive_unset", "Minigame could not enable TCP keepalive");
    }
}

/// Once-per-window gate for a log row that can repeat at socket rate
/// (negative-logging Pattern D). The first occurrence emits at once; the
/// rest of the window is counted and reported on the next emitted row.
#[derive(Debug)]
pub(super) struct LogThrottle {
    window: Duration,
    last: Option<Instant>,
    suppressed: u64,
}

impl LogThrottle {
    pub(super) fn new(window: Duration) -> Self {
        Self {
            window,
            last: None,
            suppressed: 0,
        }
    }

    /// `Some(suppressed)` if a row should be written now, carrying how many
    /// occurrences were held back since the last one; `None` to skip it.
    pub(super) fn admit(&mut self, now: Instant) -> Option<u64> {
        if self
            .last
            .is_some_and(|t| now.duration_since(t) < self.window)
        {
            self.suppressed += 1;
            return None;
        }
        self.last = Some(now);
        Some(std::mem::take(&mut self.suppressed))
    }
}
