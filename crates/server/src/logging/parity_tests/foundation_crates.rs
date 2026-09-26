//! The services crate split's first waves (W1-W3) moved code into crates
//! below both the base and the cell track: `cimmeria-auth`,
//! `cimmeria-cell-cover`, `cimmeria-minigame`, `cimmeria-cell-catalog`,
//! `cimmeria-wire` and `cimmeria-wire-log`. Each move changed the moved
//! events' `module_path!()`; these guards pin that each still lands in the
//! file and the OTLP index it did before.

use std::collections::BTreeSet;
use std::sync::Arc;

use tracing::{Dispatch, Level};
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::EnvFilter;

use super::{harness, recorder, sinks_for, Hits, OTLP_SERVER, OTLP_TRACE, SERVER_LOG};
use crate::logging::filters::{FILE_LAYERS, OTEL_FILTER};

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

/// The SmartFoxServer host moved from `cimmeria_services::minigame` to the
/// `cimmeria-minigame` crate (services crate split, wave W3c), which changed
/// the `module_path!()` of every row it logs: none names a `target:`. No file
/// layer names minigame, so the rows must still reach `server.log` from INFO
/// and exactly one OTLP index per level, `cimmeria-server` below TRACE (the
/// session and connection rows are per session, not per packet, so they are
/// not network noise). `cimmeria_services=debug` does not prefix-match
/// `cimmeria_minigame`, so without its own `OTEL_FILTER` row the DEBUG rows
/// (connection accepted and closed, socket read and send errors, a command
/// the placeholder game ignores) would silently stop reaching SigNoz.
#[test]
fn minigame_crate_events_keep_their_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for target in [
        "cimmeria_minigame::minigame::server",
        "cimmeria_minigame::minigame::server::framing",
        "cimmeria_minigame::minigame::server::handshake",
        "cimmeria_minigame::minigame::server::result_dispatch",
        "cimmeria_minigame::minigame::session",
        "cimmeria_minigame::minigame::games::livewire",
        "cimmeria_minigame::minigame::games::placeholder",
    ] {
        let sinks = |lvl| sinks_for(&dispatch, &hits, target, lvl);
        // No file keeps it at TRACE, so the trace index does not either.
        assert!(sinks(Level::TRACE).is_empty(), "{target} at TRACE");
        assert_eq!(
            sinks(Level::DEBUG),
            set(&[OTLP_SERVER]),
            "{target} at DEBUG"
        );
        for lvl in [Level::INFO, Level::WARN, Level::ERROR] {
            assert_eq!(
                sinks(lvl),
                set(&[SERVER_LOG, OTLP_SERVER]),
                "{target} at {lvl}"
            );
        }
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

/// The services-side Mercury glue moved from `cimmeria_services::mercury` to
/// the `cimmeria-wire` crate (services crate split, wave W3a), which changed
/// the `module_path!()` of its untargeted rows: the `append_entity_method`
/// appearance diagnostics (DEBUG, `mercury`), the map-load rows (INFO,
/// `world_data::map_loaded`) and the unknown-world fallback (WARN,
/// `world_data`). They must still land where they did: every level in
/// `protocol.log`, INFO and up in `server.log`, and one OTLP index per level,
/// `cimmeria-server` below TRACE. A `protocol.log` row still naming
/// `cimmeria_services::mercury` empties the file of them, and without
/// `cimmeria_wire::mercury=debug` the DEBUG rows stop reaching SigNoz. (The
/// AoI builders and the firehoses in the same crate log on hand-named
/// targets, `aoi.*` and `wire.*`, which the move did not change; the three
/// `*_is_complete_on_disk_and_sampled_in_signoz` tests in `firehose_sampling`
/// drive the moved firehose emitters.)
#[test]
fn wire_mercury_events_keep_protocol_log_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for target in [
        "cimmeria_wire::mercury",
        "cimmeria_wire::mercury::world_data",
        "cimmeria_wire::mercury::world_data::map_loaded",
    ] {
        let sinks = |lvl| sinks_for(&dispatch, &hits, target, lvl);
        assert_eq!(
            sinks(Level::TRACE),
            set(&["file:protocol.log", OTLP_TRACE]),
            "{target} at TRACE"
        );
        assert_eq!(
            sinks(Level::DEBUG),
            set(&["file:protocol.log", OTLP_SERVER]),
            "{target} at DEBUG"
        );
        for lvl in [Level::INFO, Level::WARN, Level::ERROR] {
            assert_eq!(
                sinks(lvl),
                set(&["file:protocol.log", SERVER_LOG, OTLP_SERVER]),
                "{target} at {lvl}"
            );
        }
    }
}

