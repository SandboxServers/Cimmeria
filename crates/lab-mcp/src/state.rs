//! Shared state handle for the lab MCP tools.
//!
//! Everything a tool needs to inspect or drive the running server is reachable
//! from here: the [`Orchestrator`] (cell channel, base sessions, DB pool) and
//! the admin API's [`LogBuffer`] ring (for `server_log_tail`). It is the same
//! `Arc<Orchestrator>` handle the admin API gets in `main.rs`.

use std::net::SocketAddr;
use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::{mpsc, oneshot};

use cimmeria_admin_api::ws::broadcast_layer::LogBuffer;
use cimmeria_services::base::OnlinePlayer;
use cimmeria_services::cell::messages::{BaseToCellMsg, LabQuery, LabQueryReply};
use cimmeria_services::orchestrator::Orchestrator;

/// Shared, cheaply-cloneable handle passed to every tool.
#[derive(Clone)]
pub struct LabState {
    orchestrator: Arc<Orchestrator>,
    log_buffer: LogBuffer,
}

impl LabState {
    pub fn new(orchestrator: Arc<Orchestrator>, log_buffer: LogBuffer) -> Self {
        Self {
            orchestrator,
            log_buffer,
        }
    }

    /// Clone the base→cell sender, if the cell service is wired up. Used by the
    /// console-exec and content-reload tools. Returns `None` only if the cell
    /// channel was never set (should not happen after `start_all`).
    pub async fn cell_tx(&self) -> Option<mpsc::Sender<BaseToCellMsg>> {
        let state = self.orchestrator.state();
        let state = state.read().await;
        state.cell_tx.clone()
    }

    /// Clone the Postgres pool, if connected. `PgPool` is an `Arc` internally,
    /// so the clone is cheap and lets us drop the state lock before querying.
    pub async fn db_pool(&self) -> Option<PgPool> {
        let state = self.orchestrator.state();
        let state = state.read().await;
        state.db.as_ref().map(|d| d.pool().clone())
    }

    /// Snapshot of currently-connected players (entity id, name, zone, …).
    pub async fn online_players(&self) -> Vec<OnlinePlayer> {
        let state = self.orchestrator.state();
        let state = state.read().await;
        state.base.online_players()
    }

    /// Snapshot of the recent-log ring buffer (oldest first).
    pub fn log_snapshot(&self) -> Vec<cimmeria_admin_api::ws::broadcast_layer::LogEntry> {
        self.log_buffer.snapshot()
    }

    /// Send a read-only [`LabQuery`] to the cell loop and await its reply.
    ///
    /// Mirrors the `LabConsoleExec` round-trip: clone the cell sender, send the
    /// query with a `oneshot` reply channel, and translate the reply's own
    /// `Result` plus the two channel-failure modes into one `Result`. Used by
    /// the `server_entity_*` / `server_witnesses` tools.
    pub async fn lab_query(&self, query: LabQuery) -> Result<LabQueryReply, String> {
        let Some(tx) = self.cell_tx().await else {
            return Err("cell service channel not available".to_string());
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(BaseToCellMsg::LabQuery { query, reply_tx })
            .await
            .map_err(|e| format!("failed to send to cell service: {e}"))?;
        match reply_rx.await {
            Ok(result) => result,
            Err(_) => Err("cell service dropped the reply (shutting down?)".to_string()),
        }
    }

    /// Resolve a connected player's session socket address by entity id.
    ///
    /// Reads the base `online_players` roster (the same source
    /// `server_sessions` reports) and parses the session field. `None` when no
    /// in-world player carries that entity id, or its session string does not
    /// parse as a socket address. Used by `server_packet_tap_start` to bind a
    /// tap's inbound half to the right socket.
    pub async fn session_addr_for_entity(&self, entity_id: u32) -> Option<SocketAddr> {
        self.online_players()
            .await
            .into_iter()
            .find(|p| p.id == entity_id)
            .and_then(|p| p.session.parse().ok())
    }
}
