//! # cimmeria-game
//!
//! Game logic for the Cimmeria server emulator. This crate replaces the Python
//! scripting layer with native Rust implementations of player, NPC, mob, combat,
//! inventory, mission, social, interaction, world, and command systems.

pub mod player;
pub mod being;
pub mod mob;
pub mod npc;
pub mod combat;
pub mod inventory;
pub mod missions;
pub mod social;
pub mod interactions;
pub mod world;
pub mod commands;
