//! # cimmeria-entity
//!
//! Entity system runtime for the Cimmeria server emulator. Provides the core
//! entity abstractions used by both BaseApp and CellApp: entity lifecycle,
//! property storage and synchronisation, spatial partitioning, mailbox-based
//! RPC dispatch, movement controllers, and navigation mesh queries.
//!
//! This crate models the BigWorld entity architecture where each entity has a
//! "base" half (persistent state, managed by BaseApp) and an optional "cell"
//! half (spatial state, managed by CellApp). Communication between halves and
//! with clients uses typed mailboxes that serialize calls over Mercury.

pub mod abilities;
pub mod base_entity;
pub mod cell_entity;
pub mod detour_ffi;
pub mod interaction_flags;
pub mod inventory;
pub mod mailbox;
pub mod manager;
pub mod missions;
pub mod movement;
pub mod navigation;
pub mod properties;
pub mod space;
pub mod stats;
pub mod world_grid;
