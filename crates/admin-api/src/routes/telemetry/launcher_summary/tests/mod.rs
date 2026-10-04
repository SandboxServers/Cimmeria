//! Tests for the launcher-summary ingest.
//!
//! Almost all of them call `ingest_inner` synchronously with a fresh
//! [`IngestState`] inside a field-recording layer, so a test sees exactly
//! the rows and the verdicts its own request produced. Only `routes` uses a
//! socket, for route exposure and the body limit.
//!
//! Tokens come from the real mint (`dev_session::mint_for_test`), so a
//! change to what the mint issues shows up here.
//!
//! The items marked `pub(super)` are what `fixture_tests/` borrows to
//! capture the rows of the golden request.

mod dedup;
mod envelope;
mod golden;
mod no_echo;
mod order;
mod quota;
mod routes;
mod rows;
mod scope;
mod validation;

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use serde_json::{json, Value};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use crate::routes::dev_session::{env_lock, mint_for_test, DevSessionRequest};

use super::dedup::Dedup;
use super::dto::{SummaryError, SummaryResponse, Verdict};
use super::handlers::{ingest_inner, IngestPolicy, IngestState};
use super::rows::{EVENT_BATCH, EVENT_PHASE, EVENT_SUMMARY};

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../launcher/desktop/engine/src/storage/launcher_summary/fixtures/",
            $name
        ))
    };
}

/// The golden wire fixtures, shared with the desktop launcher's exporter.
const MINT_REQUEST: &str = fixture!("mint-request.json");
pub(super) const REQUEST_ALL: &str = fixture!("request-all.json");
const REQUEST_MIXED: &str = fixture!("request-mixed.json");
/// Recorded from the engine's real install worker, not written by hand.
const REQUEST_INSTALL_FAILURE: &str = fixture!("request-install-failure.json");
const RESPONSE_MIXED: &str = fixture!("response-mixed.json");

const ENV_SECRET: &str = "CIMMERIA_TELEMETRY_HMAC_SECRET";
const ENV_KILL_SWITCH: &str = "CIMMERIA_TELEMETRY_KILL_SWITCH";
const ENV_SUMMARY_QUOTA: &str = "CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP";

/// Holds the crate's env lock for the test, with a usable HMAC secret and
/// the kill switch off; restores both variables on drop. `ingest_inner`
/// reads both, so every test that calls it holds one of these.
pub(super) struct Env {
    _lock: MutexGuard<'static, ()>,
    prev_secret: Option<String>,
    prev_kill: Option<String>,
}

impl Env {
    pub(super) fn install() -> Self {
        let lock = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let prev_secret = std::env::var(ENV_SECRET).ok();
        let prev_kill = std::env::var(ENV_KILL_SWITCH).ok();
        std::env::set_var(ENV_SECRET, "5c".repeat(64));
        std::env::remove_var(ENV_KILL_SWITCH);
        Self {
            _lock: lock,
            prev_secret,
            prev_kill,
        }
    }

    fn set_kill_switch(&self, on: bool) {
        if on {
            std::env::set_var(ENV_KILL_SWITCH, "1");
        } else {
            std::env::remove_var(ENV_KILL_SWITCH);
        }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        for (name, prev) in [
            (ENV_SECRET, self.prev_secret.take()),
            (ENV_KILL_SWITCH, self.prev_kill.take()),
        ] {
            match prev {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// One captured log record.
#[derive(Debug, Clone)]
pub(super) struct Row {
    pub(super) target: String,
    pub(super) level: tracing::Level,
    pub(super) fields: BTreeMap<String, String>,
}

impl Row {
    pub(super) fn event(&self) -> &str {
        self.fields.get("event").map_or("", String::as_str)
    }

    fn keys(&self) -> Vec<&str> {
        self.fields.keys().map(String::as_str).collect()
    }

    /// True if `needle` appears in the target, a field name or a value.
    fn mentions(&self, needle: &str) -> bool {
        self.target.contains(needle)
            || self
                .fields
                .iter()
                .any(|(k, v)| k.contains(needle) || v.contains(needle))
    }
}

/// A copy of the capture layer in `telemetry/replay_tests.rs`: it records
/// each event's own fields, which is what the OTLP bridge exports. It has
/// no filter, so it sees every target at every level.
#[derive(Clone, Default)]
struct Rows(Arc<Mutex<Vec<Row>>>);

struct FieldText<'a>(&'a mut BTreeMap<String, String>);

impl Visit for FieldText<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
}

impl<S: Subscriber> Layer<S> for Rows {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let mut fields = BTreeMap::new();
        event.record(&mut FieldText(&mut fields));
        self.0.lock().unwrap().push(Row {
            target: event.metadata().target().to_string(),
            level: *event.metadata().level(),
            fields,
        });
    }
}

impl Rows {
    /// Run `f` on this thread with the layer as the default subscriber.
    /// `with_default` is per thread, so a test with two threads calls this
    /// on each with clones of one `Rows`.
    fn record<T>(&self, f: impl FnOnce() -> T) -> T {
        let sub = tracing_subscriber::registry().with(self.clone());
        tracing::subscriber::with_default(sub, f)
    }

