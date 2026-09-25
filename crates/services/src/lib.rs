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

#[cfg(test)]
mod live_db_gate;
#[cfg(test)]
pub(crate) mod test_support;
