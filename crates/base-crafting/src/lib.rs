//! # cimmeria-base-crafting
//!
//! Crafting as a base plugin: step 5 of the plugin migration in
//! `docs/architecture/plugin-architecture.md` (#962, §4.5).
//!
//! - [`CraftingPlugin`] registers crafting with the base at startup: the
//!   consumers of the crafting payloads the cell sends in the
//!   `CellToBaseMsg::Plugin` envelope (the verbs, the station reports, the
//!   GM grants and the respec opener), the induction drops at logOff, the
//!   disconnect teardown and gate travel, the gate-travel reset of the
//!   stations in reach, the world-entry login sync, and the three seams the
//!   inventory and progression code fire (crafting item use, the Field
//!   Crafting Tool refresh, the ASP push).
//! - [`base::crafting`] is the crafting subsystem, moved whole from
//!   `cimmeria-base-session` with its tests. Its `crate::base::…` paths are
//!   unchanged: the [`base`] module re-exports the session-layer modules it
//!   names.
//!
//! The crafting verbs themselves (95-100) are SGWPlayer **cell** methods:
//! `cimmeria-cell-methods` parses them and sends the envelope. Nothing
//! depends on this crate but the composition root (`cimmeria-services`,
//! which lists it in the base plugin table) and test code, so an edit here
//! rebuilds this crate, the facade and the binaries only. What stayed below
//! and why: ADR §4.5.

#![warn(unreachable_pub)]

pub mod base {
    //! The session-layer skeleton the moved crafting code names
    //! (`crate::base::helpers`, `crate::base::ConnectedClientState`, …),
    //! re-exported from `cimmeria-base-session`, and the crafting subsystem.

    pub(crate) use cimmeria_base_session::base::{
        gm_feedback, helpers, outbox, session_identity, ConnectedClientState,
    };

    pub mod crafting;
}

mod plugin;

pub use plugin::CraftingPlugin;

// The services-side Mercury builders and the Base<->Cell messages, at the
// crate-level paths the moved code names (`crate::mercury::…`,
// `crate::cell::messages::…`), as in `cimmeria-base-session`.
use cimmeria_wire::mercury;

mod cell {
    pub(crate) use cimmeria_wire::cell::messages;
}

// Generic helpers come from `cimmeria-test-support` (a dev-dependency) and the
// session fixtures from `cimmeria_base_session::test_fixtures`, so the moved
// tests keep importing both from `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_base_session::test_fixtures::*;
    pub(crate) use cimmeria_test_support::*;
}

#[cfg(test)]
mod plugin_tests;
