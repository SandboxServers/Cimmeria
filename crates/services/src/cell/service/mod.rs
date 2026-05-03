//! CellService — spatial entity simulation service.
//!
//! In the original C++ architecture, this was the `CellApp` process that:
//! - Managed game spaces (world zones/instances)
//! - Simulated cell entity halves (spatial state, movement, AoI)
//! - Processed entity interactions within spatial proximity
//! - Ran the game tick loop for entity updates

use std::net::SocketAddr;
use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::{mpsc, Notify};
use tokio::task::JoinHandle;

use cimmeria_common::ServerConfig;

use super::messages::{BaseToCellMsg, CellToBaseMsg};

mod base_messages;
mod message_loop;
mod npc_ai;
mod startup;
mod ticks;

#[cfg(test)]
mod tests;

/// CellApp service managing spatial entity simulation.
pub struct CellService {
    /// Address the cell service binds to for BaseApp communication.
    pub listener_addr: SocketAddr,

    /// Whether the service is currently running.
    pub is_running: bool,

    /// Receiver for messages from BaseApp (set by orchestrator before start).
    pub(crate) base_to_cell_rx: Option<mpsc::Receiver<BaseToCellMsg>>,

    /// Sender for messages to BaseApp (set by orchestrator before start).
    pub(crate) cell_to_base_tx: Option<mpsc::Sender<CellToBaseMsg>>,

    /// Path to the entities directory for loading space XML files.
    pub(crate) entities_dir: String,

    /// Database pool for content engine loading (set by orchestrator).
    pub(crate) db_pool: Option<Arc<PgPool>>,

    /// Handle to the spawned cell-loop task. `stop()` notifies and awaits it
    /// so shutdown is deterministic instead of relying on the channel half
    /// being dropped at some unspecified time.
    pub(crate) cell_loop_handle: Option<JoinHandle<()>>,

    /// Signal that asks `run_cell_loop` to break out of its select loop.
    /// Cloned into the task on start; notified by `stop()`.
    pub(crate) shutdown_signal: Option<Arc<Notify>>,
}

impl CellService {
    /// Create a new cell service from server configuration.
    pub fn new(config: &ServerConfig) -> Self {
        let listener_addr = format!("{}:{}", config.cell_host, config.cell_port)
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], config.cell_port)));

        Self {
            listener_addr,
            is_running: false,
            base_to_cell_rx: None,
            cell_to_base_tx: None,
            entities_dir: "entities".to_string(),
            db_pool: None,
            cell_loop_handle: None,
            shutdown_signal: None,
        }
    }

    /// Set the database pool for content engine loading.
    pub fn set_db_pool(&mut self, pool: Arc<PgPool>) {
        self.db_pool = Some(pool);
    }

    /// Wire in the Base<->Cell channels. Called by the orchestrator before `start()`.
    pub fn set_channels(
        &mut self,
        rx: mpsc::Receiver<BaseToCellMsg>,
        tx: mpsc::Sender<CellToBaseMsg>,
    ) {
        self.base_to_cell_rx = Some(rx);
        self.cell_to_base_tx = Some(tx);
    }

    /// Get a clone of the CellToBase sender (for minigame result routing).
    pub fn cell_to_base_tx(&self) -> Option<mpsc::Sender<CellToBaseMsg>> {
        self.cell_to_base_tx.clone()
    }

    /// Stop the cell service gracefully.
    ///
    /// Signals the cell loop to break out, awaits its join handle, then drops
    /// the channels. Without the await, the task could outlive `stop()` and
    /// keep poking at shared state during teardown.
    pub async fn stop(&mut self) {
        tracing::info!("Stopping cell service");
        if let Some(signal) = self.shutdown_signal.take() {
            // notify_one() stores a permit if no waiter is currently parked,
            // so the next `shutdown.notified().await` in the loop returns
            // immediately. notify_waiters() would only wake an already-parked
            // future and could be lost between loop iterations.
            signal.notify_one();
        }
        if let Some(handle) = self.cell_loop_handle.take() {
            match handle.await {
                Ok(()) => tracing::trace!("Cell loop joined cleanly"),
                Err(e) if e.is_cancelled() => tracing::trace!("Cell loop was cancelled"),
                Err(e) => tracing::warn!(error = %e, "Cell loop task panicked"),
            }
        }
        self.base_to_cell_rx = None;
        self.cell_to_base_tx = None;
        self.is_running = false;
        tracing::trace!("Cell service stopped");
    }
}
