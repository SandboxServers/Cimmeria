//! Sender-task wiring: spawn the task, run the receive loop, retry sends.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use arc_swap::ArcSwap;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::embed::build_embed_body;
use crate::event::{ChannelKind, Event};
use crate::router::channel_for;

use super::handle::{DiscordSender, SenderHandle, SendError};
use super::stats::Stats;
use super::token_bucket::TokenBucket;
use super::{MAX_RETRIES, QUEUE_CAPACITY, RETRY_BASE};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChannelConfig, EventToggles};
    use crate::event::ChannelKind;
    use crate::sender::MockSender;
    use chrono::Utc;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

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
