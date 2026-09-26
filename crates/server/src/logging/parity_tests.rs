//! NA25 parity guard: whatever a `logs/*.log` file keeps, SigNoz gets, in
//! exactly one log index.
//!
//! The harness installs the *production* filters — one per [`FILE_LAYERS`]
//! row, `server.log`'s, and the three OTLP log filters — on recording layers,
//! then fires events at chosen targets and levels and reads back which sinks
//! accepted each. Targets come from the directive tables themselves, so a new
//! file row is tested the day it is added.
//!
//! Events are built at runtime (a leaked callsite per target) because the
//! `tracing` macros need a `'static` target literal and these targets come
//! out of the tables.

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
    directive_pairs, otel_network_log_filter, otel_server_log_filter, otel_trace_directives,
    otel_trace_log_filter_for, server_log_directives, FileLayer, FILE_LAYERS, OTEL_FILTER,
    OTLP_EXCLUDED_TARGETS, WIRE_FIREHOSE_MUTED,
};

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

// ── The parity guard ─────────────────────────────────────────────────────

/// **The guard.** For every directive of every file layer, a representative
/// event at TRACE, DEBUG and INFO that the file keeps is exported to exactly
/// one OTLP log index — or, for a `wire.firehose.*` target, to none, with its
/// sample exported instead.
#[test]
fn every_file_directive_reaches_an_otlp_index() {
    let v = parity_violations(FILE_LAYERS);
    assert!(v.is_empty(), "file/SigNoz parity broken:\n{}", v.join("\n"));
}

/// The guard's own revert-proof, kept as a test: a file layer for a new
/// system that nobody added to `OTEL_FILTER` is caught. Its TRACE rows are
/// covered (the trace filter derives from the table), its INFO rows ride the
/// default `info`, and its DEBUG rows are dropped — so exactly the DEBUG
/// representatives are reported.
#[test]
fn parity_guard_catches_a_file_layer_without_otlp_coverage() {
    let mut layers = FILE_LAYERS.to_vec();
    layers.push(FileLayer {
        file: "brand_new.log",
        directives: "off,brand_new_system=trace",
    });
    let v = parity_violations(&layers);
    assert_eq!(v.len(), 2, "got {v:#?}");
    assert!(v
        .iter()
        .all(|l| l.contains("brand_new") && l.contains("DEBUG")));
}

/// Every firehose's full target is kept by some file, so moving a row onto
/// its `wire.firehose.*` target did not drop it from disk.
#[test]
fn every_firehose_is_kept_in_full_by_a_file() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    for f in firehose::FIREHOSES {
        let sinks = sinks_for(&dispatch, &hits, f.full_target, Level::TRACE);
        assert!(
            sinks.iter().any(|s| s.starts_with("file:")),
            "{} reaches no log file; add it to its FILE_LAYERS row",
            f.full_target
        );
    }
}

