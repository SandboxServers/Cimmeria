//! NA25 parity guard: whatever a `logs/*.log` file keeps, SigNoz gets, in
//! exactly one log index.
//!
//! The harness installs the *production* filters — one per `FILE_LAYERS`
//! row, `server.log`'s, and the three OTLP log filters — on recording layers,
//! then fires events at chosen targets and levels and reads back which sinks
//! accepted each. Targets come from the directive tables themselves, so a new
//! file row is tested the day it is added.
//!
//! Events are built at runtime (a leaked callsite per target) because the
//! `tracing` macros need a `'static` target literal and these targets come
//! out of the tables.
//!
//! This module is the harness. The tests are in its children:
//!
//! - [`guard`]: the parity guard itself and the routing rules (one index per
//!   record, the derived TRACE filter, the exclusion list);
//! - [`firehose_sampling`]: the per-packet firehoses, complete on disk and
//!   sampled in SigNoz;
//! - [`crate_rows`]: every in-process crate has its own `OTEL_FILTER` row,
//!   and no row reaches into two crates;
//! - one guard per crate the services crate split
//!   (`docs/architecture/services-crate-split.md`) moved code into, that the
//!   moved modules' events keep their file and index: [`foundation_crates`]
//!   for the crates split out below both tracks (auth, cover, catalog, wire,
//!   wire-log, minigame), [`base_track`] for the four base crates,
//!   [`cell_track_lower`] for the world, combat and content crates, and
//!   [`cell_track_upper`] for the interactions, console, cell-method and cell
//!   crates.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use cimmeria_services::firehose;
use tracing::callsite::{Callsite, Identifier};
use tracing::field::{Field, FieldSet, Visit};
use tracing::metadata::Kind;
use tracing::{Dispatch, Event, Level, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::{EnvFilter, Registry};

use super::filters::{
    directive_pairs, otel_network_log_filter, otel_server_log_filter, otel_trace_log_filter_for,
    server_log_directives, FileLayer,
};

mod base_track;
mod cell_track_lower;
mod cell_track_upper;
mod crate_rows;
mod firehose_sampling;
mod foundation_crates;
mod guard;

pub(super) const OTLP_SERVER: &str = "otlp:cimmeria-server";
pub(super) const OTLP_NETWORK: &str = "otlp:cimmeria-network";
pub(super) const OTLP_TRACE: &str = "otlp:cimmeria-trace";
const SERVER_LOG: &str = "file:server.log";
pub(super) const OTLP_LOG_SINKS: [&str; 3] = [OTLP_SERVER, OTLP_NETWORK, OTLP_TRACE];
const LEVELS: [Level; 5] = [
    Level::TRACE,
    Level::DEBUG,
    Level::INFO,
    Level::WARN,
    Level::ERROR,
];

// ── Harness ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(super) struct Hit {
    sink: String,
    target: String,
    suppressed: Option<u64>,
}

pub(super) type Hits = Arc<Mutex<Vec<Hit>>>;

struct Recorder {
    sink: String,
    hits: Hits,
}

#[derive(Default)]
struct Suppressed(Option<u64>);

impl Visit for Suppressed {
    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "suppressed" {
            self.0 = Some(value);
        }
    }
    fn record_debug(&mut self, _: &Field, _: &dyn std::fmt::Debug) {}
}

impl<S: Subscriber> Layer<S> for Recorder {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut s = Suppressed::default();
        event.record(&mut s);
        self.hits.lock().unwrap().push(Hit {
            sink: self.sink.clone(),
            target: event.metadata().target().to_string(),
            suppressed: s.0,
        });
    }
}

type BoxLayer = Box<dyn Layer<Registry> + Send + Sync>;

fn recorder(sink: String, hits: &Hits) -> Recorder {
    Recorder {
        sink,
        hits: hits.clone(),
    }
}

/// Every file sink plus the three OTLP log sinks, with production filters.
/// `file_layers` is a parameter so the guard's self-test can add a row.
pub(super) fn harness(file_layers: &[FileLayer]) -> (Dispatch, Hits) {
    let hits: Hits = Arc::default();
    let mut layers: Vec<BoxLayer> = Vec::new();
    for l in file_layers {
        layers.push(Box::new(
            recorder(format!("file:{}", l.file), &hits).with_filter(EnvFilter::new(l.directives)),
        ));
    }
    layers.push(Box::new(
        recorder(SERVER_LOG.into(), &hits).with_filter(EnvFilter::new(server_log_directives())),
    ));
    layers.push(Box::new(
        recorder(OTLP_SERVER.into(), &hits).with_filter(otel_server_log_filter()),
    ));
    layers.push(Box::new(
        recorder(OTLP_NETWORK.into(), &hits).with_filter(otel_network_log_filter()),
    ));
    layers.push(Box::new(
        recorder(OTLP_TRACE.into(), &hits).with_filter(otel_trace_log_filter_for(file_layers)),
    ));
    (
        Dispatch::new(tracing_subscriber::registry().with(layers)),
        hits,
    )
}

