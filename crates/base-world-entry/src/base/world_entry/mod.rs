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
//!   trade, mail, missions, progression), in `cimmeria-base-methods` since wave
//!   B2 of docs/architecture/services-crate-split.md.
//!
//! This module is in `cimmeria-base-world-entry` since wave B3;
//! `cimmeria-services` re-exports it at its old path.

// The feature handlers are in `cimmeria-base-methods` (wave B2), re-exported
// at their old path, so `super::methods::…` in the siblings is unchanged.
// `pub`: tests in `cimmeria-services` reach them through this path.
pub use cimmeria_base_methods::base::world_entry::methods;

pub(crate) mod cell_dispatch;
mod enable_entities;
mod gate_travel;
pub(crate) mod looted_containers;
mod map_loaded;
mod map_loaded_wire_rows;
mod play_character;
mod reanchor_player;
mod teleport;

#[cfg(test)]
mod crafting_options_world_entry_tests;
/// SS-00: the online name index across world entry and reanchor.
#[cfg(test)]
mod player_index_lifecycle_tests;

// The space registry is in `cimmeria-base-session` (wave B1): base-methods
// resolves world names through it from below world entry.
pub(crate) use cimmeria_base_session::base::world_entry::space_registry;

// Public surface: the connect loop and `BaseService`, in `cimmeria-base`,
// import these through `super::world_entry::handle_*`.
#[cfg(any(test, feature = "test-support"))]
pub use cell_dispatch::handle_cell_message;
pub use cell_dispatch::route_cell_message;
pub use enable_entities::handle_enable_entities;
pub use map_loaded::handle_map_loaded;
pub use play_character::handle_play_character;

// Legacy re-exports from world_entry_appearance (kept here so connect_loop.rs's
// existing `super::world_entry::{handle_on_client_ready, handle_cancel_movie}`
// imports stay unchanged after this refactor).
pub use super::world_entry_appearance::{handle_cancel_movie, handle_on_client_ready};

// Test hooks: the gate round trips in `cimmeria-services`
// (`gate_round_trip_tests`) drive the cell's dial handler before these, so they
// stay there and reach these through the `test-support` feature.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use gate_travel::{handle_gate_travel, persist_arrival};