    fn snapshot(&self) -> Vec<Row> {
        self.0.lock().unwrap().clone()
    }
}

/// `f`'s result and every row it emitted.
pub(super) fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<Row>) {
    let rows = Rows::default();
    let out = rows.record(f);
    (out, rows.snapshot())
}

fn summary_rows(rows: &[Row]) -> Vec<&Row> {
    rows.iter().filter(|r| r.event() == EVENT_SUMMARY).collect()
}

fn phase_rows(rows: &[Row]) -> Vec<&Row> {
    rows.iter().filter(|r| r.event() == EVENT_PHASE).collect()
}

fn batch_rows(rows: &[Row]) -> Vec<&Row> {
    rows.iter().filter(|r| r.event() == EVENT_BATCH).collect()
}

fn bearer(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    headers
}

/// A token minted by the real mint from the launcher's golden mint body.
fn summary_token() -> String {
    let req: DevSessionRequest = serde_json::from_str(MINT_REQUEST).unwrap();
    mint_for_test(req).expect("summary mint").token
}

/// A player (`None`) or lab (`Some("lab")`) token from the real mint.
fn session_token(kind: Option<&str>) -> String {
    mint_for_test(DevSessionRequest {
        install_id: "3f2504e0-4f89-41d3-9a0c-0305e82c3301".into(),
        machine_id: "machine-abc".into(),
        branch: "main".into(),
        git_sha: "0123456".into(),
        launcher_version: "0.1.0".into(),
        tags: Vec::new(),
        session_kind: kind.map(str::to_string),
    })
    .expect("session mint")
    .token
}

/// One ingest under test: fresh state, a policy with the quota off, a
/// valid summary token. Fields are public to the tests so each varies only
/// what it is about.
pub(super) struct Harness {
    state: IngestState,
    policy: IngestPolicy,
    headers: HeaderMap,
    peer: IpAddr,
    now: Instant,
}

impl Harness {
    /// Needs the secret, so call it with an [`Env`] held.
    pub(super) fn new() -> Self {
        Self {
            state: IngestState::new(),
            policy: IngestPolicy {
                window: Duration::from_secs(3_600),
                per_ip: 0,
            },
            headers: bearer(&summary_token()),
            peer: "203.0.113.77".parse().unwrap(),
            now: Instant::now(),
        }
    }

    fn with_dedup_capacity(mut self, capacity: usize) -> Self {
        self.state.dedup = Dedup::with_capacity(capacity);
        self
    }

    pub(super) fn post(&self, body: &[u8]) -> Result<SummaryResponse, SummaryError> {
        ingest_inner(
            &self.state,
            &self.policy,
            self.peer,
            &self.headers,
            body,
            self.now,
        )
    }

    fn post_json(&self, body: &Value) -> Result<SummaryResponse, SummaryError> {
        self.post(body.to_string().as_bytes())
    }

    /// The verdicts for a request that must be a 200.
    fn verdicts(&self, body: &Value) -> Vec<Verdict> {
        self.post_json(body)
            .unwrap_or_else(|e| panic!("expected a 200, got {e:?}"))
            .results
    }
}

/// A UUID that differs per `n`, in canonical form.
fn id(prefix: u32, n: u32) -> String {
    format!("{prefix:08x}-0000-4000-8000-{n:012x}")
}

/// A valid summary carrying every optional field. `n` makes its ids
/// distinct from another element's.
fn element(n: u32) -> Value {
    json!({
        "event_id": id(0xe5, n),
        "attempt_id": id(0xa5, n),
        "operation": "install",
        "phase": "download",
        "outcome": "failed",
        "error_code": "install_failed",
        "duration_ms": 81234,
        "retry_count": 0,
        "phases": [
            { "phase": "starting", "duration_ms": 12 },
            { "phase": "download", "duration_ms": 81000 },
        ],
        "launcher_version": "0.1.0",
        "os": "windows",
        "arch": "x86_64",
    })
}

/// A valid request around `summaries`.
fn batch(summaries: Vec<Value>) -> Value {
    json!({
        "schema_version": 1,
        "client_dropped": { "overflow": 0, "expired": 0, "rejected": 0 },
        "summaries": summaries,
    })
}

/// A refusal as the client receives it.
#[derive(Debug)]
struct Refusal {
    status: u16,
    retry_after: Option<String>,
    body: String,
}

fn refusal(err: SummaryError) -> Refusal {
    let response = err.into_response();
    Refusal {
        status: response.status().as_u16(),
        retry_after: response
            .headers()
            .get(axum::http::header::RETRY_AFTER)
            .map(|v| v.to_str().unwrap().to_string()),
        body: body_text(response),
    }
}

fn body_text(response: axum::response::Response) -> String {
    let bytes = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(axum::body::to_bytes(response.into_body(), 64 * 1024))
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}
