//! # cimmeria-services
//!
//! Server service implementations for the Cimmeria server emulator.
//!
//! Contains the three core services (Auth, Base, Cell), a database connection
//! pool, and an orchestrator that manages service lifecycle. This mirrors the
//! original C++ multi-process architecture where AuthenticationServer, BaseApp,
//! and CellApp ran as separate services communicating over Mercury.

pub mod ability_tree;
pub mod audit;
pub mod auth;
pub mod base;
pub mod cell;
pub(crate) mod credential_redaction;
pub mod database;
pub mod firehose;
pub mod mercury;
pub mod minigame;
pub mod orchestrator;
mod orchestrator_postgres;
mod orchestrator_shards;
pub mod wire_log;

// Generic helpers come from `cimmeria-test-support` (a dev-dependency) and
// are re-exported from this module next to the crate's own fixtures.
#[cfg(test)]
pub(crate) mod test_support;

// Guards `tools/test-live-db.{sh,ps1}`: every crate with a
// `cimmeria-test-support` dev-dependency must be in the live-DB crate list.
#[cfg(test)]
mod live_db_wrapper_tests;
