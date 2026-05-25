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

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use serde_json::Value;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::embed::build_embed_body;
use crate::event::{ChannelKind, Event};
use crate::router::channel_for;

/// Internal queue depth. Sized to absorb a sub-second burst (e.g., a 28-NPC
/// AoI cascade producing one Discord-bound event per NPC) without dropping
/// while a slow Discord round-trip is in flight. Chosen empirically; tune
/// if drop counters report sustained pressure.
const QUEUE_CAPACITY: usize = 256;

/// Maximum HTTP retries on 5xx / network errors before giving up. 429s
/// are handled separately (always honoured via `Retry-After`).
const MAX_RETRIES: u32 = 2;

/// Base backoff between retry attempts. Doubles each retry: 250 ms, 500 ms.
const RETRY_BASE: Duration = Duration::from_millis(250);

/// Per-request timeout for the entire Discord POST round-trip.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

// ── Trait + handle ─────────────────────────────────────────────────────

/// Send-side abstraction. Production uses [`HttpDiscordSender`]; tests
/// use [`MockSender`]. The handle holds an `mpsc::Sender` plus the drop
/// counter — cheap to clone, cheap to `try_send`.
#[derive(Clone)]
pub struct SenderHandle {
    tx: mpsc::Sender<Event>,
    stats: Arc<Stats>,
    config: Arc<ArcSwap<Config>>,
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
    pub fn stats(&self) -> SenderStats {
        self.stats.snapshot()
    }
}

/// Trait abstracting the actual HTTP POST. Production wires
/// [`HttpDiscordSender`]; tests wire [`MockSender`].
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

// ── Stats ──────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct Stats {
    enqueued: AtomicU64,
    sent: AtomicU64,
    filtered: AtomicU64,
    dropped_full: AtomicU64,
    dropped_closed: AtomicU64,
    dropped_rate_limit: AtomicU64,
    retried: AtomicU64,
    rate_limited_429: AtomicU64,
    failed: AtomicU64,
}

