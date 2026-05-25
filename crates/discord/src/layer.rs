//! `tracing_subscriber::Layer` that harvests `warn!` and `error!` events
//! into [`Event::TracingEvent`] payloads and feeds them through a
//! [`SenderHandle`].
//!
//! Wires the negative-logging convention into Discord without
//! instrumenting every emit site — any `warn!` / `error!` with
//! structured fields (`reason`, `entity_id`, `rows_affected`, etc.)
//! shows up in the errors channel automatically.
//!
//! See `docs/architecture/negative-logging-convention.md` for the field
//! catalog the layer harvests.
//!
//! # Recursion safety
//!
//! The HTTP-send path itself emits diagnostics under
//! `target = "cimmeria_discord"`. If those events fed back into this
//! layer, every Discord post would emit its own Discord post — infinite
//! loop. The layer filters its own target out at `on_event`, before
//! ever constructing an `Event`.

use std::sync::Arc;

use arc_swap::ArcSwap;
use chrono::Utc;
use tracing::field::{Field, Visit};
use tracing::{Event as TracingEvent, Level, Subscriber};
#[cfg(test)]
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::layer::{Context, Layer};

use crate::config::Config;
use crate::event::{Event, TracingEventKind};
use crate::sender::SenderHandle;

/// Tracing layer that converts `warn!`/`error!` into Discord-bound events.
///
/// Construct with a `SenderHandle` (which carries its own config snapshot)
/// and install via `tracing_subscriber::Registry::with(layer)`. Cloning is
/// cheap; the layer holds `Arc`s.
pub struct DiscordLayer {
    sender: SenderHandle,
    /// Held separately so we can re-check toggles at layer time. The
    /// SenderHandle's own copy of this is private; we don't go through
    /// it because the sender's pre-filter doesn't return enough info to
    /// short-circuit *before* visitor traversal. Cheap to hold a second
    /// `Arc<ArcSwap<_>>`.
    config: Arc<ArcSwap<Config>>,
}

impl DiscordLayer {
    pub fn new(sender: SenderHandle, config: Arc<ArcSwap<Config>>) -> Self {
        Self { sender, config }
    }
}

impl<S: Subscriber> Layer<S> for DiscordLayer {
    fn on_event(&self, event: &TracingEvent<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = *metadata.level();

        // Map level → TracingEventKind. Only warn and error feed the
        // layer; debug/trace/info are too noisy + the use case is
        // ops visibility, not log mirror.
        let kind = match level {
            Level::WARN => TracingEventKind::Warn,
            Level::ERROR => TracingEventKind::Error,
            _ => return,
        };

        // Recursion guard: skip events emitted by Discord-internal
        // hot paths. The sender task and config-reload task both emit
        // diagnostics under the explicit `cimmeria_discord` target;
        // looping those back into the layer would post one Discord
        // message per Discord send failure indefinitely.
        //
        // Filter ONLY that explicit target — NOT any submodule path
        // like `cimmeria_discord::layer::tests`. The auto-derived
        // module-path targets are fine to harvest; the explicit
        // `target: "cimmeria_discord"` annotation is the one to drop.
        if metadata.target() == "cimmeria_discord" {
            return;
        }

        // Pre-filter: skip if Discord is off or the kind is toggled off
        // or the routed channel isn't configured. Same gate the
        // SenderHandle applies, hoisted up so we don't run the visitor
        // on filtered events.
        let cfg = self.config.load();
        if !cfg.should_post(kind.event_kind()) {
            return;
        }

        // Visit the event's fields. tracing's API gives us a `&dyn
        // Visit` callback; we collect into a Vec<(String, String)>.
        let mut visitor = FieldCollector::default();
        event.record(&mut visitor);

        let discord_event = Event::TracingEvent {
            kind,
            target: metadata.target().to_string(),
            message: visitor.message.unwrap_or_default(),
            fields: visitor.fields,
            timestamp: Utc::now(),
        };

        // try_send drops with a counter on queue full. Don't propagate
        // back into tracing — the recursion guard above already cuts
        // the loop, but failing here silently is the only correct
        // policy regardless.
        let _ = self.sender.try_send(discord_event);
    }
}

