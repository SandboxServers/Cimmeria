//! # cimmeria-cell-methods
//!
//! The client-callable cell methods: the dispatchers that decode an exposed
//! CellMethod call from the client and hand it to the system that owns it.
//!
//! - [`cell::cell_methods`]: one dispatcher per BigWorld interface (being,
//!   ability manager, combatant, inventory, missionary, contact list,
//!   organization, mail, minigame, black market, gate travel) and the
//!   SGWPlayer's own methods under `player` (combat and respawn, interaction
//!   and dialogs, trade, vendors and trainers, world, crafting and social).
//!
//! The native GM cell methods (index 109 and up) are not here: they are
//! `cell::console::gm` in `cimmeria-cell-console`, which sits beside this crate
//! rather than above or below it.
//!
//! Split out of `cimmeria-services` (wave C5a of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its old
//! nesting, so `crate::cell::…` and `super::…` paths inside it are unchanged,
//! and `cimmeria-services` re-exports `cell::cell_methods` at its old path.

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
}
