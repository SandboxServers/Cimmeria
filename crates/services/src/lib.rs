//! # cimmeria-services
//!
//! The facade over the Cimmeria service crates. It owns two things:
//!
//! - [`orchestrator`]: the `Orchestrator` that starts the auth, base and cell
//!   services in order in one process, shares the server state with the admin
//!   API and the lab endpoint, and stops them in reverse. The original C++
//!   server ran AuthenticationServer, BaseApp and CellApp as separate
//!   processes talking over Mercury.
//! - [`database`]: the PostgreSQL pool and the lab's read-only query.
//!
//! Everything else is a re-export. The services themselves live in their own
//! crates since the services crate split
//! (`docs/architecture/services-crate-split.md`): `cimmeria-auth`,
//! `cimmeria-base` and the base crates below it, `cimmeria-cell` and the cell
//! crates below it, `cimmeria-minigame`, and the shared `cimmeria-wire`
//! contract. This crate re-exports them at the paths they had when they were
//! modules here (`cimmeria_services::{auth, base, cell, mercury, …}`), which
//! the server, the admin API, the lab endpoint and the wire client import;
//! those crates depend only on this one. Its own tests are the
//! orchestrator's and the pool's, and the round trips that drive both the
//! cell and the base (`gate_round_trip_tests`, `mission_round_trip_tests`),
//! which no crate below this one can reach.

pub mod base;
pub mod cell;
pub mod database;
pub mod orchestrator;
mod orchestrator_postgres;
mod orchestrator_shards;

// Split out to `cimmeria-auth` (wave W1a of
// docs/architecture/services-crate-split.md). Re-exported at the old paths so
// `crate::auth::…` here and `cimmeria_services::{auth, audit}` downstream keep
// resolving. The crate-private `credential_redaction` re-export is gone: its
// last users here, the connect loop and login, moved to `cimmeria-base` (B4).
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

/// The gate-travel round trips that drive the cell's gate handlers
/// (`cimmeria-cell-interactions`) and then the base's world entry
/// (`cimmeria-base-world-entry`). Test-only.
#[cfg(test)]
mod gate_round_trip_tests;

/// The `sgw_mission` round-trip tests that drive the cell's missions
/// (`cimmeria-cell-content`) and the base's cell dispatch
/// (`cimmeria-base-world-entry`) against the feature handlers in
/// `cimmeria-base-methods`. Test-only.
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
