//! # cimmeria-game
//!
//! Game logic for the Cimmeria server emulator. This crate replaces the Python
//! scripting layer with native Rust implementations of player, NPC, mob, combat,
//! inventory, mission, social, world, and command systems.
//!
//! Interaction handlers (vendor, lootable, stargate, trainer) live in
//! `cimmeria-services` — see `crates/services/src/base/world_entry/methods/`
//! and `crates/services/src/cell/{interactions,ring_transport}/`. Vendor,
//! lootable, and stargate stubs that previously lived under
//! `cimmeria-game::interactions::*` were deleted as dead code after audits
//! confirmed zero callers across the workspace. Those stubs were originally
//! created as part of a "real implementation lives here, eventually" pattern
//! that did not pan out — the real handlers landed in `cimmeria-services`
//! instead, and the empty placeholders just produced two parallel surfaces
//! that confused triage. The trainer stub was retired earlier on the same
//! grounds.

pub mod being;
pub mod combat;
pub mod commands;
pub mod inventory;
pub mod missions;
pub mod npc;
pub mod player;
pub mod social;
pub mod world;
