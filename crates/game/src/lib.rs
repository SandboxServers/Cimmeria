//! # cimmeria-game
//!
//! Game logic for the Cimmeria server emulator. This crate replaces the Python
//! scripting layer with native Rust implementations of player, NPC, mob, combat,
//! inventory, mission, social, interaction, world, and command systems.

pub mod being;
pub mod combat;
pub mod commands;
pub mod interactions;
pub mod inventory;
pub mod missions;
pub mod npc;
pub mod player;
pub mod social;
pub mod world;
