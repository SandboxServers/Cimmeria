//! Log capture for negative-logging regression guards.
//!
//! Regression guards for log-only changes need to assert that a specific
//! WARN/ERROR event fired with the right structured fields. Without
//! capture, reverting a `trace!` → `warn!` change goes undetected.
//! `LogCapture` is a tracing `Layer` that records each event's level,
//! target, message body, and field map into a shared `Vec<Captured>`.
//!
//! Usage:
//!
//! ```ignore
//! let capture = LogCapture::install();
//! some_function_that_logs().await;
//! assert!(capture.find_event(tracing::Level::WARN, "AoI", "entity_to_addr_miss").is_some());
//! ```
//!
//! # Threading model — IMPORTANT
//!
//! `LogCapture::install` calls `tracing::subscriber::set_default`, which
//! installs the subscriber as the **current thread's** default. Events
//! emitted on other threads are **NOT** captured.
//!
//! In practice: do NOT use `#[tokio::test(flavor = "multi_thread")]`
//! with `LogCapture`. The default `#[tokio::test]` flavor is
//! `current_thread`, which keeps every awaited future on the test's
//! thread — that's what the existing guards rely on. `install()`
//! asserts this at runtime: if a tokio multi-thread runtime is
//! active when you call `install()`, it panics with a hint, so the
//! footgun fails loudly instead of silently dropping events.

use std::collections::HashMap as StdHashMap;
use std::sync::{Arc, Mutex};
use tracing::{
    field::{Field, Visit},
    span::{Attributes, Id, Record},
    Event, Level, Subscriber,
};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::Registry;

/// One captured event.
#[derive(Debug, Clone)]
pub struct Captured {
    pub level: Level,
    pub target: String,
    pub message: Option<String>,
    pub fields: StdHashMap<String, String>,
}

impl Captured {
    /// `true` if `self.message` contains the given substring or
    /// `self.fields["message"]` does. tracing stores message bodies on
    /// the `message` field of the event.
    pub fn message_contains(&self, needle: &str) -> bool {
        self.message.as_deref().is_some_and(|m| m.contains(needle))
            || self
                .fields
                .get("message")
                .is_some_and(|m| m.contains(needle))
    }

    /// `true` if the field map contains a key with the given value
    /// (string-compared — fields are formatted via `Debug`).
    pub fn has_field(&self, key: &str, value: &str) -> bool {
        self.fields.get(key).is_some_and(|v| v == value)
    }
}

/// Tracing `Layer` that records every event into a shared `Vec`. Install
/// via [`LogCapture::install`] inside a test scope.
pub struct LogCapture {
    events: Arc<Mutex<Vec<Captured>>>,
}

impl LogCapture {
    /// Build a subscriber with this layer installed, set it as the
    /// current thread's default via [`tracing::subscriber::set_default`],
    /// and return a guard that captures into the returned `LogCapture`.
    ///
    /// Holds the default-guard for the lifetime of the returned
    /// [`LogCaptureGuard`]; drop the guard to restore the previous
    /// subscriber.
    ///
    /// # Panics
    ///
    /// Panics if called from inside a tokio multi-thread runtime —
    /// `set_default` is thread-local, so events emitted on worker
    /// threads would be silently dropped. Use the default
    /// `#[tokio::test]` (current-thread) flavor.
    pub fn install() -> LogCaptureGuard {
        // Multi-thread runtime detection. `Handle::try_current()` only
        // succeeds inside a runtime; we then ask the flavor and bail
        // if MultiThread. Outside any runtime (sync tests), the check
        // is a no-op.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread {
                panic!(
                    "LogCapture::install called inside a multi-thread tokio runtime. \
                     LogCapture uses thread-local set_default, so events from worker \
                     threads are NOT captured. Switch the test to the default \
                     `#[tokio::test]` (current_thread) flavor."
                );
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let layer = CaptureLayer {
            events: events.clone(),
        };
        let subscriber = Registry::default().with(layer);
        let default_guard = tracing::subscriber::set_default(subscriber);
        LogCaptureGuard {
            capture: LogCapture { events },
            _default_guard: default_guard,
        }
    }
}

