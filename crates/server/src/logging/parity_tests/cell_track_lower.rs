//! The lower half of the services crate split's cell track (waves W2b and
//! C1-C3): the world state in `cimmeria-cell-world`, combat and the NPC AI in
//! `cimmeria-cell-combat`, and the content layer in `cimmeria-cell-content`.
//! Each move changed the moved events' `module_path!()`; these guards pin
//! that each still lands in the file and the OTLP index it did before.

use std::collections::BTreeSet;

use tracing::Level;

use super::{harness, sinks_for, LEVELS, OTLP_SERVER, OTLP_TRACE, SERVER_LOG};
use crate::logging::filters::FILE_LAYERS;

/// `spawn_npcs_from_records` and `spawn_instance_npcs_from_records` moved
/// from `cell::spawner::npcs` to `cell::space_manager::npc_population`
/// (services crate split, wave W2b). `aoi.log` keeps every
/// `cell::space_manager` module, so the move alone would have re-routed the
/// spawn rows there; they must stay in `spawner.log` and only there, while
/// the rest of `space_manager` keeps `aoi.log`.
#[test]
fn npc_population_events_keep_spawner_log() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let population = "cimmeria_cell_world::cell::space_manager::npc_population";
    let spawn = "cimmeria_cell_world::cell::space_manager::spawn";
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

