//! # cimmeria-cell-pets
//!
//! The pets feature as a cell plugin: the pilot of
//! `docs/architecture/plugin-architecture.md` (#962).
//!
//! - [`PetsPlugin`] registers the pet cell methods (88-90), the pet tick
//!   hooks and the base-destroy hook with the cell at startup.
//! - [`cell::cell_methods::player::pet`] holds the owner's pet commands
//!   (pets campaign PT-04), moved from `cimmeria-cell-methods` with their
//!   tests. The module tree keeps its old nesting, so the moved code's
//!   `crate::cell::…` and `super::…` paths are unchanged.
//!
//! Nothing depends on this crate but the composition root
//! (`cimmeria-services`, which lists it in the plugin table) and test code,
//! so an edit here rebuilds this crate, the facade and the binaries only.
//! The pet world half (`cimmeria_cell_world::cell::pets`) and the pet AI and
//! owner abilities (`cimmeria-cell-combat`) stay below: combat, content,
//! interactions and the console call them (ADR §3.8).

#![warn(unreachable_pub)]

pub mod cell;
mod plugin;

pub use plugin::PetsPlugin;

// Lower crates, at the crate-root paths the moved code names them by.
pub(crate) use cimmeria_wire::mercury;

// Generic helpers come from `cimmeria-test-support` (a dev-dependency) and the
// world fixtures from `cimmeria_cell_world::test_fixtures`, so the moved tests
// keep importing both from `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_cell_world::test_fixtures::*;
    pub(crate) use cimmeria_test_support::*;
}

#[cfg(test)]
mod plugin_tests;
