//! World entry orchestration -- play_character (entity teardown), ENABLE_ENTITIES
//! (create player), mapLoaded (enter world), gate travel, and CellToBase message dispatch.
//!
//! Sub-concerns are split into sibling modules:
//! - `space_registry` -- world_name -> space_id table populated by CellService SpaceData.
//! - `play_character` -- `playCharacter` -> RESET_ENTITIES teardown.
//! - `enable_entities` -- ENABLE_ENTITIES dispatch (char list vs create-player).
//! - `map_loaded` -- mapLoaded -> VIEWPORT + CELL + entity-data enter-world.
//! - `gate_travel` -- gate transitions (replays world entry against a new space).
//! - `cell_dispatch` -- CellToBaseMsg fan-out to per-feature handlers.
//! - `methods` -- DB queries and feature handlers (player load, inventory, vendor,
//!   mail, missions, progression). Renamed from `world_entry_methods` for parity
//!   with this directory's naming.

pub(crate) mod methods;

mod cell_dispatch;
mod enable_entities;
mod gate_travel;
mod map_loaded;
mod play_character;
mod reanchor_player;
pub(crate) mod space_registry;
mod teleport;

// Public surface (re-exports keep `super::world_entry::handle_*` imports working
// in connect_loop.rs and friends).
pub(crate) use cell_dispatch::handle_cell_message;
pub(crate) use enable_entities::handle_enable_entities;
pub(crate) use map_loaded::handle_map_loaded;
pub(crate) use play_character::handle_play_character;

// Legacy re-exports from world_entry_appearance (kept here so connect_loop.rs's
// existing `super::world_entry::{handle_on_client_ready, handle_cancel_movie}`
// imports stay unchanged after this refactor).
pub(crate) use super::world_entry_appearance::{handle_cancel_movie, handle_on_client_ready};

#[cfg(test)]
mod tests;
