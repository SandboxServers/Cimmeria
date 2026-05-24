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
// Log capture for negative-logging regression guards (issue #304).
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
// Each test scope owns its own subscriber via
// `tracing::subscriber::with_default`; events outside the scope are
// ignored. Capture is process-local — tests using it must not run
// inside `#[tokio::test(flavor = "multi_thread")]` if they care about
// observing only their own events.
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
    /// thread-local default, and return a guard that captures into the
    /// returned `LogCapture`.
    ///
    /// Holds the default-guard for the lifetime of the returned
    /// `LogCaptureGuard`; drop the guard to restore the previous
    /// subscriber.
    pub(crate) fn install() -> LogCaptureGuard {
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
    /// Use the `reason` field convention (per CLAUDE.md / issue #304) to
    /// pin down WHICH negative-log this is, not just any warn at the
    /// same target.
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
    #[allow(dead_code)] // helper API — used by guards in other crates / future PRs
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

    // No-op span methods — capture doesn't care about spans, only events.
    fn on_new_span(&self, _attrs: &Attributes<'_>, _id: &Id, _ctx: Context<'_, S>) {}
    fn on_record(&self, _id: &Id, _values: &Record<'_>, _ctx: Context<'_, S>) {}
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
