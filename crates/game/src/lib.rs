//! # cimmeria-game
//!
//! Game logic for the Cimmeria server emulator. This crate replaces the Python
//! scripting layer with native Rust implementations of player, NPC, mob, combat,
//! inventory, mission, social, world, and command systems.
//!
//! Interaction handlers (vendor, lootable, stargate, trainer) live in
//! `cimmeria-services` — see `crates/services/src/base/world_entry/methods/`
//! and `crates/services/src/cell/{interactions,ring_transport}/`. The stub
//! `interactions/` module that previously lived here was deleted in
//! `cleanup/dead-interaction-stubs` after the audits on #52, #58, #59
//! confirmed zero callers.

pub mod being;
pub mod combat;
pub mod commands;
pub mod inventory;
pub mod missions;
pub mod npc;
pub mod player;
pub mod social;
pub mod world;
