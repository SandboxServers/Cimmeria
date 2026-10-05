//! # cimmeria-cell-chatter
//!
//! Ambient chatter as a cell plugin (`docs/architecture/plugin-architecture.md`,
//! #962): groups of NPCs who talk among themselves in say chat on a schedule.
//! The first group is the Debug Area's System Lords' summit (DA-09,
//! `docs/content/debug-area.md#system-lords-summit`).
//!
//! - [`ChatterPlugin`] registers one tick hook with the cell at startup.
//! - [`cell::chatter`] holds the tick: each group's schedule
//!   ([`cell::chatter::schedule`]) and the line delivery
//!   ([`cell::chatter::speak`]).
//!
//! The groups and their lines are seed data
//! (`resources.ambient_chatter_groups` / `ambient_chatter_lines`), loaded by
//! the cell's startup into `SpaceManager::resources` as an
//! `AmbientChatterCatalog`. A line reaches the client as
//! `onPlayerCommunication(speaker, 0, CHAN_say, text)`, the route the
//! `npc_bark` content action and player say chat already use, so no client
//! patch is involved.
//!
//! Nothing depends on this crate but the composition root
//! (`cimmeria-services`, which lists it in the plugin table) and test code.

#![warn(unreachable_pub)]

pub mod cell;
mod plugin;

pub use plugin::ChatterPlugin;

#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_cell_world::test_fixtures::*;
    pub(crate) use cimmeria_test_support::*;
}
