//! The [`SenderHandle`], the [`DiscordSender`] trait, and the send-side
//! error types.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use arc_swap::ArcSwap;
use serde_json::Value;
use thiserror::Error;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::event::Event;

use super::stats::Stats;

// ── Trait + handle ─────────────────────────────────────────────────────

/// Send-side abstraction. Production uses [`HttpDiscordSender`](super::HttpDiscordSender);
/// tests use [`MockSender`](super::MockSender). The handle holds an
/// `mpsc::Sender` plus the drop counter — cheap to clone, cheap to
/// `try_send`.
#[derive(Clone)]
pub struct SenderHandle {
    pub(super) tx: mpsc::Sender<Event>,
    pub(super) stats: Arc<Stats>,
    pub(super) config: Arc<ArcSwap<Config>>,
}

impl SenderHandle {
    /// Enqueue an event for posting. Returns immediately. If the queue
    /// is full, increments the drop counter and returns `Err`. Most
    /// callers ignore the return value — the counter + heartbeat is
    /// the reporting surface.
    pub fn try_send(&self, event: Event) -> Result<(), QueueFull> {
        // Cheap pre-filter: skip events that won't post anyway (toggle
        // off, channel unconfigured, Discord disabled). Saves a queue
        // slot for events that WILL post.
        let cfg = self.config.load();
        if !cfg.should_post(event.kind()) {
            self.stats.filtered.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        match self.tx.try_send(event) {
            Ok(()) => {
                self.stats.enqueued.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.stats.dropped_full.fetch_add(1, Ordering::Relaxed);
                Err(QueueFull)
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.stats.dropped_closed.fetch_add(1, Ordering::Relaxed);
                Err(QueueFull)
            }
        }
    }

    /// Stats snapshot for diagnostics endpoints.
    pub fn stats(&self) -> super::SenderStats {
        self.stats.snapshot()
    }
}

/// Trait abstracting the actual HTTP POST. Production wires
/// [`HttpDiscordSender`](super::HttpDiscordSender); tests wire
/// [`MockSender`](super::MockSender).
#[async_trait::async_trait]
pub trait DiscordSender: Send + Sync + 'static {
    /// Post one embed body to the given webhook URL. The implementation
    /// is responsible for retry on 5xx and honouring `Retry-After` on
    /// 429.
    async fn send(&self, url: &str, body: &Value) -> Result<(), SendError>;
}

#[derive(Debug, Error)]
pub enum SendError {
    #[error("non-retryable HTTP {0}: {1}")]
    HttpStatus(u16, String),
    #[error("network error: {0}")]
    Network(String),
    #[error("rate-limit exhausted ({retries} retries)")]
    RetriesExhausted { retries: u32 },
}

#[derive(Debug, Error)]
#[error("Discord queue full or closed; event dropped")]
pub struct QueueFull;