struct LeakedCallsite(std::sync::OnceLock<&'static Metadata<'static>>);

impl Callsite for LeakedCallsite {
    fn set_interest(&self, _: tracing::subscriber::Interest) {}
    fn metadata(&self) -> &Metadata<'_> {
        self.0.get().expect("metadata set at construction")
    }
}

/// Event metadata for a runtime `target`. Leaks a few bytes per call; the
/// test process is short-lived.
fn event_meta(target: &str, level: Level) -> &'static Metadata<'static> {
    let cs: &'static LeakedCallsite =
        Box::leak(Box::new(LeakedCallsite(std::sync::OnceLock::new())));
    let target: &'static str = Box::leak(target.to_owned().into_boxed_str());
    let meta: &'static Metadata<'static> = Box::leak(Box::new(Metadata::new(
        "parity event",
        target,
        level,
        None,
        None,
        None,
        FieldSet::new(&[], Identifier(cs)),
        Kind::EVENT,
    )));
    cs.0.set(meta).ok();
    meta
}

/// The sinks that accept one event at `target`/`level`, dispatched the way
/// the macros do it: `enabled` first (which records each per-layer filter's
/// verdict), then `event`.
pub(super) fn sinks_for(
    dispatch: &Dispatch,
    hits: &Hits,
    target: &str,
    level: Level,
) -> BTreeSet<String> {
    hits.lock().unwrap().clear();
    let meta = event_meta(target, level);
    tracing::dispatcher::with_default(dispatch, || {
        tracing::dispatcher::get_default(|d| {
            if d.enabled(meta) {
                let values: [(&Field, Option<&dyn tracing::Value>); 0] = [];
                d.event(&Event::new(meta, &meta.fields().value_set(&values)));
            }
        });
    });
    hits.lock()
        .unwrap()
        .iter()
        .map(|h| h.sink.clone())
        .collect()
}

/// The directive target and a child under it: a module-path directive
/// covers its submodules, a dotted one covers `target.*`.
fn representatives(target: &str) -> [String; 2] {
    let child = if target.contains("::") || target.starts_with("cimmeria_") {
        format!("{target}::parity_child")
    } else {
        format!("{target}.parity_child")
    };
    [target.to_string(), child]
}

fn firehose_for(target: &str) -> Option<&'static firehose::Firehose> {
    firehose::FIREHOSES
        .iter()
        .find(|f| target.starts_with(f.full_target))
}

/// Every way `file_layers` breaks parity, as human-readable lines.
fn parity_violations(file_layers: &[FileLayer]) -> Vec<String> {
    let (dispatch, hits) = harness(file_layers);
    let mut out = Vec::new();
    for layer in file_layers {
        let file_sink = format!("file:{}", layer.file);
        for (target, level) in directive_pairs(layer.directives) {
            if level.eq_ignore_ascii_case("off") {
                continue;
            }
            for t in representatives(target) {
                for lvl in [Level::TRACE, Level::DEBUG, Level::INFO] {
                    let sinks = sinks_for(&dispatch, &hits, &t, lvl);
                    if !sinks.contains(&file_sink) {
                        continue; // the file does not keep it; nothing owed
                    }
                    let exported: Vec<_> = OTLP_LOG_SINKS
                        .iter()
                        .filter(|s| sinks.contains(**s))
                        .collect();
                    // Firehoses are TRACE rows. At any other level a
                    // `wire.firehose.*` row is an ordinary row and owes an
                    // index like any other.
                    if let Some(f) = firehose_for(&t).filter(|_| lvl == Level::TRACE) {
                        if !exported.is_empty() {
                            out.push(format!(
                                "{}: firehose {t} at {lvl} is exported in full to {exported:?}",
                                layer.file
                            ));
                        }
                        let sample = sinks_for(&dispatch, &hits, f.sample_target, f.sample_level);
                        if !OTLP_LOG_SINKS.iter().any(|s| sample.contains(*s)) {
                            out.push(format!(
                                "{}: firehose {t}'s sample {} at {} reaches no OTLP index",
                                layer.file, f.sample_target, f.sample_level
                            ));
                        }
                    } else if exported.len() != 1 {
                        out.push(format!(
                            "{}: {t} at {lvl} reaches {} OTLP log indexes {exported:?}, want 1",
                            layer.file,
                            exported.len()
                        ));
                    }
                }
            }
        }
    }
    out
}
