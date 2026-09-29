//! The NA25 parity guard itself, and the routing rules it relies on: each
//! record lands in exactly one OTLP log index, the TRACE filter is derived
//! from the file table, and the `off` exclusions are the only rows SigNoz
//! never sees.

use std::collections::BTreeSet;

use cimmeria_services::firehose;
use tracing::Level;
use tracing_subscriber::EnvFilter;

use super::{
    event_meta, harness, parity_violations, sinks_for, LEVELS, OTLP_CLIENT, OTLP_LOG_SINKS,
    OTLP_NETWORK, OTLP_SERVER, OTLP_TRACE, SERVER_LOG,
};
use crate::logging::filters::{
    directive_pairs, otel_trace_directives, FileLayer, FILE_LAYERS, OTEL_FILTER,
    OTLP_EXCLUDED_TARGETS, WIRE_FIREHOSE_MUTED,
};

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
    // Combat moved to cimmeria-cell-combat in wave C2 of the services split.
    let combat = "cimmeria_cell_combat::cell::combat::damage";
    let noise = "cimmeria_base::base::connect_loop::encrypted";
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

/// The telemetry ingest's client-side replays land in `cimmeria-client` at
/// every level, and nowhere else: not WARN in `cimmeria-server`, not TRACE
/// in `cimmeria-trace`. The ingest's own rows about an upload
/// (`launcher.ingest`, `launcher.bundle`) stay server-side, and the key
/// dump stays off everywhere.
///
/// Reverting the client exclusion in `routes_to_server` or `routes_to_trace`
/// makes a client row reach two indexes and fails this.
#[test]
fn client_replays_land_only_in_the_client_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let otlp = |target: &str, lvl: Level| -> Vec<String> {
        sinks_for(&dispatch, &hits, target, lvl)
            .into_iter()
            .filter(|s| s.starts_with("otlp:"))
            .collect()
    };
    for target in crate::otel::CLIENT_TARGETS {
        for lvl in LEVELS {
            assert_eq!(otlp(target, lvl), [OTLP_CLIENT], "{target} at {lvl}");
        }
    }
    assert_eq!(otlp("launcher.ingest", Level::DEBUG), [OTLP_SERVER]);
    assert_eq!(otlp("launcher.bundle", Level::WARN), [OTLP_SERVER]);
    // A sibling that only shares the prefix is not a client row.
    assert_eq!(otlp("client.nativex", Level::WARN), [OTLP_SERVER]);
    for lvl in LEVELS {
        assert!(
            otlp("launcher.key_dump", lvl).is_empty(),
            "key_dump at {lvl}"
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
    assert!(has("cimmeria_base::base::connect_loop=trace"), "{d}");
    assert!(has("cimmeria_mercury=trace"), "{d}");
    assert!(has("cimmeria_wire::mercury=trace"), "{d}");
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
