//! Shared helpers for tests.
//!
//! Two unrelated test seams live here:
//!
//! - **Live-DB**: tests that need a real PostgreSQL connection call
//!   [`test_pool`] and self-skip when `DATABASE_URL` is unset. The unit-test
//!   suite stays green on a fresh checkout; only `DATABASE_URL=postgres://…
//!   cargo test` exercises the integration path. See
//!   `docs/architecture/integration-test-infra.md` for the rationale,
//!   local-setup steps, and per-test data-isolation patterns.
//!
//! - **Transport fake**: [`TestTransport`] is the canonical UDP fake — a
//!   recording [`cimmeria_mercury::transport::Transport`] impl that handler
//!   unit tests pass as `&Arc<dyn Transport>` in place of a real socket, then
//!   assert byte-exact, addr-correct fan-out on. This is the seam behind the
//!   **fan-out byte test** type in `TESTING.md`. See
//!   `docs/architecture/transport-trait.md`.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// The canonical recording UDP fake — see the module doc-comment and
/// `docs/architecture/transport-trait.md`. Re-exported here so handler unit
/// tests can `use crate::test_support::TestTransport;` without reaching into
/// the mercury crate path.
pub(crate) use cimmeria_mercury::test_transport::TestTransport;

/// Why a live-DB test couldn't run.
///
/// Distinguishes "no DATABASE_URL configured" (expected on a fresh
/// checkout — silent skip) from "DATABASE_URL set but unreachable"
/// (likely misconfiguration — surface the connection error so the
/// developer can fix it).
pub(crate) enum SkipReason {
    /// `DATABASE_URL` env var was unset or empty.
    NotConfigured,
    /// `DATABASE_URL` was set but `connect()` failed. The string
    /// captures sqlx's underlying error for operator triage.
    ConnectFailed(String),
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkipReason::NotConfigured => write!(f, "DATABASE_URL not set"),
            SkipReason::ConnectFailed(e) => write!(f, "DATABASE_URL set but connect failed: {e}"),
        }
    }
}

/// Open a `PgPool` against the developer-supplied `DATABASE_URL`, or
/// return a [`SkipReason`] explaining why no pool was produced.
///
/// Bounded to 4 connections — high enough for tests that exercise
/// concurrent paths (drainer + caller in parallel), low enough that
/// a careless test loop can't exhaust a hand-tuned local Postgres.
pub(crate) async fn test_pool() -> Result<PgPool, SkipReason> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => return Err(SkipReason::NotConfigured),
    };
    PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&url)
        .await
        .map_err(|e| SkipReason::ConnectFailed(e.to_string()))
}

/// Convenience macro: skip a test with a reason-specific message if
/// no DB pool is available. Pairs with [`test_pool`] — same gate,
/// less ceremony at each call site.
///
/// ```ignore
/// #[tokio::test]
/// async fn my_db_test() {
///     let pool = require_db_or_skip!();
///     // ... test body uses pool ...
/// }
/// ```
macro_rules! require_db_or_skip {
    () => {{
        match $crate::test_support::test_pool().await {
            Ok(p) => p,
            Err(reason) => {
                eprintln!("{}: skipping live-DB test ({reason})", module_path!(),);
                return;
            }
        }
    }};
}

pub(crate) use require_db_or_skip;

// ── SpaceManager test fixtures ────────────────────────────────────────

use crate::cell::space_manager::SpaceManager;

/// Standard test space setup: SpaceManager with a single Agnos space.
///
/// The same ~5-line block was duplicated across dispatch, interaction,
/// and vendor test modules. Extracted here so every cell test shares
/// the same default world geometry.
pub(crate) fn make_space_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(spaces_xml).unwrap();
    mgr.create_startup_spaces(cell_spaces_xml).unwrap();
    mgr
}

