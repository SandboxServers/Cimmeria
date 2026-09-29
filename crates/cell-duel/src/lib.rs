//! # cimmeria-cell-duel
//!
//! Duels as a cell plugin: step 2 of the plugin migration in
//! `docs/architecture/plugin-architecture.md` (#962).
//!
//! - [`DuelPlugin`] registers the duel cell methods (102 `sendDuelResponse`,
//!   103 `duelForfeit`), the duel tick and the disconnect, travel and death
//!   hooks with the cell at startup.
//! - [`cell::duel`] holds the answer ([`cell::duel::response`]), the
//!   forfeit ([`cell::duel::forfeit`]), the tick ([`cell::duel::tick`]) and
//!   the engage it runs, moved from `cimmeria-cell-world` with the duel
//!   tests. The module re-exports the duel world half, so the moved code's
//!   `super::…` and `crate::cell::duel::…` paths are unchanged.
//!
//! Nothing depends on this crate but the composition root
//! (`cimmeria-services`, which lists it in the plugin table) and test code,
//! so an edit here rebuilds this crate, the facade and the binaries only.
//! The duel world half (`cimmeria_cell_world::cell::duel`: the registry, the
//! challenge, the end paths, the non-lethal clamp, the GM commands) stays
//! below: combat, the AoI enter path, the console and the base-message
//! handler call it (ADR §3.8, §4.2).

#![warn(unreachable_pub)]

pub mod cell;
mod plugin;

pub use plugin::DuelPlugin;

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