#[derive(Default)]
struct FieldCollector {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl Visit for FieldCollector {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let name = field.name();
        let s = format!("{:?}", value);
        // Strip outer quotes that Debug-formatting adds for `&str`.
        let s = s
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(&s)
            .to_string();
        if name == "message" {
            self.message = Some(s);
        } else {
            self.fields.push((name.to_string(), s));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChannelConfig, EventToggles};
    use crate::event::{ChannelKind, EventKind};
    use crate::sender::{spawn, MockSender};
    use std::collections::HashMap;
    use std::time::Duration;

    fn errors_only_config(warning: bool, error: bool) -> Arc<ArcSwap<Config>> {
        let mut channels = HashMap::new();
        channels.insert(
            ChannelKind::Errors,
            ChannelConfig {
                url: "https://discord.com/api/webhooks/1/errors".into(),
                rate_limit_per_min: 60,
            },
        );
        let events = EventToggles {
            warning,
            error,
            ..EventToggles::default()
        };
        let cfg = Config {
            enabled: true,
            username: None,
            avatar_url: None,
            channels,
            events,
        };
        Arc::new(ArcSwap::new(Arc::new(cfg)))
    }

    /// Bug shape: a `warn!` event from non-Discord code lands in the
    /// errors channel. Pin both directions: warn enabled → fires, warn
    /// disabled → doesn't fire.
    #[tokio::test(flavor = "current_thread")]
    async fn warn_event_routed_when_enabled() {
        let mock = MockSender::new();
        let calls = mock.calls_handle();
        let cfg = errors_only_config(/* warning */ true, /* error */ true);
        let (handle, _task) = spawn(mock, cfg.clone());

        let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::warn!(reason = "test", entity_id = 42, "something happened");
        tokio::time::sleep(Duration::from_millis(50)).await;

        let recorded = calls.lock().await;
        assert_eq!(recorded.len(), 1, "warn must produce one Discord post");
        let (url, body) = &recorded[0];
        assert!(url.ends_with("/errors"), "must route to errors channel");
        let serialized = body.to_string();
        assert!(serialized.contains("something happened"), "message body");
        assert!(serialized.contains("reason"), "field key in fields list");
    }

    /// Same emit but with `warning = false` → no post.
    #[tokio::test(flavor = "current_thread")]
    async fn warn_event_filtered_when_disabled() {
        let mock = MockSender::new();
        let calls = mock.calls_handle();
        let cfg = errors_only_config(/* warning */ false, /* error */ true);
        let (handle, _task) = spawn(mock, cfg.clone());

        let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::warn!("nope");
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(
            calls.lock().await.len(),
            0,
            "warn must not post when disabled"
        );
    }

    /// Recursion guard — events from `cimmeria_discord` must NOT round-
    /// trip back through the layer. A regression here is an infinite
    /// loop on the first send failure.
    #[tokio::test(flavor = "current_thread")]
    async fn discord_self_target_filtered() {
        let mock = MockSender::new();
        let calls = mock.calls_handle();
        let cfg = errors_only_config(true, true);
        let (handle, _task) = spawn(mock, cfg.clone());

        let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::warn!(target: "cimmeria_discord", "should not post");
        tokio::time::sleep(Duration::from_millis(50)).await;

        let recorded: Vec<_> = calls.lock().await.clone();
        assert!(
            recorded.is_empty(),
            "events tagged `target: cimmeria_discord` must be filtered to prevent recursion, got {:?}",
            recorded
        );
    }

    /// info/debug/trace are deliberately ignored — Discord is for warn+
    /// only.
    #[tokio::test(flavor = "current_thread")]
    async fn info_debug_trace_ignored() {
        let mock = MockSender::new();
        let calls = mock.calls_handle();
        let cfg = errors_only_config(true, true);
        let (handle, _task) = spawn(mock, cfg.clone());

        let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::info!("ignored");
        tracing::debug!("ignored");
        tracing::trace!("ignored");
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(calls.lock().await.len(), 0);
    }