/// `server.log` keeps every INFO+ row from every target. Each of those must
/// be exported, except the exporter's own transport.
#[test]
fn server_log_targets_reach_an_otlp_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let named = directive_pairs(OTEL_FILTER)
        .chain(directive_pairs(WIRE_FIREHOSE_MUTED))
        .map(|(t, _)| t);
    let generic = [
        "cimmeria_server",
        "cimmeria_services::orchestrator",
        "cimmeria_admin_api::routes",
        "sqlx::query",
        "some_third_party_crate",
    ];
    let mut violations = Vec::new();
    for target in named.chain(generic) {
        let excluded = OTLP_EXCLUDED_TARGETS.iter().any(|(t, _)| *t == target);
        for lvl in [Level::INFO, Level::WARN, Level::ERROR] {
            let sinks = sinks_for(&dispatch, &hits, target, lvl);
            if !sinks.contains(SERVER_LOG) {
                continue;
            }
            let n = OTLP_LOG_SINKS
                .iter()
                .filter(|s| sinks.contains(**s))
                .count();
            let want = usize::from(!excluded);
            if n != want {
                violations.push(format!("{target} at {lvl}: {n} OTLP indexes, want {want}"));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// The exclusion list stays honest: each entry really is `off` in
/// `OTEL_FILTER`, and every `off` there is on the list.
#[test]
fn otlp_exclusions_match_the_off_directives() {
    let off: BTreeSet<&str> = directive_pairs(OTEL_FILTER)
        .filter(|(_, l)| l.eq_ignore_ascii_case("off"))
        .map(|(t, _)| t)
        .collect();
    let listed: BTreeSet<&str> = OTLP_EXCLUDED_TARGETS.iter().map(|(t, _)| *t).collect();
    for (t, reason) in OTLP_EXCLUDED_TARGETS {
        assert!(reason.len() > 20, "{t}: every exclusion needs its reason");
    }
    assert_eq!(off, listed);
}

// ── Routing: one index per record ────────────────────────────────────────

/// TRACE goes to `cimmeria-trace` and nowhere else; DEBUG and above keep
/// their pre-NA25 index.
#[test]
fn each_level_lands_in_its_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let otlp = |target: &str, lvl: Level| -> Vec<String> {
        sinks_for(&dispatch, &hits, target, lvl)
            .into_iter()
            .filter(|s| s.starts_with("otlp:"))
            .collect()
    };
    let combat = "cimmeria_services::cell::combat::damage";
    let noise = "cimmeria_services::base::connect_loop::encrypted";
    assert_eq!(otlp(combat, Level::TRACE), [OTLP_TRACE]);
    assert_eq!(otlp(combat, Level::DEBUG), [OTLP_SERVER]);
    assert_eq!(otlp(combat, Level::INFO), [OTLP_SERVER]);
    assert_eq!(otlp(noise, Level::TRACE), [OTLP_TRACE]);
    assert_eq!(otlp(noise, Level::DEBUG), [OTLP_NETWORK]);
    assert_eq!(otlp(noise, Level::INFO), [OTLP_NETWORK]);
    assert_eq!(otlp(noise, Level::WARN), [OTLP_SERVER]);
    assert_eq!(otlp("mercury.packet", Level::INFO), [OTLP_NETWORK]);
    assert_eq!(otlp("npc_ai.transition", Level::DEBUG), [OTLP_SERVER]);
    // Custom-target TRACE rows, which no file keeps.
    assert_eq!(otlp("npc_ai.tick", Level::TRACE), [OTLP_TRACE]);
    // Module paths no file keeps at TRACE stay out of the trace index.
    assert!(otlp("cimmeria_services::orchestrator", Level::TRACE).is_empty());
    // Round 2: `launcher=debug` exports the launcher replays, but the
    // session-key dump under it stays on the host at every level, the trace
    // index (where `launcher` is raised to TRACE) included.
    assert_eq!(otlp("launcher.ingest", Level::DEBUG), [OTLP_SERVER]);
    for lvl in LEVELS {
        assert!(
            otlp("launcher.key_dump", lvl).is_empty(),
            "key_dump at {lvl}"
        );
    }
    // The exporter's own transport never loops back.
    for lvl in LEVELS {
        assert!(otlp("hyper::proto", lvl).is_empty(), "hyper at {lvl}");
    }
}

/// The auth service moved from `cimmeria_services::auth` to the
/// `cimmeria-auth` crate (services crate split, wave W1a), which changed its
/// events' `module_path!()`. They must still land where they did before the
/// move: every level in `auth.log`, INFO and up in `server.log`, and one OTLP
/// index per level. `cimmeria_services=debug` does not prefix-match
/// `cimmeria_auth`, so without its own `OTEL_FILTER` row the DEBUG rows and
/// the handler spans would silently stop reaching SigNoz.
#[test]
fn auth_crate_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let handlers = "cimmeria_auth::auth::handlers";
    let sinks = |lvl| sinks_for(&dispatch, &hits, handlers, lvl);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    assert_eq!(sinks(Level::TRACE), set(&["file:auth.log", OTLP_TRACE]));
    assert_eq!(sinks(Level::DEBUG), set(&["file:auth.log", OTLP_SERVER]));
    for lvl in [Level::INFO, Level::WARN, Level::ERROR] {
        assert_eq!(
            sinks(lvl),
            set(&["file:auth.log", SERVER_LOG, OTLP_SERVER]),
            "{handlers} at {lvl}"
        );
    }
}

/// Cover moved from `cimmeria_services::cell::cover` to the
/// `cimmeria-cell-cover` crate (wave W2a), which changed the `module_path!()`
/// of its untargeted rows: the loader's counts and skipped-row warnings, and
/// the poisoned-mutex warnings. No file layer names cover, so they must still
/// reach `server.log` from INFO and one OTLP index per level as before.
/// `cimmeria_services=debug` does not prefix-match `cimmeria_cell_cover`, so
/// without its own `OTEL_FILTER` row a DEBUG row in the crate would not reach
/// SigNoz, as it did before the move. (The hand-named `cover.*` targets did
/// not change.)
#[test]
fn cover_crate_events_keep_their_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let loader = "cimmeria_cell_cover::cell::cover::loader";
    let sinks = |lvl| sinks_for(&dispatch, &hits, loader, lvl);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    // No file keeps it at TRACE, so the trace index does not either.
    assert!(sinks(Level::TRACE).is_empty(), "{loader} at TRACE");
    assert_eq!(sinks(Level::DEBUG), set(&[OTLP_SERVER]));
    for lvl in [Level::INFO, Level::WARN, Level::ERROR] {
        assert_eq!(
            sinks(lvl),
            set(&[SERVER_LOG, OTLP_SERVER]),
            "{loader} at {lvl}"
        );
    }
}

