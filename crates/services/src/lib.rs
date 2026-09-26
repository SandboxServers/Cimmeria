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
pub mod firehose;
pub mod mercury;
pub mod minigame;
pub mod orchestrator;
mod orchestrator_postgres;
mod orchestrator_shards;
pub mod wire_log;

// Split out to `cimmeria-auth` (wave W1a of
// docs/architecture/services-crate-split.md). Re-exported at the old paths so
// `crate::auth::…` here and `cimmeria_services::{auth, audit}` downstream keep
// resolving. `credential_redaction` was crate-private and stays so here.
pub(crate) use cimmeria_auth::credential_redaction;
pub use cimmeria_auth::{audit, auth};

// Split out to `cimmeria-cell-catalog` (wave W2b), with `cell::spawner` and
// `cell::respawner_fallback`; re-exported at the old path.
pub use cimmeria_cell_catalog::ability_tree;

// Generic helpers come from `cimmeria-test-support` (a dev-dependency) and
// are re-exported from this module next to the crate's own fixtures.
#[cfg(test)]
pub(crate) mod test_support;

// Guards `tools/test-live-db.{sh,ps1}`: every crate with a
// `cimmeria-test-support` dev-dependency must be in the live-DB crate list.
#[cfg(test)]
mod live_db_wrapper_tests;