/// Same as [`make_space_manager`], but also creates a player entity at
/// the origin so tests that need an avatar don't repeat the entity
/// creation boilerplate.
pub(crate) fn make_space_manager_with_player(entity_id: u32) -> SpaceManager {
    let mut mgr = make_space_manager();
    mgr.create_entity(entity_id, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr
}

// ── ConnectedClientState fixture ──────────────────────────────────────
//
// `ConnectedClientState` is built up across the Phase 3 handshake
// (login.rs) and there is no production constructor for "an empty one"
// — every field has to be threaded through. Several unit tests just
// want a structurally-valid placeholder so they can populate the one
// or two fields the test actually cares about. Centralise that here.

use crate::base::ConnectedClientState;
use cimmeria_mercury::encryption::MercuryEncryption;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Build a `ConnectedClientState` with all fields zeroed/None and
/// fresh `Arc`s — a structurally-valid placeholder for tests that
/// only mutate a couple of fields. Not for production paths.
pub(crate) fn test_default_connected_client_state() -> ConnectedClientState {
    let key = [0u8; 32];
    ConnectedClientState {
        enc: MercuryEncryption::from_session_key(key),
        key,
        account_id: 0,
        access_level: 0,
        dnd_message: None,
        char_list_sent: false,
        world_entry_sent: false,
        pending_player_entity_id: None,
        player_entity_id: None,
        next_seq: Arc::new(AtomicU32::new(0)),
        next_seq_unreliable: Arc::new(AtomicU32::new(0)),
        pending_acks: Arc::new(Mutex::new(Vec::new())),
        last_recv: Arc::new(Mutex::new(Instant::now())),
        account_entity_id: 0,
        next_data_id: 0,
        pending_world_entry: None,
        pending_player_load_data: None,
        pending_map_loaded: None,
        pending_client_ready: None,
        deferred_aoi_msgs: Vec::new(),
        cached_appearance_args: None,
        cached_tint_args: None,
        weapon_holstered: true,
        cancelled: Arc::new(AtomicBool::new(false)),
        cinematic_spam_cancel: Arc::new(AtomicBool::new(false)),
        player_name: None,
        player_level: None,
        player_archetype: None,
        world_name: None,
        player_xp: None,
        player_training_points: None,
        active_player_id: None,
        pending_destination_ring_id: None,
        channel: Mutex::new(cimmeria_mercury::channel::Channel::new(
            "127.0.0.1:9999".parse().unwrap(),
        )),
    }
}

// ──────────────────────────────────────────────────────────────────────
// Log capture for negative-logging regression guards.
//
// Regression guards for log-only changes need to assert that a specific
// WARN/ERROR event fired with the right structured fields. Without
// capture, reverting a `trace!` → `warn!` change goes undetected.
// `LogCapture` is a tracing `Layer` that records each event's level,
// target, message body, and field map into a shared `Vec<Captured>`.
//
// Usage:
//
// ```ignore
// let capture = LogCapture::install();
// some_function_that_logs().await;
// assert!(capture.find_event(tracing::Level::WARN, "AoI", "entity_to_addr_miss").is_some());
// ```
//
// # Threading model — IMPORTANT
//
// `LogCapture::install` calls `tracing::subscriber::set_default`, which
// installs the subscriber as the **current thread's** default. Events
// emitted on other threads are **NOT** captured.
//
// In practice: do NOT use `#[tokio::test(flavor = "multi_thread")]`
// with `LogCapture`. The default `#[tokio::test]` flavor is
// `current_thread`, which keeps every awaited future on the test's
// thread — that's what the existing guards rely on. `install()`
// asserts this at runtime: if a tokio multi-thread runtime is
// active when you call `install()`, it panics with a hint, so the
// footgun fails loudly instead of silently dropping events.
// ──────────────────────────────────────────────────────────────────────

use std::collections::HashMap as StdHashMap;
use tracing::{
    field::{Field, Visit},
    span::{Attributes, Id, Record},
    Event, Level, Subscriber,
};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::Registry;

/// One captured event.
#[allow(dead_code)] // fields read via accessors in guard tests
#[derive(Debug, Clone)]
pub(crate) struct Captured {
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
pub(crate) struct LogCapture {
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
    pub(crate) fn install() -> LogCaptureGuard {
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
pub(crate) struct LogCaptureGuard {
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
    #[allow(dead_code)] // debug-only accessor
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
    #[allow(dead_code)] // exercised by npc_ai phase 2-7 guards
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
    // mid-span). Each call appears as a synthetic "record" event whose
    // target is `span_record:<span_name>` so tests can find the recorded
    // value by [`LogCaptureGuard::find_span_attribute`].
    fn on_record(&self, _id: &Id, values: &Record<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        values.record(&mut visitor);
        // We don't have direct access to the span's name from `_id`
        // without walking the registry — but tests want field+value
        // matching, not span-name matching. Tag the row as
        // `span_record:?` and let the visitor's fields carry the
        // discriminator.
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
mod log_capture_tests {
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