/// The decoded wire-message stream moved from `cimmeria_services::wire_log`
/// to the `cimmeria-wire-log` crate (services crate split, wave W3b). Its rows
/// use the hand-named `wire.in` / `wire.out` targets, which the move did not
/// change: at INFO, the level they are emitted at, they must still reach
/// `protocol.log` and `cimmeria-server`, and stay out of `server.log`
/// (`WIRE_FIREHOSE_MUTED`). An untargeted row in the crate would now carry
/// `cimmeria_wire_log::…`, which no file names, so it must get what
/// `cimmeria_services::wire_log` got: `server.log` from INFO and one OTLP
/// index per level. Its DEBUG export comes from `cimmeria_wire_log=debug`
/// alone: with that row dropped, the DEBUG row must stop reaching SigNoz.
/// Until wave F wire's row was a bare `cimmeria_wire=debug`, which
/// prefix-matched this crate too, so dropping this row changed nothing and
/// this test pinned only that lowering wire's row left wire-log's export
/// alone; wire's rows now name its modules (`crate_rows`).
#[test]
fn wire_log_events_keep_protocol_log_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for target in ["wire.in", "wire.out"] {
        let sinks = |lvl| sinks_for(&dispatch, &hits, target, lvl);
        assert_eq!(
            sinks(Level::INFO),
            set(&["file:protocol.log", OTLP_SERVER]),
            "{target} at INFO"
        );
        for lvl in [Level::WARN, Level::ERROR] {
            assert_eq!(
                sinks(lvl),
                set(&["file:protocol.log", SERVER_LOG, OTLP_SERVER]),
                "{target} at {lvl}"
            );
        }
    }
    for target in [
        "cimmeria_wire_log::wire_log",
        "cimmeria_wire_log::wire_log::tap",
    ] {
        let sinks = |lvl| sinks_for(&dispatch, &hits, target, lvl);
        // No file keeps it at TRACE, so the trace index does not either.
        assert!(sinks(Level::TRACE).is_empty(), "{target} at TRACE");
        assert_eq!(
            sinks(Level::DEBUG),
            set(&[OTLP_SERVER]),
            "{target} at DEBUG"
        );
        for lvl in [Level::INFO, Level::WARN, Level::ERROR] {
            assert_eq!(
                sinks(lvl),
                set(&[SERVER_LOG, OTLP_SERVER]),
                "{target} at {lvl}"
            );
        }
    }

    let row = "cimmeria_wire_log=debug,";
    let without = OTEL_FILTER.replace(row, "");
    assert_ne!(
        without, OTEL_FILTER,
        "OTEL_FILTER no longer carries `{row}`; update this test"
    );
    let hits: Hits = Arc::default();
    let dispatch = Dispatch::new(
        tracing_subscriber::registry()
            .with(recorder(OTLP_SERVER.into(), &hits).with_filter(EnvFilter::new(without))),
    );
    assert!(
        sinks_for(
            &dispatch,
            &hits,
            "cimmeria_wire_log::wire_log",
            Level::DEBUG
        )
        .is_empty(),
        "wire-log's DEBUG export must depend on its own `{row}` row, not on a wire row"
    );
}