/// The spawner's DB loaders moved from `cimmeria_services::cell::spawner` to
/// the `cimmeria-cell-catalog` crate (services crate split, wave W2b), which
/// changed their events' `module_path!()`. They must still land where they
/// did before: every level in `spawner.log`, INFO and up in `server.log`, and
/// one OTLP index per level. `cimmeria_services=debug` does not prefix-match
/// `cimmeria_cell_catalog`, so without its own `OTEL_FILTER` row the DEBUG
/// rows would silently stop reaching SigNoz. The ability-tree catalog has no
/// file of its own and keeps `server.log` plus its index.
#[test]
fn catalog_crate_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    let loaders = "cimmeria_cell_catalog::cell::spawner::npcs";
    let sinks = |lvl| sinks_for(&dispatch, &hits, loaders, lvl);
    assert_eq!(sinks(Level::TRACE), set(&["file:spawner.log", OTLP_TRACE]));
    assert_eq!(sinks(Level::DEBUG), set(&["file:spawner.log", OTLP_SERVER]));
    for lvl in [Level::INFO, Level::WARN, Level::ERROR] {
        assert_eq!(
            sinks(lvl),
            set(&["file:spawner.log", SERVER_LOG, OTLP_SERVER]),
            "{loaders} at {lvl}"
        );
    }
    let tree = "cimmeria_cell_catalog::ability_tree::catalog";
    assert_eq!(
        sinks_for(&dispatch, &hits, tree, Level::DEBUG),
        set(&[OTLP_SERVER])
    );
    assert_eq!(
        sinks_for(&dispatch, &hits, tree, Level::INFO),
        set(&[SERVER_LOG, OTLP_SERVER])
    );
}

/// `spawn_npcs_from_records` and `spawn_instance_npcs_from_records` moved
/// from `cell::spawner::npcs` to `cell::space_manager::npc_population`
/// (services crate split, wave W2b). `aoi.log` keeps every
/// `cell::space_manager` module, so the move alone would have re-routed the
/// spawn rows there; they must stay in `spawner.log` and only there, while
/// the rest of `space_manager` keeps `aoi.log`.
#[test]
fn npc_population_events_keep_spawner_log() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let population = "cimmeria_services::cell::space_manager::npc_population";
    let spawn = "cimmeria_services::cell::space_manager::spawn";
    for lvl in LEVELS {
        let sinks = sinks_for(&dispatch, &hits, population, lvl);
        assert!(
            sinks.contains("file:spawner.log") && !sinks.contains("file:aoi.log"),
            "{population} at {lvl}: {sinks:?}"
        );
        let sinks = sinks_for(&dispatch, &hits, spawn, lvl);
        assert!(
            sinks.contains("file:aoi.log") && !sinks.contains("file:spawner.log"),
            "{spawn} at {lvl}: {sinks:?}"
        );
    }
}

/// No target, at any level, is indexed twice.
#[test]
fn no_record_reaches_two_indexes() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let targets = FILE_LAYERS
        .iter()
        .flat_map(|l| directive_pairs(l.directives))
        .chain(directive_pairs(OTEL_FILTER))
        .map(|(t, _)| t)
        .chain(firehose::FIREHOSES.iter().map(|f| f.sample_target));
    for target in targets {
        for lvl in LEVELS {
            let n = sinks_for(&dispatch, &hits, target, lvl)
                .iter()
                .filter(|s| s.starts_with("otlp:"))
                .count();
            assert!(n <= 1, "{target} at {lvl} reaches {n} OTLP log indexes");
        }
    }
}

/// The derived trace directives: file targets and custom targets at TRACE,
/// the firehose off, no module-path blanket.
#[test]
fn trace_directives_are_derived_from_the_tables() {
    let d = otel_trace_directives();
    let has = |needle: &str| d.split(',').any(|x| x == needle);
    assert!(d.starts_with("off,"), "{d}");
    assert!(has("cimmeria_services::base::connect_loop=trace"), "{d}");
    assert!(has("cimmeria_mercury=trace"), "{d}");
    assert!(has("movement.navmesh=trace"), "{d}");
    assert!(has("wire.sampled=trace"), "{d}");
    assert!(has("wire.firehose=off"), "{d}");
    assert!(!d.contains("wire.firehose.decrypt"), "{d}");
    assert!(!has("cimmeria_services=trace"), "{d}");
    assert!(!has("hyper=trace"), "{d}");
    d.parse::<EnvFilter>()
        .expect("derived directives must parse");
}

