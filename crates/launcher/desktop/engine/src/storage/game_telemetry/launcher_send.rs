//! The send path of `LauncherSender`: a send posts its events in chunks and
//! reports how many it posted and where a retry may resume (in memory only).
use super::{
    locked, LauncherSender, LauncherTarget, SendOutcome, LAUNCHER_MAX_ROWS, MAX_HOLD,
    MIN_RETRY_HOLD,
};
use cimmeria_launcher_core::telemetry::chunk::{self, ChunkError};
use cimmeria_launcher_core::telemetry::events::TelemetryEvent;
use std::time::Duration;

/// What one send did, per event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendReport {
    pub outcome: SendOutcome,
    /// Events accepted by the server (the sum of the posted chunk ranges).
    pub posted: usize,
    /// `Some(i)`: `events[i..]` were not posted and may be sent again.
    /// `None`: whatever was not posted is dropped.
    pub retry_from: Option<usize>,
}

impl LauncherSender {
    /// Send `events` now, in chunks, and report what was posted. Never panics,
    /// never writes to disk, never logs a field value, never logs at target
    /// `adoption`.
    pub async fn send_report(
        &self,
        target: &LauncherTarget,
        events: &[TelemetryEvent],
    ) -> SendReport {
        if events.is_empty() {
            return SendReport {
                outcome: SendOutcome::Empty,
                posted: 0,
                retry_from: None,
            };
        }
        if self.held() {
            tracing::debug!(
                target: "launcher.pipeline",
                reason = "held",
                "launcher telemetry batch not sent"
            );
            return SendReport {
                outcome: SendOutcome::Held,
                posted: 0,
                retry_from: Some(0),
            };
        }
        let mut live = match self.usable_session(target).await {
            Ok(live) => live,
            Err(reason) => {
                tracing::debug!(
                    target: "launcher.pipeline",
                    reason,
                    "launcher telemetry batch not sent"
                );
                return SendReport {
                    outcome: SendOutcome::MintFailed { reason },
                    posted: 0,
                    retry_from: Some(0),
                };
            }
        };
        let max_bytes = live.session.marker().telemetry.chunk_max_bytes;
        let Ok(batches) = chunk::split_batches(events, max_bytes, LAUNCHER_MAX_ROWS) else {
            tracing::debug!(
                target: "launcher.pipeline",
                reason = "dropped",
                "launcher telemetry batch not sent"
            );
            return SendReport {
                outcome: SendOutcome::Dropped,
                posted: 0,
                retry_from: None,
            };
        };
        let mut posted = 0;
        for range in batches {
            let (start, count) = (range.start, range.len());
            let batch = &events[range];
            let mut result = self.post(&live, batch).await;
            if matches!(result, Err(ChunkError::TokenRejected)) {
                // A 401 is the only error that mints again, and only once.
                *locked(&self.session) = None;
                let Ok(fresh) = self.mint(target).await else {
                    tracing::debug!(
                        target: "launcher.pipeline",
                        reason = "dropped",
                        "launcher telemetry batch not sent"
                    );
                    return SendReport {
                        outcome: SendOutcome::Dropped,
                        posted,
                        retry_from: Some(start),
                    };
                };
                live = fresh;
                result = self.post(&live, batch).await;
            }
            match result {
                Ok(()) => posted += count,
                Err(err @ (ChunkError::KillSwitch { .. } | ChunkError::RateLimited { .. })) => {
                    let asked = err.retry_after_secs().unwrap_or(60);
                    self.hold(Duration::from_secs(asked).clamp(MIN_RETRY_HOLD, MAX_HOLD));
                    return dropped_from(start, posted);
                }
                Err(ChunkError::Http(_) | ChunkError::Status { .. }) => {
                    // D-LTL5: a transport or 5xx failure keeps the events and
                    // takes the floor hold, so an offline launcher does not retry at once.
                    self.hold(MIN_RETRY_HOLD);
                    return dropped_from(start, posted);
                }
                // A rejection, a JSON or IO error, or a second 401: the events are dropped.
                Err(_) => {
                    tracing::debug!(
                        target: "launcher.pipeline",
                        reason = "dropped",
                        "launcher telemetry batch not sent"
                    );
                    return SendReport {
                        outcome: SendOutcome::Dropped,
                        posted,
                        retry_from: None,
                    };
                }
            }
        }
        SendReport {
            outcome: SendOutcome::Sent,
            posted,
            retry_from: None,
        }
    }
}

/// The report of a chunk that asks for a retry: its events and the rest are kept.
fn dropped_from(start: usize, posted: usize) -> SendReport {
    tracing::debug!(
        target: "launcher.pipeline",
        reason = "dropped",
        "launcher telemetry batch not sent"
    );
    SendReport {
        outcome: SendOutcome::Dropped,
        posted,
        retry_from: Some(start),
    }
}