/// RAII guard returned by [`LogCapture::install`]. Drop to restore the
/// previous tracing subscriber. Dereferences to [`LogCapture`] so
/// `guard.find_event(...)` works.
pub struct LogCaptureGuard {
    capture: LogCapture,
    _default_guard: tracing::subscriber::DefaultGuard,
}

impl LogCaptureGuard {
    /// First event whose level matches and whose message contains
    /// `message_substr` AND whose fields include the `reason` field set
    /// to `reason_value`. Returns `None` if no match.
    ///
    /// Use the `reason` field convention (per CLAUDE.md and
    /// `docs/architecture/negative-logging-convention.md`) to pin
    /// down WHICH negative-log this is, not just any warn at the
    /// same target.
    ///
    /// # Field stability
    ///
    /// `reason_value` is matched by **exact string equality**. Treat
    /// `reason` field values as stable API: renaming a value (even a
    /// typo fix) will trip every guard pinned to the old string.
    /// Coordinate via the convention doc when adding a new `reason`
    /// or renaming an existing one. For substring matching, see
    /// [`Self::find_message`] (no `reason` filter).
    pub fn find_event(
        &self,
        level: Level,
        message_substr: &str,
        reason_value: &str,
    ) -> Option<Captured> {
        self.capture
            .events
            .lock()
            .unwrap()
            .iter()
            .find(|c| {
                c.level == level
                    && c.message_contains(message_substr)
                    && c.has_field("reason", reason_value)
            })
            .cloned()
    }

    /// First event at `level` whose message contains `message_substr`.
    /// Use when the new log doesn't carry a `reason` field.
    pub fn find_message(&self, level: Level, message_substr: &str) -> Option<Captured> {
        self.capture
            .events
            .lock()
            .unwrap()
            .iter()
            .find(|c| c.level == level && c.message_contains(message_substr))
            .cloned()
    }

    /// All captured events. Useful for debugging when an expected event
    /// doesn't fire — `eprintln!("{:#?}", guard.all())` shows what did.
    pub fn all(&self) -> Vec<Captured> {
        self.capture.events.lock().unwrap().clone()
    }

    /// Look for a span field set to `value` via `Span::current().record(...)`
    /// or the instrument-macro placeholder mechanism. Returns true if
    /// any captured `on_record` call (target prefix `span_record:`)
    /// carries `field_name = value`.
    ///
    /// Used to regression-guard the `decision_outcome` vocab the NPC
    /// AI handlers fill into the dispatcher span — see
    /// `docs/architecture/observability.md#npc_ai_decision_outcome-enum`.
    /// Without this, deleting a `Span::current().record(...)` line
    /// would go unnoticed because there's no event-level emission to
    /// catch.
    pub fn span_recorded(&self, field_name: &str, value: &str) -> bool {
        self.capture
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.target.starts_with("span_record:") && c.has_field(field_name, value))
    }
}

struct CaptureLayer {
    events: Arc<Mutex<Vec<Captured>>>,
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        let metadata = event.metadata();
        self.events.lock().unwrap().push(Captured {
            level: *metadata.level(),
            target: metadata.target().to_string(),
            message: visitor.fields.get("message").cloned(),
            fields: visitor.fields,
        });
    }

    // Span entry: capture the initial attribute set so a span declared
    // with `fields(decision_outcome = tracing::field::Empty)` and later
    // filled by `Span::current().record("decision_outcome", "...")` is
    // queryable via [`LogCaptureGuard::find_span_attribute`].
    fn on_new_span(&self, attrs: &Attributes<'_>, _id: &Id, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        attrs.record(&mut visitor);
        let metadata = attrs.metadata();
        self.events.lock().unwrap().push(Captured {
            level: *metadata.level(),
            target: format!("span:{}", metadata.name()),
            message: None,
            fields: visitor.fields,
        });
    }

    // Span field record (via `Span::current().record(...)` or the
    // `instrument` macro's `fields(...)` placeholders being filled
    // mid-span). Each call appears as a synthetic "record" event tagged
    // with target `span_record:?` — we don't have direct access to the
    // span's name from `_id` without walking the registry, and tests
    // want field+value matching, not span-name matching. The fixed `?`
    // discriminator is what [`LogCaptureGuard::span_recorded`] greps
    // for via `starts_with("span_record:")`.
    fn on_record(&self, _id: &Id, values: &Record<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        values.record(&mut visitor);
        self.events.lock().unwrap().push(Captured {
            level: tracing::Level::TRACE,
            target: "span_record:?".to_string(),
            message: None,
            fields: visitor.fields,
        });
    }
}

