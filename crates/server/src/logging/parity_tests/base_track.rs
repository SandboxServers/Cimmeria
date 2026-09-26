//! The services crate split's base track (waves B1-B4) moved the BaseApp
//! into `cimmeria-base-session`, `cimmeria-base-methods`,
//! `cimmeria-base-world-entry` and `cimmeria-base`. Each move changed the
//! moved events' `module_path!()`; these guards pin that each still lands in
//! the file and the OTLP index it did before.

use std::collections::BTreeSet;
use std::sync::Arc;

use tracing::{Dispatch, Level};
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::EnvFilter;

use super::{
    harness, recorder, sinks_for, Hits, OTLP_NETWORK, OTLP_SERVER, OTLP_TRACE, SERVER_LOG,
};
use crate::logging::filters::{FILE_LAYERS, OTEL_FILTER};

/// The BaseApp session layer moved from `cimmeria_services::base` to the
/// `cimmeria-base-session` crate (services crate split, wave B1), which
/// changed its events' `module_path!()`. They must still land where they did
/// before the move:
///
/// - the send helpers keep `base.log`, and tick sync keeps `base.log` and the
///   `cimmeria-network` index below WARN (`otel::is_network_noise_target`);
/// - cooked-data delivery keeps `character.log`;
/// - the space registry keeps `world_entry.log`, which the old
///   `cimmeria_services::base::world_entry` row reached by prefix;
/// - the modules no file names (outbox, contact list, deferred AoI, crafting,
///   GM spawn) keep `server.log` from INFO;
///
/// and every one reaches one OTLP index per level. `cimmeria_services=debug`
/// does not prefix-match `cimmeria_base_session`, so without its own
/// `OTEL_FILTER` row the DEBUG rows would silently stop reaching SigNoz; a
/// file row still naming `cimmeria_services::base::…` empties the file of
/// them.
#[test]
fn base_session_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for (target, file, noise) in [
        (
            "cimmeria_base_session::base::helpers",
            "file:base.log",
            false,
        ),
        (
            "cimmeria_base_session::base::helpers::witness_broadcast",
            "file:base.log",
            false,
        ),
        (
            "cimmeria_base_session::base::tick_sync",
            "file:base.log",
            true,
        ),
        (
            "cimmeria_base_session::base::cooked_data",
            "file:character.log",
            false,
        ),
        (
            "cimmeria_base_session::base::world_entry::space_registry",
            "file:world_entry.log",
            false,
        ),
    ] {
        let sinks = |lvl| sinks_for(&dispatch, &hits, target, lvl);
        let below_warn = if noise { OTLP_NETWORK } else { OTLP_SERVER };
        assert_eq!(
            sinks(Level::TRACE),
            set(&[file, OTLP_TRACE]),
            "{target} at TRACE"
        );
        assert_eq!(
            sinks(Level::DEBUG),
            set(&[file, below_warn]),
            "{target} at DEBUG"
        );
        assert_eq!(
            sinks(Level::INFO),
            set(&[file, SERVER_LOG, below_warn]),
            "{target} at INFO"
        );
        for lvl in [Level::WARN, Level::ERROR] {
            assert_eq!(
                sinks(lvl),
                set(&[file, SERVER_LOG, OTLP_SERVER]),
                "{target} at {lvl}"
            );
        }
    }
    for target in [
        "cimmeria_base_session::base::outbox",
        "cimmeria_base_session::base::contact_list::handlers::presence_fanout",
        "cimmeria_base_session::base::deferred_aoi",
        "cimmeria_base_session::base::crafting::persistence",
        "cimmeria_base_session::base::gm_spawn",
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

/// The BaseApp's feature handlers moved from
/// `cimmeria_services::base::world_entry::methods` to the
/// `cimmeria-base-methods` crate (services crate split, wave B2), which
/// changed the `module_path!()` of their untargeted rows: inventory grants,
/// moves and use, vendor purchases and repairs, the trade swap, mail
/// forwarding, mission persistence, player load and progression. The old
/// `cimmeria_services::base::world_entry` row kept them in `world_entry.log`
/// by prefix, and they must still land there at every level, in `server.log`
/// from INFO, and in one OTLP index per level, `cimmeria-server` below TRACE
/// (they are per request, not per packet, so not network noise).
/// `cimmeria_services=debug` and
/// `cimmeria_base_session=debug` do not prefix-match `cimmeria_base_methods`,
/// so without its own `OTEL_FILTER` row the DEBUG rows would silently stop
/// reaching SigNoz, and without its own `world_entry.log` row the file would
/// lose them. (The hand-named `abilities`, `progression` and
/// `trade.atomic_swap` targets did not change.)
#[test]
fn base_methods_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for target in [
        "cimmeria_base_methods::base::world_entry::methods::inventory::grant::grant_item",
        "cimmeria_base_methods::base::world_entry::methods::inventory::move_",
        "cimmeria_base_methods::base::world_entry::methods::vendor::purchase",
        "cimmeria_base_methods::base::world_entry::methods::vendor::helpers",
        "cimmeria_base_methods::base::world_entry::methods::trade::execute",
        "cimmeria_base_methods::base::world_entry::methods::trade::execute::swap",
        "cimmeria_base_methods::base::world_entry::methods::mail",
        "cimmeria_base_methods::base::world_entry::methods::missions",
        "cimmeria_base_methods::base::world_entry::methods::player_load::core::player_data",
        "cimmeria_base_methods::base::world_entry::methods::progression",
        "cimmeria_base_methods::base::world_entry::methods::world_entry_db",
    ] {
        let sinks = |lvl| sinks_for(&dispatch, &hits, target, lvl);
        let file = "file:world_entry.log";
        assert_eq!(
            sinks(Level::TRACE),
            set(&[file, OTLP_TRACE]),
            "{target} at TRACE"
        );
        assert_eq!(
            sinks(Level::DEBUG),
            set(&[file, OTLP_SERVER]),
            "{target} at DEBUG"
        );
        for lvl in [Level::INFO, Level::WARN, Level::ERROR] {
            assert_eq!(
                sinks(lvl),
                set(&[file, SERVER_LOG, OTLP_SERVER]),
                "{target} at {lvl}"
            );
        }
    }
}

