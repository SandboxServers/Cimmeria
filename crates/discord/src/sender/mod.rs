//! Send-side pipeline: bounded queue, per-channel token bucket, HTTP POST.
//!
//! # Architecture
//!
//! ```text
//! ┌────────┐  Event  ┌──────────────┐  Embed JSON  ┌──────────────┐
//! │ emit() │ ──────► │ SenderHandle │ ───────────► │ sender task  │
//! └────────┘         │ (mpsc tx)    │              │ (per-channel │
//!                    └──────────────┘              │  token       │
//!                                                  │  buckets +   │
//!                                                  │  reqwest)    │
//!                                                  └──────────────┘
//! ```
//!
//! ## Back-pressure policy: drop on full
//!
//! The bounded `mpsc::channel` has capacity `QUEUE_CAPACITY`. When the
//! sender task can't keep up (Discord slow / network blip / dead-channel
//! 4xx looping), `try_send` returns `Err(Full(_))` and the event is
//! **dropped with an atomic counter increment**. The tick loop never
//! blocks on Discord — the *whole point* of the Discord layer is
//! ops-visibility, and a stalled tick loop trying to deliver an
//! ops-visibility message would be a worse failure mode than the message
//! being dropped.
//!
//! The drop counter is exposed via [`SenderStats`] (read with
//! `DiscordRuntime::stats()`). An admin-api endpoint and periodic
//! heartbeat task that posts the drop count to the lifecycle channel
//! are tracked as follow-ups — wiring lives in the `cimmeria-admin-api`
//! crate, not here.
//!
//! The module is split along these seams:
//!
//! - [`handle`] — the [`SenderHandle`], the [`DiscordSender`] trait, and
//!   the [`SendError`] / [`QueueFull`] error types.
//! - [`stats`] — the atomic [`Stats`](stats::Stats) counters and the
//!   [`SenderStats`] snapshot.
//! - [`token_bucket`] — the per-channel rate limiter.
//! - [`task`] — the [`spawn`] wiring + the sender task loop + retry logic.
//! - [`http`] — the production [`HttpDiscordSender`] + Retry-After parsing.
//! - [`mock`] — the in-memory [`MockSender`] for tests.

mod handle;
mod http;
mod mock;
mod stats;
mod task;
mod token_bucket;

use std::time::Duration;

pub use handle::{DiscordSender, QueueFull, SendError, SenderHandle};
pub use http::HttpDiscordSender;
pub use mock::MockSender;
pub use stats::SenderStats;
pub use task::spawn;

// ── Shared tuning constants ──────────────────────────────────────────────

/// Internal queue depth. Sized to absorb a sub-second burst (e.g., a 28-NPC
/// AoI cascade producing one Discord-bound event per NPC) without dropping
/// while a slow Discord round-trip is in flight. Chosen empirically; tune
/// if drop counters report sustained pressure.
pub(super) const QUEUE_CAPACITY: usize = 256;

/// Maximum HTTP retries on 5xx / network errors before giving up. 429s
/// are handled separately (always honoured via `Retry-After`).
pub(super) const MAX_RETRIES: u32 = 2;

/// Base backoff between retry attempts. Doubles each retry: 250 ms, 500 ms.
pub(super) const RETRY_BASE: Duration = Duration::from_millis(250);

/// Per-request timeout for the entire Discord POST round-trip.
pub(super) const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