#[derive(Default)]
struct FieldVisitor {
    fields: StdHashMap<String, String>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields
            .insert(field.name().to_string(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }
}

#[cfg(test)]
mod tests {
    //! Self-tests for the LogCapture helper. Each assertion pins
    //! behavior that the negative-log regression guards in other
    //! modules depend on, so a refactor to LogCapture that
    //! accidentally breaks one of these contracts trips here first.
    use super::{Captured, LogCapture};
    use tracing::Level;

    /// A bool-only event hits the `record_bool` visitor path. The
    /// other negative-log guards mostly emit Debug-formatted values
    /// (covered by `record_debug` via `?expr` / `%expr`), so this
    /// test exists to keep the typed-record fast paths exercised.
    #[test]
    fn captures_bool_field_via_record_bool() {
        let capture = LogCapture::install();
        tracing::warn!(succeeded = true, "bool test");
        let event = capture
            .find_message(Level::WARN, "bool test")
            .expect("warn must be captured");
        assert!(
            event.has_field("succeeded", "true"),
            "record_bool must store the boolean as 'true'/'false': {event:#?}"
        );
    }

    /// `record_i64` is the visitor path for `i32` / `i64` fields used
    /// without `?` formatting. Most existing guards rely on it
    /// transparently — pin it.
    #[test]
    fn captures_signed_int_field_via_record_i64() {
        let capture = LogCapture::install();
        tracing::warn!(rows_affected = -1i64, "signed int test");
        let event = capture
            .find_message(Level::WARN, "signed int test")
            .expect("warn must be captured");
        assert!(
            event.has_field("rows_affected", "-1"),
            "record_i64 must format as decimal: {event:#?}"
        );
    }

    /// `record_u64` is the visitor path for `u32` / `u64` fields
    /// (entity_id, witness_id, seq, etc.). The existing guards all
    /// exercise this, but pin it explicitly so a regression that
    /// switches storage to a different format trips loudly.
    #[test]
    fn captures_unsigned_int_field_via_record_u64() {
        let capture = LogCapture::install();
        tracing::warn!(entity_id = 4242u32, "unsigned int test");
        let event = capture
            .find_message(Level::WARN, "unsigned int test")
            .expect("warn must be captured");
        assert!(
            event.has_field("entity_id", "4242"),
            "record_u64 must format as decimal: {event:#?}"
        );
    }

    /// `record_str` is the visitor path for `&str` fields. The
    /// `reason` and `phase` convention fields go through here when
    /// emitted as bare strings — `reason = "entity_to_addr_miss"`.
    /// `find_event` exact-matches on this, so a stored value with
    /// wrapping quotes or whitespace would silently break every
    /// guard.
    #[test]
    fn captures_string_field_unquoted_via_record_str() {
        let capture = LogCapture::install();
        tracing::warn!(reason = "entity_to_addr_miss", "str test");
        let event = capture
            .find_message(Level::WARN, "str test")
            .expect("warn must be captured");
        assert!(
            event.has_field("reason", "entity_to_addr_miss"),
            "record_str must store the bare string with no quotes: {event:#?}"
        );
    }