/// World entry, the CellToBase dispatch, `onClientReady`, the cinematic AoI
/// hold's release and the character list moved from
/// `cimmeria_services::base::{world_entry, world_entry_appearance, character}`
/// to the `cimmeria-base-world-entry` crate (services crate split, wave B3),
/// which changed the `module_path!()` of their untargeted rows. They must still
/// land where they did: world entry and its appearance half in
/// `world_entry.log`, the character list in `character.log`, at every level,
/// in `server.log` from INFO, and in one OTLP index per level,
/// `cimmeria-server` below TRACE (the AoI dispatch runs per AoI event, not per
/// datagram, so it is not network noise). None of `cimmeria_services=debug`,
/// `cimmeria_base_session=debug` and `cimmeria_base_methods=debug`
/// prefix-matches `cimmeria_base_world_entry`, so without its own
/// `OTEL_FILTER` row the DEBUG rows would silently stop reaching SigNoz, and
/// without its own file rows the files would lose them.
///
/// The hand-named `aoi.*` rows the AoI dispatch and the hold emit did not
/// change; `aoi.cinematic_hold` and `aoi.create_emit` are pinned here too, so
/// the move is seen not to have touched them.
#[test]
fn base_world_entry_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for (target, file) in [
        (
            "cimmeria_base_world_entry::base::world_entry::cell_dispatch::aoi_dispatch",
            "file:world_entry.log",
        ),
        (
            "cimmeria_base_world_entry::base::world_entry::cell_dispatch::inventory_dispatch",
            "file:world_entry.log",
        ),
        (
            "cimmeria_base_world_entry::base::world_entry::gate_travel",
            "file:world_entry.log",
        ),
        (
            "cimmeria_base_world_entry::base::world_entry::gate_travel::persist_arrival",
            "file:world_entry.log",
        ),
        (
            "cimmeria_base_world_entry::base::world_entry::map_loaded",
            "file:world_entry.log",
        ),
        (
            "cimmeria_base_world_entry::base::world_entry::reanchor_player",
            "file:world_entry.log",
        ),
        (
            "cimmeria_base_world_entry::base::world_entry::teleport",
            "file:world_entry.log",
        ),
        (
            "cimmeria_base_world_entry::base::world_entry_appearance::client_ready",
            "file:world_entry.log",
        ),
        (
            "cimmeria_base_world_entry::base::world_entry_appearance::cinematic",
            "file:world_entry.log",
        ),
        (
            "cimmeria_base_world_entry::base::world_entry_appearance::cinematic_aoi_hold",
            "file:world_entry.log",
        ),
        (
            "cimmeria_base_world_entry::base::character",
            "file:character.log",
        ),
    ] {
        let sinks = |lvl| sinks_for(&dispatch, &hits, target, lvl);
        assert_eq!(
            sinks(Level::TRACE),
            set(&[file, OTLP_TRACE]),
            "{target} at TRACE"
        );
        assert_eq!(
            sinks(Level::DEBUG),
            set(&[file, OTLP_SERVER]),
            "{target} at DEBUG"
        );
        for lvl in [Level::INFO, Level::WARN, Level::ERROR] {
            assert_eq!(
                sinks(lvl),
                set(&[file, SERVER_LOG, OTLP_SERVER]),
                "{target} at {lvl}"
            );
        }
    }
    // The character creator, `cimmeria-base` since wave B4, keeps its own
    // `character.log` row; the moved `character` row must not have been its
    // only route there.
    let create = "cimmeria_base::base::character_create";
    assert!(
        sinks_for(&dispatch, &hits, create, Level::DEBUG).contains("file:character.log"),
        "{create} must stay in character.log"
    );
    // Hand-named targets: no file, `server.log` from INFO, one index.
    assert_eq!(
        sinks_for(&dispatch, &hits, "aoi.cinematic_hold", Level::INFO),
        set(&[SERVER_LOG, OTLP_SERVER]),
        "aoi.cinematic_hold at INFO"
    );
    assert_eq!(
        sinks_for(&dispatch, &hits, "aoi.create_emit", Level::DEBUG),
        set(&[OTLP_SERVER]),
        "aoi.create_emit at DEBUG"
    );
}

