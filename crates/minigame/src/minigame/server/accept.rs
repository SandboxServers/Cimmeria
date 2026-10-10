//! The accept loop: bind, admit or refuse each socket against
//! [`ListenerLimits`], and hand admitted ones to a connection task.

use std::time::Duration;

use cimmeria_wire::cell::messages::CellToBaseMsg;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::time::Instant;

use super::handle_connection;
use super::limits::{ConnectionTracker, ListenerLimits, Refusal};
use crate::minigame::session::{SessionRegistry, PENDING_SESSION_TTL, SWEEP_INTERVAL};

/// How often a full-server refusal may log at INFO. Refusals in between
/// are counted and reported on the next row as `suppressed`
/// (negative-logging Pattern D).
const TOTAL_CAP_LOG_WINDOW: Duration = Duration::from_secs(60);

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
                max_connections = limits.max_connections,
                max_connections_per_ip = limits.max_connections_per_ip,
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
    registry.spawn_sweep(PENDING_SESSION_TTL, SWEEP_INTERVAL);

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
    let tracker = ConnectionTracker::new(&limits);
    let mut total_cap_log = TotalCapLog::default();

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let permit = match tracker.try_acquire(peer.ip()) {
                    Ok(permit) => permit,
                    Err(refusal) => {
                        // Closing the socket is the whole refusal: SFS has
                        // no "server full" frame the SWF would show.
                        drop(stream);
                        match refusal {
                            Refusal::PerIpCap => tracing::debug!(
                                %peer,
                                reason = refusal.reason(),
                                limit = limits.max_connections_per_ip,
                                "Minigame connection refused",
                            ),
                            Refusal::TotalCap => {
                                total_cap_log.refused(peer, tracker.open(), &limits)
                            }
                        }
                        continue;
                    }
                };
                tracing::debug!(peer = %peer, "Minigame connection accepted");
                tokio::spawn(handle_connection(
                    stream,
                    peer,
                    permit,
                    registry.clone(),
                    result_tx.clone(),
                    external_port,
                    limits.clone(),
                ));
            }
            Err(e) => {
                tracing::warn!(error = %e, "Minigame accept error");
                // Out of descriptors keeps failing until a connection
                // closes; without a pause this loop spins and floods.
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

/// Throttle for the full-server refusal row. A full server can turn away a
/// real player, so it logs at INFO rather than DEBUG, but under a flood one
/// row a minute with a count is enough.
#[derive(Default)]
struct TotalCapLog {
    last: Option<Instant>,
    suppressed: u64,
}

impl TotalCapLog {
    fn refused(&mut self, peer: std::net::SocketAddr, open: usize, limits: &ListenerLimits) {
        let now = Instant::now();
        if self
            .last
            .is_some_and(|t| now.duration_since(t) < TOTAL_CAP_LOG_WINDOW)
        {
            self.suppressed += 1;
            return;
        }
        tracing::info!(
            %peer,
            reason = Refusal::TotalCap.reason(),
            open,
            limit = limits.max_connections,
            suppressed = self.suppressed,
            "Minigame connection refused: server at its connection cap",
        );
        self.last = Some(now);
        self.suppressed = 0;
    }
}