impl Stats {
    fn snapshot(&self) -> SenderStats {
        SenderStats {
            enqueued: self.enqueued.load(Ordering::Relaxed),
            sent: self.sent.load(Ordering::Relaxed),
            filtered: self.filtered.load(Ordering::Relaxed),
            dropped_full: self.dropped_full.load(Ordering::Relaxed),
            dropped_closed: self.dropped_closed.load(Ordering::Relaxed),
            dropped_rate_limit: self.dropped_rate_limit.load(Ordering::Relaxed),
            retried: self.retried.load(Ordering::Relaxed),
            rate_limited_429: self.rate_limited_429.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of sender stats. Read via `DiscordRuntime::stats()`; intended
/// to be surfaced by an admin-api stats endpoint when one is wired up.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SenderStats {
    pub enqueued: u64,
    pub sent: u64,
    pub filtered: u64,
    pub dropped_full: u64,
    pub dropped_closed: u64,
    pub dropped_rate_limit: u64,
    pub retried: u64,
    pub rate_limited_429: u64,
    pub failed: u64,
}

// ── Token bucket (per-channel rate limiter) ─────────────────────────────

/// Simple token bucket. `capacity` tokens refill at `refill_per_sec`.
/// `try_acquire` returns `true` if a token is available, `false` if not.
struct TokenBucket {
    /// Rate this bucket was constructed for (per minute). Stored so the
    /// sender task can detect a live-reload rate change and rebuild
    /// without losing accumulated tokens needlessly.
    rate_per_min: u32,
    capacity: f64,
    tokens: f64,
    refill_per_sec: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(rate_per_min: u32) -> Self {
        let per_sec = rate_per_min as f64 / 60.0;
        // Burst capacity = 1 minute's worth (or 5 — whichever is smaller —
        // matches Discord's 5-msg burst budget).
        let capacity = rate_per_min.min(5) as f64;
        Self {
            rate_per_min,
            capacity,
            tokens: capacity,
            refill_per_sec: per_sec,
            last_refill: Instant::now(),
        }
    }

    fn try_acquire(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
        self.last_refill = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

// ── Wiring: spawn the sender task ───────────────────────────────────────

/// Spawn the sender task and return a handle. The returned `JoinHandle`
/// is normally just left to run; awaiting it produces `()` on graceful
/// shutdown (channel closed by dropping all `SenderHandle`s).
pub fn spawn<S: DiscordSender>(
    sender: S,
    config: Arc<ArcSwap<Config>>,
) -> (SenderHandle, tokio::task::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(QUEUE_CAPACITY);
    let stats = Arc::new(Stats::default());
    let handle = SenderHandle {
        tx,
        stats: stats.clone(),
        config: config.clone(),
    };
    let task = tokio::spawn(run_sender_task(sender, rx, stats, config));
    (handle, task)
}

async fn run_sender_task<S: DiscordSender>(
    sender: S,
    mut rx: mpsc::Receiver<Event>,
    stats: Arc<Stats>,
    config: Arc<ArcSwap<Config>>,
) {
    // Per-channel token buckets. Built lazily when an event for the
    // channel first arrives so reconfiguring the rate-limit takes
    // effect for new channels without restart.
    let buckets: Arc<Mutex<HashMap<ChannelKind, TokenBucket>>> =
        Arc::new(Mutex::new(HashMap::new()));

    while let Some(event) = rx.recv().await {
        let cfg = config.load();
        if !cfg.should_post(event.kind()) {
            // Re-check (the config may have changed between try_send
            // and now). Filtered events don't update the dropped
            // counter — they're not failures.
            stats.filtered.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        let kind = event.kind();
        let channel = channel_for(kind);
        let Some(url) = cfg.webhook_url_for(kind) else {
            stats.dropped_closed.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        let rate = cfg.rate_limit_for(kind).unwrap_or(60);

        // Acquire token. If the bucket is empty, drop with the
        // `dropped_rate_limit` counter — better than blocking the
        // task. Drops still feed the heartbeat.
        //
        // Rate live-reload: rebuild the bucket when the configured
        // rate changes. Replacing the bucket loses accumulated tokens
        // for that channel (the next event has to wait one refill
        // interval), which is the right policy — operators tuning
        // the rate down expect immediate effect, not "wait for the
        // old burst to drain."
        let acquired = {
            let mut map = buckets.lock().await;
            // Rebuild on rate mismatch (live-reload) or first-time entry.
            let stale = map.get(&channel).is_some_and(|b| b.rate_per_min != rate);
            if stale {
                map.insert(channel, TokenBucket::new(rate));
            }
            map.entry(channel)
                .or_insert_with(|| TokenBucket::new(rate))
                .try_acquire()
        };
        if !acquired {
            stats.dropped_rate_limit.fetch_add(1, Ordering::Relaxed);
            tracing::debug!(
                target: "cimmeria_discord",
                channel = channel.as_str(),
                event = kind.as_str(),
                "rate-limit drop"
            );
            continue;
        }

        let body = build_embed_body(&event, cfg.username.as_deref(), cfg.avatar_url.as_deref());
        let url = url.to_string();
        let sender_ref = &sender;
        let stats_ref = &stats;
        match send_with_retries(sender_ref, &url, &body, stats_ref).await {
            Ok(()) => {
                stats.sent.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                stats.failed.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(
                    target: "cimmeria_discord",
                    channel = channel.as_str(),
                    event = kind.as_str(),
                    error = %e,
                    "Discord send failed after retries"
                );
            }
        }
    }
}

/// Loop: try send, retry up to MAX_RETRIES on transient errors.
async fn send_with_retries<S: DiscordSender>(
    sender: &S,
    url: &str,
    body: &Value,
    stats: &Stats,
) -> Result<(), SendError> {
    let mut attempt = 0u32;
    loop {
        match sender.send(url, body).await {
            Ok(()) => return Ok(()),
            Err(SendError::HttpStatus(429, _)) => {
                stats.rate_limited_429.fetch_add(1, Ordering::Relaxed);
                // The HttpDiscordSender already honoured Retry-After
                // internally before returning. If we're here, it's after
                // the wait — treat like a retry to try once more.
                if attempt >= MAX_RETRIES {
                    return Err(SendError::RetriesExhausted { retries: attempt });
                }
                attempt += 1;
                stats.retried.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            Err(SendError::HttpStatus(code, msg)) if code < 500 => {
                // 4xx (config bug, e.g. unknown webhook) — don't retry.
                return Err(SendError::HttpStatus(code, msg));
            }
            Err(e) => {
                if attempt >= MAX_RETRIES {
                    return Err(e);
                }
                attempt += 1;
                stats.retried.fetch_add(1, Ordering::Relaxed);
                tokio::time::sleep(RETRY_BASE * (1u32 << (attempt - 1))).await;
            }
        }
    }
}

// ── HTTP implementation ─────────────────────────────────────────────────

/// Production HTTP sender. Owns a `reqwest::Client` configured with a
/// 10 s per-request timeout. Honours Discord's `Retry-After` 429 header
/// by sleeping in-task before returning the 429.
pub struct HttpDiscordSender {
    client: reqwest::Client,
}

impl HttpDiscordSender {
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self { client })
    }
}

impl Default for HttpDiscordSender {
    fn default() -> Self {
        Self::new().expect("reqwest client construction cannot fail with default config")
    }
}

#[async_trait::async_trait]
impl DiscordSender for HttpDiscordSender {
    async fn send(&self, url: &str, body: &Value) -> Result<(), SendError> {
        let resp = self
            .client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| SendError::Network(e.to_string()))?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        // 429: honour Retry-After in-task. We sleep here so the sender
        // loop above sees it as a single retryable error rather than
        // racing other channels' buckets.
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = parse_retry_after(&resp).unwrap_or(Duration::from_secs(1));
            tracing::debug!(
                target: "cimmeria_discord",
                wait_ms = retry_after.as_millis() as u64,
                "Discord 429 — honouring Retry-After"
            );
            tokio::time::sleep(retry_after).await;
            return Err(SendError::HttpStatus(429, "rate limited".to_string()));
        }
        let body_text = resp
            .text()
            .await
            .unwrap_or_else(|_| "(no body)".to_string());
        Err(SendError::HttpStatus(status.as_u16(), body_text))
    }
}

fn parse_retry_after(resp: &reqwest::Response) -> Option<Duration> {
    let header = resp.headers().get("Retry-After")?;
    let s = header.to_str().ok()?;
    // Discord sends fractional seconds (e.g., "0.345"). Try float
    // first, then integer.
    if let Ok(secs) = s.parse::<f64>() {
        let cap = secs.clamp(0.0, 60.0);
        return Some(Duration::from_secs_f64(cap));
    }
    if let Ok(secs) = s.parse::<u64>() {
        return Some(Duration::from_secs(secs.min(60)));
    }
    None
}

// ── Mock for tests ──────────────────────────────────────────────────────

/// In-memory recorder used by tests. Records every `(url, body)` pair
/// and lets the test assert ordering, count, or content. `next_error`
/// lets a test simulate a failure response on the next call.
#[derive(Default)]
pub struct MockSender {
    calls: Arc<Mutex<Vec<(String, Value)>>>,
    next_error: Arc<Mutex<Option<SendError>>>,
}

impl MockSender {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn calls(&self) -> Vec<(String, Value)> {
        self.calls.lock().await.clone()
    }

    pub async fn call_count(&self) -> usize {
        self.calls.lock().await.len()
    }

    pub async fn set_next_error(&self, e: SendError) {
        *self.next_error.lock().await = Some(e);
    }

    /// Cheap handle to the underlying call log, for assertion helpers
    /// in cross-module tests (layer.rs, integration tests, …).
    pub fn calls_handle(&self) -> Arc<Mutex<Vec<(String, Value)>>> {
        self.calls.clone()
    }
}

#[async_trait::async_trait]
impl DiscordSender for MockSender {
    async fn send(&self, url: &str, body: &Value) -> Result<(), SendError> {
        if let Some(e) = self.next_error.lock().await.take() {
            return Err(e);
        }
        self.calls
            .lock()
            .await
            .push((url.to_string(), body.clone()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChannelConfig, EventToggles};
    use crate::event::ChannelKind;
    use chrono::Utc;
    use std::collections::HashMap;

    fn test_config(toggles: EventToggles) -> Arc<ArcSwap<Config>> {
        let mut channels = HashMap::new();
        for c in ChannelKind::ALL {
            channels.insert(
                *c,
                ChannelConfig {
                    url: format!("https://discord.com/api/webhooks/1/{}", c.as_str()),
                    rate_limit_per_min: 60,
                },
            );
        }
        let cfg = Config {
            enabled: true,
            username: None,
            avatar_url: None,
            channels,
            events: toggles,
        };
        Arc::new(ArcSwap::new(Arc::new(cfg)))
    }

    fn login_event() -> Event {
        Event::PlayerLogin {
            account_id: 1,
            character_name: Some("alice".into()),
            addr: "127.0.0.1:50000".parse().unwrap(),
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn enabled_event_is_delivered_to_mock_once() {
        let mock = MockSender::new();
        let calls = mock.calls.clone();
        let cfg = test_config(EventToggles::default());
        let (handle, _task) = spawn(mock, cfg);

        handle.try_send(login_event()).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(calls.lock().await.len(), 1, "exactly one send");
        let stats = handle.stats();
        assert_eq!(stats.enqueued, 1);
        assert_eq!(stats.sent, 1);
        assert_eq!(stats.dropped_full, 0);
    }

    #[tokio::test]
    async fn disabled_event_is_filtered_pre_queue() {
        let toggles = EventToggles {
            player_login: false,
            ..EventToggles::default()
        };
        let mock = MockSender::new();
        let calls = mock.calls.clone();
        let cfg = test_config(toggles);
        let (handle, _task) = spawn(mock, cfg);

        handle.try_send(login_event()).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(calls.lock().await.len(), 0);
        assert_eq!(handle.stats().filtered, 1);
        assert_eq!(handle.stats().enqueued, 0);
    }

    #[tokio::test]
    async fn queue_full_drops_with_counter() {
        // Wire a sender that holds the lock forever to back-pressure
        // the queue. We saturate from a single producer.
        struct BlockingSender {
            gate: Arc<tokio::sync::Notify>,
        }
        #[async_trait::async_trait]
        impl DiscordSender for BlockingSender {
            async fn send(&self, _: &str, _: &Value) -> Result<(), SendError> {
                self.gate.notified().await;
                Ok(())
            }
        }
        let gate = Arc::new(tokio::sync::Notify::new());
        let mock = BlockingSender { gate: gate.clone() };
        let cfg = test_config(EventToggles::default());
        let (handle, _task) = spawn(mock, cfg);

        // Fill the queue past capacity.
        let mut sent_ok = 0u64;
        let mut dropped = 0u64;
        for _ in 0..(QUEUE_CAPACITY + 32) {
            match handle.try_send(login_event()) {
                Ok(()) => sent_ok += 1,
                Err(_) => dropped += 1,
            }
        }
        assert!(
            dropped > 0,
            "must drop SOME (queue cap = {})",
            QUEUE_CAPACITY
        );
        let stats = handle.stats();
        assert_eq!(stats.dropped_full, dropped);
        assert_eq!(stats.enqueued, sent_ok);

        gate.notify_waiters();
    }

    #[tokio::test]
    async fn token_bucket_caps_to_burst() {
        let mut bucket = TokenBucket::new(60); // 1/sec, burst 5
                                               // Burst of 5 should all succeed.
        for _ in 0..5 {
            assert!(bucket.try_acquire(), "burst-budget acquires must succeed");
        }
        // 6th immediately fails.
        assert!(!bucket.try_acquire(), "6th acquire must fail before refill");
    }

    #[tokio::test]
    async fn token_bucket_refills_over_time() {
        let mut bucket = TokenBucket::new(600); // 10/sec
        for _ in 0..5 {
            bucket.try_acquire();
        }
        assert!(!bucket.try_acquire());
        tokio::time::sleep(Duration::from_millis(200)).await;
        // 200ms × 10/sec = 2 tokens.
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
    }

    /// 4xx errors don't retry — keeping a broken webhook URL from
    /// chewing through retries.
    #[tokio::test]
    async fn http_4xx_does_not_retry() {
        let mock = MockSender::new();
        mock.set_next_error(SendError::HttpStatus(404, "not found".into()))
            .await;
        let stats = Arc::new(Stats::default());
        let body = serde_json::json!({});
        let err = send_with_retries(&mock, "https://x", &body, &stats)
            .await
            .unwrap_err();
        assert!(matches!(err, SendError::HttpStatus(404, _)));
        assert_eq!(stats.retried.load(Ordering::Relaxed), 0);
    }

    /// Network errors retry up to MAX_RETRIES.
    #[tokio::test(start_paused = true)]
    async fn network_error_retries_up_to_limit() {
        struct AlwaysFails {
            calls: Arc<AtomicU64>,
        }
        #[async_trait::async_trait]
        impl DiscordSender for AlwaysFails {
            async fn send(&self, _: &str, _: &Value) -> Result<(), SendError> {
                self.calls.fetch_add(1, Ordering::Relaxed);
                Err(SendError::Network("simulated".into()))
            }
        }
        let calls = Arc::new(AtomicU64::new(0));
        let sender = AlwaysFails {
            calls: calls.clone(),
        };
        let stats = Arc::new(Stats::default());
        let body = serde_json::json!({});
        let err = send_with_retries(&sender, "https://x", &body, &stats)
            .await
            .unwrap_err();
        assert!(matches!(err, SendError::Network(_)));
        // Initial + MAX_RETRIES retries
        assert_eq!(calls.load(Ordering::Relaxed), (MAX_RETRIES + 1) as u64);
        assert_eq!(stats.retried.load(Ordering::Relaxed), MAX_RETRIES as u64);
    }
}
