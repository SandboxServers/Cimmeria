//! # cimmeria-cell-org
//!
//! Squads and organization creation as a cell plugin: step 3 of the plugin
//! migration in `docs/architecture/plugin-architecture.md` (#962).
//!
//! - [`OrgPlugin`] registers the OrganizationMember cell methods (8-19) and
//!   `onOrganizationCreation` (94), the disconnect hook (a squad member
//!   leaves, an open registrar offer ends) and the world-entry hook (the
//!   squad is re-sent after a gate trip) with the cell at startup.
//! - [`cell::organization`] holds the router for 8-19, the Team and Command
//!   forward to the base, the squad invite answer, leave, loot mode and
//!   minimap ping, the squad disconnect and world-entry replay, and the
//!   creation name check, moved from `cimmeria-cell-methods` with the
//!   organization tests. The module re-exports the organization half that
//!   stayed below, so the moved code's `super::…` paths are unchanged.
//!
//! Nothing depends on this crate but the composition root
//! (`cimmeria-services`, which lists it in the plugin table) and test code,
//! so an edit here rebuilds this crate, the facade and the binaries only.
//! The half the lower crates call stays in
//! `cimmeria_cell_interactions::cell::organization`: the base-forwarded
//! squad invite and kick and the registrar reply and create result (the
//! base-message handler in `cimmeria-cell` calls them), the GM squad
//! commands (`cimmeria-cell-console` calls them), and the fanout, feedback
//! and telemetry everything shares. The registries stay in
//! `cimmeria-cell-world` (ADR §3.8, §4.3).

#![warn(unreachable_pub)]

pub mod cell;
mod plugin;

pub use plugin::OrgPlugin;

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
