//! # cimmeria-services
//!
//! Server service implementations for the Cimmeria server emulator.
//!
//! Contains the three core services (Auth, Base, Cell), a database connection
//! pool, and an orchestrator that manages service lifecycle. This mirrors the
//! original C++ multi-process architecture where AuthenticationServer, BaseApp,
//! and CellApp ran as separate services communicating over Mercury.

pub mod base;
pub mod cell;
pub mod database;
pub mod orchestrator;
mod orchestrator_postgres;
mod orchestrator_shards;

// Split out to `cimmeria-auth` (wave W1a of
// docs/architecture/services-crate-split.md). Re-exported at the old paths so
// `crate::auth::…` here and `cimmeria_services::{auth, audit}` downstream keep
// resolving. `credential_redaction` was crate-private and stays so here.
pub(crate) use cimmeria_auth::credential_redaction;
pub use cimmeria_auth::{audit, auth};

// Split out to `cimmeria-cell-catalog` (wave W2b), with `cell::spawner` and
// `cell::respawner_fallback`; re-exported at the old path.
pub use cimmeria_cell_catalog::ability_tree;

// The server's Mercury message layer (the services-side packet builders) and
// the per-packet log firehoses, split out to `cimmeria-wire` (wave W3a).
// Re-exported at the old paths, so `crate::mercury::…` here and
// `cimmeria_services::{mercury, firehose}` downstream keep resolving.
pub use cimmeria_wire::{firehose, mercury};

// The decoded wire-message stream and the per-session packet tap, split out
// to `cimmeria-wire-log` (wave W3b). Re-exported at the old path, so
// `crate::wire_log::…` here and `cimmeria_services::wire_log::tap` downstream
// keep resolving.
pub use cimmeria_wire_log::wire_log;
// The in-process SmartFoxServer host for the Flash minigames, split out to
// `cimmeria-minigame` (wave W3c). Re-exported at the old path, so
// `crate::minigame::…` here (the orchestrator starts its server, the base
// registers tickets in its `SessionRegistry`) keeps resolving.
pub use cimmeria_minigame::minigame;

/// The `mercury::aoi` test that drives a `SpaceManager`, which is still in
/// this crate. Test-only.
#[cfg(test)]
mod mercury_aoi_tests;

/// The `sgw_mission` round-trip tests that drive the cell's missions and
/// the base's cell dispatch, both still in this crate, against the
/// feature handlers in `cimmeria-base-methods`. Test-only.
#[cfg(test)]
mod mission_round_trip_tests;

// Generic helpers come from `cimmeria-test-support` (a dev-dependency) and
// are re-exported from this module next to the crate's own fixtures.
#[cfg(test)]
pub(crate) mod test_support;

// Guards `tools/test-live-db.{sh,ps1}`: every crate with a
// `cimmeria-test-support` dev-dependency must be in the live-DB crate list.
#[cfg(test)]
mod live_db_wrapper_tests;
