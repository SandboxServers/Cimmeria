//! # cimmeria-cell-interactions
//!
//! The cell's player interactions: what a player does to an NPC, a gate or
//! another player, and the paths that move a player somewhere else.
//!
//! - [`cell::interactions`]: the `interact` and `initialResponse` dispatch
//!   (dialogs, vendors, trainers, loot, the DHD), beside the content crate's
//!   dialog display, which it re-exports.
//! - [`cell::gate_travel`]: the stargate dial, the gate sequences, the
//!   crossing and the per-tick dial and crossing drains.
//! - [`cell::space_transfer`]: the GM console's cross-space transfer.
//! - [`cell::respawn`]: the respawn fork the player's Defeat Window and the GM
//!   `gmRespawn` share, with the region registration and client-cache resync
//!   it queues after the reanchor.
//! - [`cell::trade`]: the player-to-player trade session state and its
//!   outbound wire, which the departure paths cancel.
//! - [`cell::mail`]: the mail requests the cell forwards to the base.
//!
//! Split out of `cimmeria-services` (wave C4 of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its old
//! nesting, so `crate::cell::…` and `super::…` paths inside it are unchanged,
//! and `cimmeria-services` re-exports each module at its old path.

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
