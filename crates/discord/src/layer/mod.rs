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
/// Targets the telemetry ingest re-emits client-side rows under
/// (`crates/admin-api/src/routes/telemetry/replay.rs`).
pub const CLIENT_REPLAY_TARGETS: &[&str] = &[
    "client.native",
    "launcher.client_log",
    "launcher.debug_log",
    "launcher.session_meta",
];

/// `(target, event)` pairs that are content-quality or known-gap telemetry:
/// they stay WARN in the logs and SigNoz and never post to Discord. Other
/// events on the same target still post. Each row says why.
pub const SIGNOZ_ONLY_EVENTS: &[(&str, &str)] = &[
    // A spawn outside `find_path`'s start box is seed-placement data, read
    // from the SigNoz views. It is reported once per spawn id per process,
    // but the colo restarts on every deploy and instanced worlds
    // (Castle_CellBlock, one per login) spawn the same rows again, so it
    // posted the same handful of spawns all day (2026-09-29).
    ("spawner.npc_behaviour", "spawn_off_mesh"),
    // Effect rows naming a script nothing registers: one row per boot for a
    // known, test-pinned seed gap (the scripts crate's live-DB guard), so
    // it posted once per deploy with nothing new to act on.
    ("abilities", "effect_script_unregistered"),
];

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

        // Movement-validation telemetry is warn-only calibration data
        // destined for SigNoz (used to compute the legitimate p99.9 speed
        // before the speed layer is ever promoted to snap-back). It fires
        // during normal play — sub-tick deltas produce huge / infinite
        // implied-speed ratios — so harvesting it would flood the Discord
        // errors channel with non-actionable noise. It still flows to logs
        // and SigNoz; it just never posts to Discord. The `movement.validation`
        // target covers both `speed_warning` and `validation_reject`.
        if metadata.target() == "movement.validation" {
            return;
        }

        // Client telemetry replayed by the upload ingest (the game DLL's
        // events and the launcher's tailed client logs) is the client's
        // own warn/error stream, one row per client event. It belongs in
        // SigNoz, never in the server's errors channel: a lab client
        // running a repro campaign posted hundreds of them (2026-09-29).
        // The server's own ingest records (`launcher.ingest`,
        // `launcher.bundle`) still post.
        if CLIENT_REPLAY_TARGETS.contains(&metadata.target()) {
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

        // Content-quality / known-gap events are SigNoz-only. Matched on the
        // structured `event` field, so the rest of the target still posts.
        if is_signoz_only(metadata.target(), &visitor.fields) {
            return;
        }

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

/// `true` when `(target, fields["event"])` is a [`SIGNOZ_ONLY_EVENTS`] row.
fn is_signoz_only(target: &str, fields: &[(String, String)]) -> bool {
    let Some(event) = fields
        .iter()
        .find_map(|(k, v)| (k == "event").then_some(v.as_str()))
    else {
        return false;
    };
    SIGNOZ_ONLY_EVENTS
        .iter()
        .any(|&(t, e)| t == target && e == event)
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
mod tests;