/// The cell's world state moved from `cimmeria_services::cell` to the
/// `cimmeria-cell-world` crate (services crate split, wave C1), which changed
/// the `module_path!()` of every untargeted row in it. Each moved module must
/// still land where it did: in its own file at every level (`aoi.log` for
/// `space_manager` and the NPC AI's state primitives under `cell::service`,
/// `combat.log` for the world half of combat, `spawner.log` for the ring
/// FSM, `dispatch.log` for the GM gate), in `server.log` from INFO, and in
/// one OTLP index per level. The effect scripts, the cover stance, arrival
/// and the playtest friction watch have no file and keep `server.log` plus
/// their index. `cimmeria_services=debug` does not prefix-match
/// `cimmeria_cell_world`, so without its own `OTEL_FILTER` row every DEBUG
/// row here would silently stop reaching SigNoz; without the file rows the
/// files would silently empty of them.
#[test]
fn world_crate_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for (target, file) in [
        (
            "cimmeria_cell_world::cell::space_manager::entities",
            "aoi.log",
        ),
        (
            "cimmeria_cell_world::cell::service::npc_ai::detectors::sweep",
            "aoi.log",
        ),
        (
            "cimmeria_cell_world::cell::service::npc_ai::transition",
            "aoi.log",
        ),
        (
            "cimmeria_cell_world::cell::combat::aggression",
            "combat.log",
        ),
        (
            "cimmeria_cell_world::cell::ring_transport::transporter::manager",
            "spawner.log",
        ),
        (
            "cimmeria_cell_world::cell::ring_transport::runtime::teardown",
            "spawner.log",
        ),
        (
            "cimmeria_cell_world::cell::dispatch::gm_gate",
            "dispatch.log",
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
        "cimmeria_cell_world::cell::effects::passives",
        "cimmeria_cell_world::cell::cover::stance",
        "cimmeria_cell_world::cell::arrival",
        "cimmeria_cell_world::cell::playtest_friction_watch",
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

/// Combat and the NPC AI's behaviour moved from `cimmeria_services::cell` to
/// the `cimmeria-cell-combat` crate (services crate split, wave C2), which
/// changed the `module_path!()` of every untargeted row in them. Each must
/// still land where it did: `combat.log` for `combat` and `abilities`,
/// `aoi.log` for the NPC AI under `cell::service` (the services
/// `cell::service=trace` row used to keep it), `server.log` from INFO, and
/// one OTLP index per level. The effect pulsing and the bandolier, reload and
/// item-sequence handlers have no file and keep `server.log` plus their
/// index. `cimmeria_services=debug` does not prefix-match
/// `cimmeria_cell_combat`, so without its own `OTEL_FILTER` row every DEBUG
/// row here would stop reaching SigNoz; without the file rows the two files
/// would empty of them.
#[test]
fn combat_crate_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for (target, file) in [
        ("cimmeria_cell_combat::cell::combat::damage", "combat.log"),
        (
            "cimmeria_cell_combat::cell::combat::threat::aggro",
            "combat.log",
        ),
        (
            "cimmeria_cell_combat::cell::abilities::use_ability::handle",
            "combat.log",
        ),
        ("cimmeria_cell_combat::cell::abilities::death", "combat.log"),
        (
            "cimmeria_cell_combat::cell::service::npc_ai::dispatch",
            "aoi.log",
        ),
        (
            "cimmeria_cell_combat::cell::service::npc_ai::fight_cover",
            "aoi.log",
        ),
        (
            "cimmeria_cell_combat::cell::service::npc_ai::leash::begin",
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
        "cimmeria_cell_combat::cell::effects::pulsing::tick",
        "cimmeria_cell_combat::cell::cell_methods::inventory::bandolier::active_slot",
        "cimmeria_cell_combat::cell::cell_methods::player::world::reload",
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

/// The content layer moved from `cimmeria_services::cell` to the
/// `cimmeria-cell-content` crate (services crate split, wave C3), which
/// changed the `module_path!()` of every untargeted row in it. Each must
/// still land where it did: `content.log` for the chain executor and its
/// dispatchers, `missions.log` for missions, `spawner.log` for the ring
/// dispatcher and entry points (beside the ring FSM's world-crate rows),
/// `interactions.log` for the dialog display (beside the interaction
/// handlers that stay in services), `server.log` from INFO, and one OTLP
/// index per level. `cimmeria_services=debug` does not prefix-match
/// `cimmeria_cell_content`, so without its own `OTEL_FILTER` row every DEBUG
/// row here would stop reaching SigNoz; without the file rows the four files
/// would empty of them.
#[test]
fn content_crate_events_keep_their_file_and_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for (target, file) in [
        (
            "cimmeria_cell_content::cell::content::engine_loader",
            "content.log",
        ),
        (
            "cimmeria_cell_content::cell::content::event_dispatch::cover",
            "content.log",
        ),
        (
            "cimmeria_cell_content::cell::content::executor::dialog",
            "content.log",
        ),
        (
            "cimmeria_cell_content::cell::missions::progression",
            "missions.log",
        ),
        (
            "cimmeria_cell_content::cell::missions::lifecycle",
            "missions.log",
        ),
        (
            "cimmeria_cell_content::cell::ring_transport::dispatch",
            "spawner.log",
        ),
        (
            "cimmeria_cell_content::cell::ring_transport::runtime::entry",
            "spawner.log",
        ),
        (
            "cimmeria_cell_content::cell::interactions::dialog",
            "interactions.log",
        ),
        // The interaction handlers (in cimmeria-cell-interactions since wave
        // C4); the moved dialog row must not have been their only route to
        // the file.
        (
            "cimmeria_cell_interactions::cell::interactions::dispatch::interact",
            "interactions.log",
        ),
        // The ring FSM stayed in the world crate and keeps its own row.
        (
            "cimmeria_cell_world::cell::ring_transport::transporter::manager",
            "spawner.log",
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
}

/// The effect scripts moved from `cimmeria_cell_world::cell::effects` to the
/// `cimmeria-cell-effect-scripts` crate (#962 step 4), which changed the
/// `module_path!()` of any untargeted row in them. No file layer named them
/// before the move and none names them now; the crate's own row keeps DEBUG
/// reaching SigNoz and INFO `server.log`, as `cimmeria_cell_world=debug` did.
/// The runtime that stayed (`cell::effects::passives`) is pinned above.
#[test]
fn effect_scripts_crate_events_keep_their_index() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    for target in [
        "cimmeria_cell_effect_scripts::cell::effects::scripts",
        "cimmeria_cell_effect_scripts::cell::effects::pet_scripts",
        "cimmeria_cell_effect_scripts::cell::effects::ammo_emp",
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
