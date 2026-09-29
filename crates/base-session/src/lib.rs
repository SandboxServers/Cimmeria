//! # cimmeria-base-session
//!
//! The BaseApp's per-connection session layer, the bottom of the base track:
//!
//! - [`base`]'s session types: `ConnectedClientState` (one per client that
//!   finished the Phase 3 handshake), `PendingClientReadyInfo`, the
//!   admin-API `OnlinePlayer` snapshot and `BaseError`.
//! - The per-session plumbing every base handler shares: the witness send
//!   helpers and reliable-send bookkeeping (`base::helpers`), the pre-ready
//!   and cinematic deferred-AoI buffer (`base::deferred_aoi*`,
//!   `base::cinematic_aoi_hold`), the durable base→cell outbox
//!   (`base::outbox`), tick sync and retransmits (`base::tick_sync`), the
//!   `(account_id, player_id)` log correlator (`base::session_identity`), the
//!   GM feedback channel (`base::gm_feedback`) and cooked-data delivery
//!   (`base::cooked_data`).
//! - The session-scoped handlers with no world-entry dependency: contact
//!   list, GM spawn, console authoring, and the chat-channel registration
//!   payloads (`base::world_entry_chat`).
//! - The base plugin API (`base::plugin`, #962 step 5) and the inventory
//!   advisory locks every inventory write shares (`base::inventory_locks`).
//!   Crafting, which used to live here, is a plugin in
//!   `cimmeria-base-crafting`.
//! - Two leaves of the world-entry tree the methods crate needs below it:
//!   the space registry (`base::world_entry::space_registry`) and the
//!   appearance wire builders (`base::world_entry_appearance::builders`).
//!
//! Split out of `cimmeria-services` (wave B1 of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its
//! old nesting, so `crate::base::…` and `super::…` paths inside it are
//! unchanged, and `cimmeria-services` re-exports each module and type at its
//! old path (`cimmeria_services::base::OnlinePlayer`, …).

#![warn(unreachable_pub)]

pub mod base;

// The crate-level paths the moved code names, as it did in
// `cimmeria-services`: the services-side Mercury builders and, under
// `cell::`, the Base<->Cell messages and the spawner catalog. Private
// imports, so the code keeps writing `crate::mercury::…` and
// `crate::cell::messages::…`.
use cimmeria_wire::mercury;

mod cell {
    pub(crate) use cimmeria_cell_catalog::cell::spawner;
    pub(crate) use cimmeria_wire::cell::messages;

    /// The guard that the GM spawn handler maps a template as the cell's
    /// startup cache does. Test-only.
    #[cfg(test)]
    mod spawner_tests;
}

/// Test fixtures for other crates' tests, behind the `test-support` feature.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod test_fixtures;

// Generic helpers come from `cimmeria-test-support` (a dev-dependency), next to
// this crate's own fixtures, so the moved tests keep importing them from
// `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use crate::test_fixtures::*;
    pub(crate) use cimmeria_test_support::*;
}
