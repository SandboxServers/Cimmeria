//! # cimmeria-services
//!
//! Server service implementations for the Cimmeria server emulator.
//!
//! Contains the three core services (Auth, Base, Cell), a database connection
//! pool, and an orchestrator that manages service lifecycle. This mirrors the
//! original C++ multi-process architecture where AuthenticationServer, BaseApp,
//! and CellApp ran as separate services communicating over Mercury.

pub mod audit;
pub mod auth;
pub mod base;
pub mod cell;
pub mod database;
pub mod mercury;
pub mod orchestrator;
