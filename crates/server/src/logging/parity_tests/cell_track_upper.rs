//! The upper half of the services crate split's cell track: the C4-C6
//! preparation moves, then `cimmeria-cell-interactions` (C4),
//! `cimmeria-cell-console` (C5b), `cimmeria-cell-methods` (C5a) and
//! `cimmeria-cell` (C6). Each move changed the moved events'
//! `module_path!()`; these guards pin that each still lands in the file and
//! the OTLP index it did before, and that no crate's `OTEL_FILTER` row covers
//! a sibling crate by prefix.

use std::collections::BTreeSet;
use std::sync::Arc;

use tracing::{Dispatch, Level};
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::EnvFilter;

use super::{harness, recorder, sinks_for, Hits, OTLP_SERVER, OTLP_TRACE, SERVER_LOG};
use crate::logging::filters::{FILE_LAYERS, OTEL_FILTER};

/// The C4-C6 preparation of the services split moved cell modules inside
/// `cimmeria-services`, which changed their `module_path!()`:
///
/// - chat is `cell::console::chat` (was `cell::chat`), and keeps
///   `interactions.log`;
/// - the client-cache resync and the hotbar seed are `cell::respawn::resync`
///   (were under `cell::service::base_messages::player_init`, which
///   `aoi.log`'s `cell::service` row kept), and keep `aoi.log`;
/// - the respawn fork, the region registration, the trade session state and
///   the GM handlers had no file before and have none now. The trade state is
///   `cell::trade`, not under `cell::interactions`, whose row would have put
///   it in `interactions.log`.
///
/// Every one keeps `server.log` from INFO and one OTLP index per level. The
/// chat and resync rows fail this with the old file rows; the trade row fails
/// it if the state moves under `cell::interactions`. Wave C4 moved the resync,
/// the fork, the region registration and the trade state on to
/// `cimmeria-cell-interactions`, and wave C5b moved chat and the GM handlers
/// on to `cimmeria-cell-console`, so they are named at those crates' paths;
/// `interactions_crate_events_keep_their_file_and_index` and
/// `console_crate_events_keep_their_file_and_index` cover the rest of them.
#[test]
fn cell_prep_moves_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for (target, file) in [
        (
            "cimmeria_cell_console::cell::console::chat",
            "file:interactions.log",
        ),
        (
            "cimmeria_cell_interactions::cell::respawn::resync",
            "file:aoi.log",
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
    for target in [
        "cimmeria_cell_interactions::cell::respawn",
        "cimmeria_cell_interactions::cell::respawn::region_registration",
        "cimmeria_cell_interactions::cell::trade::state",
        "cimmeria_cell_interactions::cell::trade::wire",
        "cimmeria_cell_console::cell::console::gm::world",
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

/// The player interactions moved from `cimmeria_services::cell` to the
/// `cimmeria-cell-interactions` crate (services crate split, wave C4), which
/// changed the `module_path!()` of every untargeted row in them. Each must
/// still land where it did: `interactions.log` for the interaction handlers
/// and mail (beside the content crate's dialog display), `spawner.log` for
/// gate travel, `aoi.log` for the client-cache resync, `server.log` from INFO,
/// and one OTLP index per level. The space transfer, the respawn fork, the
/// region registration and the trade state had no file and have none; they
/// reach SigNoz from DEBUG. `cimmeria_services=debug` does not prefix-match
/// `cimmeria_cell_interactions`, so without its own `OTEL_FILTER` row every
/// DEBUG row here would stop reaching SigNoz; without the file rows the three
/// files would empty of them.
#[test]
fn interactions_crate_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for (target, file) in [
        (
            "cimmeria_cell_interactions::cell::interactions::dispatch::interact",
            "interactions.log",
        ),
        (
            "cimmeria_cell_interactions::cell::interactions::loot",
            "interactions.log",
        ),
        (
            "cimmeria_cell_interactions::cell::interactions::trainer",
            "interactions.log",
        ),
        ("cimmeria_cell_interactions::cell::mail", "interactions.log"),
        // The dialog display stayed in the content crate and keeps its own row.
        (
            "cimmeria_cell_content::cell::interactions::dialog",
            "interactions.log",
        ),
        (
            "cimmeria_cell_interactions::cell::gate_travel",
            "spawner.log",
        ),
        (
            "cimmeria_cell_interactions::cell::gate_travel::tick",
            "spawner.log",
        ),
        (
            "cimmeria_cell_interactions::cell::respawn::resync",
            "aoi.log",
        ),
    ] {
        let file = format!("file:{file}");
        let file = file.as_str();
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
    for target in [
        "cimmeria_cell_interactions::cell::space_transfer",
        "cimmeria_cell_interactions::cell::respawn",
        "cimmeria_cell_interactions::cell::respawn::region_registration",
        "cimmeria_cell_interactions::cell::trade::state",
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

/// The GM surfaces moved from `cimmeria_services::cell::console` to the
/// `cimmeria-cell-console` crate (services crate split, wave C5b), which
/// changed the `module_path!()` of every untargeted row in them. Each must
/// still land where it did: chat in `interactions.log` (beside the
/// interaction handlers and the dialog display), `server.log` from INFO, and
/// one OTLP index per level. The rest of the console (the dispatcher, the
/// command families, the #523 authoring commands) and the native GM handlers
/// had no file and have none; they reach SigNoz from DEBUG.
/// `cimmeria_services=debug` does not prefix-match `cimmeria_cell_console`, so
/// without its own `OTEL_FILTER` row every DEBUG row here would stop reaching
/// SigNoz; without the chat row `interactions.log` would lose chat.
///
/// The crate shares the `cimmeria_cell_co` prefix with the combat, content
/// and cover crates. Each of their rows must still be the only thing that
/// exports its own crate's DEBUG rows: if the console's row (or a shorter one
/// standing in for it) prefix-matched a sibling, removing the sibling's row
/// would change nothing, and that crate's guard could no longer fail.
#[test]
fn console_crate_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    let target = "cimmeria_cell_console::cell::console::chat";
    let file = "file:interactions.log";
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

    for target in [
        "cimmeria_cell_console::cell::console",
        "cimmeria_cell_console::cell::console::dispatch",
        "cimmeria_cell_console::cell::console::seed",
        "cimmeria_cell_console::cell::console::spawn::authoring",
        "cimmeria_cell_console::cell::console::travel",
        "cimmeria_cell_console::cell::console::gm",
        "cimmeria_cell_console::cell::console::gm::travel",
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

    for (row, sibling) in [
        (
            "cimmeria_cell_combat=debug,",
            "cimmeria_cell_combat::cell::abilities::use_ability",
        ),
        (
            "cimmeria_cell_content=debug,",
            "cimmeria_cell_content::cell::content::executor",
        ),
        (
            "cimmeria_cell_cover=debug,",
            "cimmeria_cell_cover::cell::cover",
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
             cimmeria-cell-console's"
        );
    }
}

/// The pet cell methods moved from `cimmeria-cell-methods` to the
/// `cimmeria-cell-pets` plugin crate (#962 pilot). Their rows name explicit
/// targets (`pets.command`), but an untargeted row there takes the new
/// module path, which `cimmeria_cell_methods=debug` does not prefix-match;
/// the crate's own row keeps DEBUG reaching SigNoz and INFO `server.log`.
#[test]
fn cell_pets_crate_events_keep_their_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for target in [
        "cimmeria_cell_pets::plugin",
        "cimmeria_cell_pets::cell::cell_methods::player::pet",
        "cimmeria_cell_pets::cell::cell_methods::player::pet::stance",
    ] {
        let sinks = |lvl| sinks_for(&dispatch, &hits, target, lvl);
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

/// The duel answer, forfeit, tick and engage moved from `cimmeria-cell-world`
/// to the `cimmeria-cell-duel` plugin crate (#962 step 2). Their rows name the
/// `duel` target, but an untargeted row there takes the new module path,
/// which `cimmeria_cell_world=debug` does not prefix-match; the crate's own
/// row keeps DEBUG reaching SigNoz and INFO `server.log`.
#[test]
fn cell_duel_crate_events_keep_their_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for target in [
        "cimmeria_cell_duel::plugin",
        "cimmeria_cell_duel::cell::duel::response",
        "cimmeria_cell_duel::cell::duel::tick",
    ] {
        let sinks = |lvl| sinks_for(&dispatch, &hits, target, lvl);
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

/// The organization router, the squad cell methods, the disconnect and
/// world-entry replay and the creation name check moved from
/// `cimmeria-cell-methods` to the `cimmeria-cell-org` plugin crate (#962
/// step 3). Most rows name the `org` / `squad` targets, but the untargeted
/// `org.create` span takes the new module path, which
/// `cimmeria_cell_methods=debug` does not prefix-match; the crate's own row
/// keeps DEBUG reaching SigNoz and INFO `server.log`. The half that stayed
/// below (`cimmeria_cell_interactions::cell::organization`) keeps its
/// `cimmeria_cell_interactions=debug` row and no file layer.
#[test]
fn cell_org_crate_events_keep_their_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for target in [
        "cimmeria_cell_org::plugin",
        "cimmeria_cell_org::cell::organization::creation",
        "cimmeria_cell_org::cell::organization::squad::membership",
        "cimmeria_cell_interactions::cell::organization::creation",
    ] {
        let sinks = |lvl| sinks_for(&dispatch, &hits, target, lvl);
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

/// The client-callable cell methods moved from
/// `cimmeria_services::cell::cell_methods` to the `cimmeria-cell-methods` crate
/// (services crate split, wave C5a), which changed the `module_path!()` of
/// every untargeted row in them. No file layer named them before the move and
/// none names them now: they reach `server.log` from INFO and one OTLP index
/// per level, `cimmeria-server` below TRACE (they log per cell-method call, not
/// per packet, so they are not network noise). `cimmeria_services=debug` does
/// not prefix-match `cimmeria_cell_methods`, so without its own `OTEL_FILTER`
/// row every DEBUG row here would stop reaching SigNoz. The second half pins
/// that no other crate's row covers the crate by prefix: with its own row
/// dropped, none of its DEBUG rows is exported.
#[test]
fn cell_methods_crate_events_keep_their_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    let targets = [
        "cimmeria_cell_methods::cell::cell_methods::being",
        "cimmeria_cell_methods::cell::cell_methods::contact_list",
        "cimmeria_cell_methods::cell::cell_methods::inventory::item_ops",
        "cimmeria_cell_methods::cell::cell_methods::missionary",
        "cimmeria_cell_methods::cell::cell_methods::player::combat",
        "cimmeria_cell_methods::cell::cell_methods::player::interaction::interact",
        "cimmeria_cell_methods::cell::cell_methods::player::trade::handlers",
        "cimmeria_cell_methods::cell::cell_methods::player::vendor::train",
        "cimmeria_cell_methods::cell::cell_methods::player::world",
    ];
    for target in targets {
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

    let row = "cimmeria_cell_methods=debug,";
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
    for target in targets {
        assert!(
            sinks_for(&dispatch, &hits, target, Level::DEBUG).is_empty(),
            "{target}'s DEBUG export must depend on its own `{row}` row"
        );
    }
}

/// The cell service moved from `cimmeria_services::cell::{service, dispatch}`
/// to the `cimmeria-cell` crate (services crate split, wave C6), which changed
/// the `module_path!()` of every untargeted row in it. Each must still land
/// where it did: the cell loop, the base-message handlers and the ticks in
/// `aoi.log` (the old `cimmeria_services::cell::service` row), the cell-method
/// router in `dispatch.log` (the old `cimmeria_services::cell::dispatch` row),
/// at every level, in `server.log` from INFO, and in one OTLP index per level,
/// `cimmeria-server` below TRACE (they log per message or per tick, not per
/// datagram, so they are not network noise). `cimmeria_services=debug` does
/// not prefix-match `cimmeria_cell`, so without the crate's own `OTEL_FILTER`
/// row every DEBUG row here would stop reaching SigNoz, and a file row still
/// naming `cimmeria_services::cell::…` would empty the file of them.
///
/// The row names `cimmeria_cell::cell`, never the bare crate: EnvFilter
/// matches by string prefix, so `cimmeria_cell=debug` would also match every
/// `cimmeria_cell_*` crate (the B4 rule for `cimmeria_base::base`). Each of
/// those crates' own rows must still be the only thing that exports its DEBUG
/// rows, or their guards could never fail again. The last block proves
/// this guard catches the bare row: under a filter with `cimmeria_cell=debug`
/// in place of the real row, dropping a sibling's row no longer stops its
/// export.
#[test]
fn cell_crate_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for (target, file) in [
        ("cimmeria_cell::cell::service", "file:aoi.log"),
        ("cimmeria_cell::cell::service::startup", "file:aoi.log"),
        ("cimmeria_cell::cell::service::message_loop", "file:aoi.log"),
        (
            "cimmeria_cell::cell::service::base_messages::lifecycle",
            "file:aoi.log",
        ),
        (
            "cimmeria_cell::cell::service::base_messages::player_init",
            "file:aoi.log",
        ),
        (
            "cimmeria_cell::cell::service::base_messages::player_init::mission_restore",
            "file:aoi.log",
        ),
        (
            "cimmeria_cell::cell::service::ticks::npc_movement",
            "file:aoi.log",
        ),
        (
            "cimmeria_cell::cell::service::ticks::npc_respawn",
            "file:aoi.log",
        ),
        ("cimmeria_cell::cell::dispatch", "file:dispatch.log"),
        ("cimmeria_cell::cell::dispatch::router", "file:dispatch.log"),
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

    let own_row = "cimmeria_cell::cell=debug,";
    assert!(
        OTEL_FILTER.contains(own_row),
        "OTEL_FILTER no longer carries `{own_row}`; update this test"
    );
    for directive in OTEL_FILTER.split(',').map(str::trim) {
        let target = directive.split_once('=').map_or(directive, |(t, _)| t);
        assert_ne!(
            target, "cimmeria_cell",
            "OTEL_FILTER carries a bare `{directive}`: it prefix-matches every \
             cimmeria_cell_* crate; name `cimmeria_cell::cell` instead"
        );
    }

    // The DEBUG export of `target` under `filter` with `row` dropped.
    let exported_without = |filter: &str, row: &str, target: &str| -> bool {
        let without = filter.replace(row, "");
        assert_ne!(
            without, filter,
            "the filter no longer carries `{row}`; update this test"
        );
        let hits: Hits = Arc::default();
        let dispatch = Dispatch::new(
            tracing_subscriber::registry()
                .with(recorder(OTLP_SERVER.into(), &hits).with_filter(EnvFilter::new(without))),
        );
        !sinks_for(&dispatch, &hits, target, Level::DEBUG).is_empty()
    };
    assert!(
        !exported_without(OTEL_FILTER, own_row, "cimmeria_cell::cell::service::ticks"),
        "the cell service's DEBUG export must depend on its own `{own_row}` row"
    );
    let siblings = [
        (
            "cimmeria_cell_world=debug,",
            "cimmeria_cell_world::cell::space_manager",
        ),
        (
            "cimmeria_cell_combat=debug,",
            "cimmeria_cell_combat::cell::abilities::use_ability",
        ),
        (
            "cimmeria_cell_content=debug,",
            "cimmeria_cell_content::cell::content::executor",
        ),
        (
            "cimmeria_cell_interactions=debug,",
            "cimmeria_cell_interactions::cell::gate_travel",
        ),
        (
            "cimmeria_cell_methods=debug,",
            "cimmeria_cell_methods::cell::cell_methods::player::combat",
        ),
        (
            "cimmeria_cell_pets=debug,",
            "cimmeria_cell_pets::cell::cell_methods::player::pet",
        ),
        (
            "cimmeria_cell_duel=debug,",
            "cimmeria_cell_duel::cell::duel::response",
        ),
        (
            "cimmeria_cell_org=debug,",
            "cimmeria_cell_org::cell::organization::creation",
        ),
        (
            "cimmeria_cell_console=debug,",
            "cimmeria_cell_console::cell::console::dispatch",
        ),
        (
            "cimmeria_cell_cover=debug,",
            "cimmeria_cell_cover::cell::cover",
        ),
        (
            "cimmeria_cell_catalog=debug,",
            "cimmeria_cell_catalog::cell::spawner",
        ),
    ];
    for (row, sibling) in siblings {
        assert!(
            !exported_without(OTEL_FILTER, row, sibling),
            "{sibling}'s DEBUG export must depend on its own `{row}` row, not on \
             cimmeria-cell's"
        );
    }

    // The same check under the bare crate-name row this test exists to reject:
    // it would shadow every sibling, so the loop above would fail on it.
    let bare = OTEL_FILTER.replace(own_row, "cimmeria_cell=debug,");
    for (row, sibling) in siblings {
        assert!(
            exported_without(&bare, row, sibling),
            "a bare `cimmeria_cell=debug` must shadow {sibling}'s own `{row}` row; \
             if it no longer does, this guard proves nothing"
        );
    }
}