    /// Error events carry the right EventKind and route to the same
    /// channel as warns (both go to `errors`).
    #[tokio::test(flavor = "current_thread")]
    async fn error_event_routed_to_errors_channel() {
        let mock = MockSender::new();
        let calls = mock.calls_handle();
        let cfg = errors_only_config(true, true);
        let (handle, _task) = spawn(mock, cfg.clone());

        let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::error!(player_id = 100, "DB failure");
        tokio::time::sleep(Duration::from_millis(50)).await;

        let recorded = calls.lock().await;
        assert_eq!(recorded.len(), 1);
        assert!(recorded[0].0.ends_with("/errors"));
        // The EventKind discriminator is implicit via routing; pin
        // that we're using the right variant by spot-checking the body
        // mentions the player_id field.
        assert!(recorded[0].1.to_string().contains("player_id"));
    }

    /// Fields are recorded in stable order — `reason` and `entity_id`
    /// from the negative-logging convention need to show up somewhere
    /// predictable in the embed.
    #[tokio::test(flavor = "current_thread")]
    async fn structured_fields_preserved_in_embed() {
        let mock = MockSender::new();
        let calls = mock.calls_handle();
        let cfg = errors_only_config(true, true);
        let (handle, _task) = spawn(mock, cfg.clone());

        let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::error!(
            reason = "entity_to_addr_miss",
            entity_id = 42u32,
            entity_count_in_map = 17u64,
            "AoI: no client addr -- skipping"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;

        let recorded = calls.lock().await;
        let body = &recorded[0].1;
        let s = body.to_string();
        assert!(s.contains("entity_to_addr_miss"));
        assert!(s.contains("entity_id"));
        assert!(s.contains("42"));
    }

    /// Sanity: EventKind for the tracing event matches expectation.
    #[test]
    fn tracing_event_kind_warn_maps_to_warning() {
        assert_eq!(TracingEventKind::Warn.event_kind(), EventKind::Warning);
        assert_eq!(TracingEventKind::Error.event_kind(), EventKind::Error);
    }

    /// Field-visitor coverage for every primitive type the
    /// `FieldCollector` implements. tracing emits structured fields
    /// by calling one of `record_str`, `record_i64`, `record_u64`,
    /// `record_bool`, `record_f64`, or `record_debug` depending on
    /// the value's type. The existing tests only cover string +
    /// debug; this pins the numeric and boolean arms so future
    /// refactors don't quietly drop a primitive type from the
    /// embed's fields list.
    ///
    /// Reverting any `record_*` arm to `unimplemented!()` (or to a
    /// `panic!`) trips this immediately because tracing macros
    /// dispatch to the visitor by value type at compile time.
    #[tokio::test(flavor = "current_thread")]
    async fn field_collector_records_all_primitive_types() {
        let mock = MockSender::new();
        let calls = mock.calls_handle();
        let cfg = errors_only_config(true, true);
        let (handle, _task) = spawn(mock, cfg.clone());

        let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::error!(
            // i64 (record_i64)
            signed_field = -42_i64,
            // u64 (record_u64)
            unsigned_field = 99_u64,
            // bool (record_bool)
            bool_field = true,
            // f64 (record_f64). Not an approx-pi value — clippy
            // would flag that as `approx_constant`.
            float_field = 2.5_f64,
            // &str (record_str)
            string_field = "hello",
            "primitive harvest"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;

        let recorded = calls.lock().await;
        assert_eq!(recorded.len(), 1, "exactly one post recorded");
        let body = recorded[0].1.to_string();

        // Every field name + every formatted value must appear in
        // the body. Each pair pins a distinct `record_*` arm.
        for (key, val) in [
            ("signed_field", "-42"),
            ("unsigned_field", "99"),
            ("bool_field", "true"),
            ("float_field", "2.5"),
            ("string_field", "hello"),
        ] {
            assert!(body.contains(key), "key `{key}` missing in body: {body}");
            assert!(
                body.contains(val),
                "value `{val}` for key `{key}` missing in body: {body}"
            );
        }
    }
}