    /// `find_event` returns None on level mismatch even when the
    /// message + reason both match — exact level discipline.
    #[test]
    fn find_event_returns_none_on_level_mismatch() {
        let capture = LogCapture::install();
        tracing::debug!(reason = "test_reason", "level test");
        assert!(
            capture
                .find_event(Level::WARN, "level test", "test_reason")
                .is_none(),
            "DEBUG event must not match a WARN find_event query"
        );
    }

    /// `find_event` returns None on reason mismatch even when the
    /// level + message match. Pins the exact-match contract documented
    /// in the find_event doc.
    #[test]
    fn find_event_returns_none_on_reason_mismatch() {
        let capture = LogCapture::install();
        tracing::warn!(reason = "actual_reason", "reason test");
        assert!(
            capture
                .find_event(Level::WARN, "reason test", "different_reason")
                .is_none(),
            "exact-match on `reason` must reject a near-miss"
        );
    }

    /// `find_message` returns None when no message matches at the
    /// requested level. Covers the "no event" branch.
    #[test]
    fn find_message_returns_none_when_no_match() {
        let capture = LogCapture::install();
        // Don't emit anything.
        assert!(capture
            .find_message(Level::WARN, "nothing was logged")
            .is_none());
    }

    /// `all()` returns every captured event in insertion order. Used
    /// by the existing guards as the `Captured: {:#?}` debug payload
    /// in their assertion messages — when a guard fails, all() shows
    /// what DID fire. Pin order so the debug output is deterministic.
    #[test]
    fn all_returns_events_in_emission_order() {
        let capture = LogCapture::install();
        tracing::info!("first");
        tracing::warn!("second");
        tracing::error!("third");

        let events: Vec<Captured> = capture.all();
        assert_eq!(events.len(), 3, "must capture every emission");
        // Match the levels in order — verifies insertion-order
        // preservation.
        assert_eq!(events[0].level, Level::INFO);
        assert_eq!(events[1].level, Level::WARN);
        assert_eq!(events[2].level, Level::ERROR);
    }

    /// `Captured::has_field` returns false when the key is absent.
    /// The find_event chain short-circuits on this; the false-branch
    /// is otherwise un-exercised because the guards only assert on
    /// the true-branch.
    #[test]
    fn has_field_returns_false_when_key_absent() {
        let capture = LogCapture::install();
        tracing::warn!(only_field = "x", "absent test");
        let event = capture.find_message(Level::WARN, "absent test").unwrap();
        assert!(!event.has_field("not_there", "anything"));
    }

    /// `Captured::message_contains` falls back to `fields["message"]`
    /// when the dedicated `message` accessor is None. tracing stores
    /// the body on either the `Captured::message` field or in the
    /// fields map depending on construction; the helper must accept
    /// both shapes so `find_message`/`find_event` are consistent.
    #[test]
    fn message_contains_finds_substring_via_fields_fallback() {
        // Build a Captured directly with the body ONLY in fields[].
        let mut fields = std::collections::HashMap::new();
        fields.insert("message".to_string(), "fallback body here".to_string());
        let c = Captured {
            level: Level::WARN,
            target: "test".to_string(),
            message: None, // not on the dedicated field
            fields,
        };
        assert!(
            c.message_contains("fallback body"),
            "message_contains must consult fields['message'] when the \
             dedicated `message` accessor is None"
        );
        assert!(
            !c.message_contains("absent substring"),
            "must reject non-matching substring"
        );
    }

    /// Multi-thread runtime detection: install() panics when called
    /// inside a `flavor = "multi_thread"` tokio runtime. The panic
    /// message names the fix. This guards against accidentally
    /// removing the runtime check (which would re-introduce the
    /// silent event-drop on worker threads).
    #[test]
    #[should_panic(expected = "multi-thread tokio runtime")]
    fn install_panics_inside_multi_thread_tokio_runtime() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("multi_thread runtime build");
        // The panic must originate from inside install(), called via
        // the runtime so Handle::try_current() succeeds.
        rt.block_on(async {
            let _ = LogCapture::install();
        });
    }
}