/// `BaseService`, the connect loop, login, the SGWPlayer base-method dispatch
/// and the character creator moved from
/// `cimmeria_services::base::{service, connect_loop, login, dispatch,
/// character_create}` to the `cimmeria-base` crate (services crate split, wave
/// B4), which changed the `module_path!()` of every row in them (none has a
/// hand-named target). They must still land where they did: the service, the
/// connect loop and login in `base.log`, the dispatch in `dispatch.log`, the
/// character creator in `character.log`, at every level, in `server.log` from
/// INFO, and in one OTLP index per level. The connect loop's encrypted-bundle
/// scanner and cell-method arms fire per datagram, so below WARN they keep the
/// `cimmeria-network` index (`otel::is_network_noise_target`); everything else
/// keeps `cimmeria-server`. `cimmeria_services=debug` does not prefix-match
/// `cimmeria_base`, so without the `cimmeria_base::base=debug` row the DEBUG
/// rows would silently stop reaching SigNoz, and a file row still naming
/// `cimmeria_services::base::…` would empty the file of them.
///
/// The new row names `cimmeria_base::base`, not the crate, so that it does not
/// prefix-match the other base crates: each of their own rows must still be
/// the only thing that exports their DEBUG rows, or the guards above for
/// those crates could no longer fail.
#[test]
fn base_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for (target, file, noise) in [
        ("cimmeria_base::base::service", "file:base.log", false),
        ("cimmeria_base::base::connect_loop", "file:base.log", false),
        (
            "cimmeria_base::base::connect_loop::account_arms",
            "file:base.log",
            false,
        ),
        (
            "cimmeria_base::base::connect_loop::encrypted",
            "file:base.log",
            true,
        ),
        (
            "cimmeria_base::base::connect_loop::cell_arms",
            "file:base.log",
            true,
        ),
        ("cimmeria_base::base::login", "file:base.log", false),
        ("cimmeria_base::base::dispatch", "file:dispatch.log", false),
        (
            "cimmeria_base::base::dispatch::chat",
            "file:dispatch.log",
            false,
        ),
        (
            "cimmeria_base::base::dispatch::session",
            "file:dispatch.log",
            false,
        ),
        (
            "cimmeria_base::base::character_create",
            "file:character.log",
            false,
        ),
    ] {
        let sinks = |lvl| sinks_for(&dispatch, &hits, target, lvl);
        let below_warn = if noise { OTLP_NETWORK } else { OTLP_SERVER };
        assert_eq!(
            sinks(Level::TRACE),
            set(&[file, OTLP_TRACE]),
            "{target} at TRACE"
        );
        assert_eq!(
            sinks(Level::DEBUG),
            set(&[file, below_warn]),
            "{target} at DEBUG"
        );
        assert_eq!(
            sinks(Level::INFO),
            set(&[file, SERVER_LOG, below_warn]),
            "{target} at INFO"
        );
        for lvl in [Level::WARN, Level::ERROR] {
            assert_eq!(
                sinks(lvl),
                set(&[file, SERVER_LOG, OTLP_SERVER]),
                "{target} at {lvl}"
            );
        }
    }

    for (row, sibling) in [
        (
            "cimmeria_base_session=debug,",
            "cimmeria_base_session::base::outbox",
        ),
        (
            "cimmeria_base_methods=debug,",
            "cimmeria_base_methods::base::world_entry::methods::inventory",
        ),
        (
            "cimmeria_base_world_entry=debug,",
            "cimmeria_base_world_entry::base::world_entry::teleport",
        ),
    ] {
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
            sinks_for(&dispatch, &hits, sibling, Level::DEBUG).is_empty(),
            "{sibling}'s DEBUG export must depend on its own `{row}` row, not on \
             cimmeria-base's"
        );
    }
}
