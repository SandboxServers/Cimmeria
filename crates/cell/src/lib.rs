//! # cimmeria-cell
//!
//! The CellApp service, under `cell::`:
//!
//! - [`cell::CellService`]: the service's lifecycle. `start()` loads the
//!   startup caches and spawns the cell loop; `stop()` signals it and joins it.
//! - `cell::service`: the cell loop (`message_loop`), which drains the
//!   BaseApp's messages and runs the per-frame ticks (AoI, NPC movement and
//!   respawn, regen, reload and holster promotion, cover detection, the
//!   auto-cycle), and the per-message handlers under `base_messages` (entity
//!   lifecycle, movement, the login-time player state, ability grants,
//!   inventory and bandolier events, GM spawns, the lab console and query).
//! - [`cell::dispatch`]: the router that maps a client's flattened cell-method
//!   index to the per-interface dispatchers in `cimmeria-cell-methods` and the
//!   native GM tail in `cimmeria-cell-console`, behind the GM gate.
//!
//! Split out of `cimmeria-services` (wave C6 of
//! `docs/architecture/services-crate-split.md`), the top of the cell track.
//! The module tree keeps its old nesting, so `crate::cell::…` and `super::…`
//! paths inside it are unchanged, and `cimmeria-services` re-exports
//! `cell::dispatch` and `cell::CellService` at their old paths, where the
//! orchestrator builds the service. The base is reached only through the
//! Base<->Cell messages in `cimmeria-wire`.

#![warn(unreachable_pub)]

pub mod cell;

// Lower crates, at the crate-root paths the moved code names them by.
pub(crate) use cimmeria_cell_catalog::ability_tree;
pub(crate) use cimmeria_wire::mercury;

// Generic helpers come from `cimmeria-test-support` (a dev-dependency) and the
// world fixtures from `cimmeria_cell_world::test_fixtures`, so the moved tests
// keep importing both from `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_cell_world::test_fixtures::*;
    pub(crate) use cimmeria_test_support::*;
    // The effect scripts (#962 step 4): tests that dispatch one install the
    // registry on the manager they build, as the cell does at startup.
    pub(crate) use cimmeria_cell_effect_scripts::cell::effects::registry::install as install_effect_scripts;
}