/// The `movement.navmesh` `advisory_off_mesh_accepted` row is gated by
/// `tracing::enabled!(target: "movement.navmesh", TRACE)`. Before NA25 no
/// layer enabled it and it never fired; the trace index does.
#[test]
fn navmesh_advisory_row_is_enabled_and_exported() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let meta = event_meta("movement.navmesh", Level::TRACE);
    assert!(dispatch.enabled(meta), "the enabled! gate must now pass");
    let sinks = sinks_for(&dispatch, &hits, "movement.navmesh", Level::TRACE);
    assert!(sinks.contains(OTLP_TRACE), "{sinks:?}");
}

// ── Firehose sampling through the real filters ───────────────────────────

/// Run `emit` `n` times through the harness; return
/// (rows in `file`, sampled rows exported, sum of `1 + suppressed`).
fn run_firehose(n: u64, file: &str, full_target: &str, emit: impl Fn()) -> (u64, u64, u64) {
    let (dispatch, hits) = harness(FILE_LAYERS);
    tracing::dispatcher::with_default(&dispatch, || {
        for _ in 0..n {
            emit();
        }
    });
    let hits = hits.lock().unwrap().clone();
    let in_file = hits
        .iter()
        .filter(|h| h.sink == format!("file:{file}") && h.target == full_target)
        .count() as u64;
    assert!(
        !hits
            .iter()
            .any(|h| h.sink.starts_with("otlp:") && h.target == full_target),
        "the full firehose {full_target} must not be exported"
    );
    let sampled: Vec<_> = hits
        .iter()
        .filter(|h| h.sink.starts_with("otlp:") && h.target != full_target)
        .collect();
    let reconstructed = sampled.iter().map(|h| 1 + h.suppressed.unwrap_or(0)).sum();
    (in_file, sampled.len() as u64, reconstructed)
}

fn addr() -> std::net::SocketAddr {
    "127.0.0.1:32832".parse().unwrap()
}

/// `n` DECRYPT_OK rows: all `n` in base.log, `ceil(n / 53)` in
/// `cimmeria-trace`, and the samples' counts add back up. Dropping the
/// `admit()` gate fails the sample count (1000 vs 19, verified); a firehose
/// target missing from `wire.firehose=off`'s reach fails the "not exported"
/// assertion.
#[test]
fn decrypt_ok_is_complete_on_disk_and_sampled_in_signoz() {
    let s = firehose::FirehoseSampler::new(firehose::DECRYPT_OK_SAMPLE_EVERY);
    let n = 1_000;
    let (file, sampled, sum) = run_firehose(n, "base.log", firehose::DECRYPT_OK_TARGET, || {
        firehose::log_decrypt_ok(&s, addr(), &[0xAB]);
    });
    assert_eq!(file, n);
    assert_eq!(sampled, n.div_ceil(53));
    assert_eq!(sum, (sampled - 1) * 53 + 1);
}

#[test]
fn udp_in_is_complete_on_disk_and_sampled_in_signoz() {
    let s = firehose::FirehoseSampler::new(firehose::UDP_IN_SAMPLE_EVERY);
    let n = 500;
    let (file, sampled, _) = run_firehose(n, "base.log", firehose::UDP_IN_TARGET, || {
        firehose::log_udp_in(&s, addr(), &[1, 2]);
    });
    assert_eq!((file, sampled), (n, n.div_ceil(53)));
}

/// The AoI relay: every row in world_entry.log, a 1-in-101
/// `wire.out.avatar_update` DEBUG sample in `cimmeria-server`.
#[test]
fn aoi_position_is_complete_on_disk_and_sampled_in_signoz() {
    let s = firehose::FirehoseSampler::new(firehose::AOI_POSITION_SAMPLE_EVERY);
    let row = firehose::EntityMovedRow {
        witness_id: 1,
        entity_id: 2,
        position: [0.0; 3],
        direction: [0.0; 3],
        velocity: [0.0; 3],
        npc_moved_since_last: None,
    };
    let n = 1_010;
    let (file, sampled, sum) =
        run_firehose(n, "world_entry.log", firehose::AOI_POSITION_TARGET, || {
            firehose::log_entity_moved(&s, &row)
        });
    assert_eq!(file, n);
    assert_eq!(sampled, n.div_ceil(101));
    assert_eq!(sum, (sampled - 1) * 101 + 1);
}
