//! # cimmeria-cell-content
//!
//! The cell's content layer: what data-driven chains and missions do to the
//! world.
//!
//! - [`cell::content`]: the chain engine bridge. It loads chains from the
//!   database at startup, fires events from gameplay (`fire_*`), and executes
//!   the resolved actions against the world. `EngineEvents` is the chain
//!   engine as the `ContentEvents` seam combat raises kills, health-threshold
//!   crossings and flanks through.
//! - [`cell::missions`]: accept, abandon, advance and complete, the client
//!   mission updates, and the `MissionUpdate` the base persists.
//! - [`cell::ring_transport`]: the effect dispatcher and the public entry
//!   points of the ring transporter (interact, select destination, region
//!   trigger, the per-tick deadline scan), beside the world crate's ring state
//!   machine, which it re-exports. They fire content chains, and the content
//!   executor starts ring trips, so they live here (§2G).
//! - [`cell::interactions`]: `send_dialog_display`, the one choke point every
//!   dialog display goes through.
//!
//! Split out of `cimmeria-services` (wave C3 of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its old
//! nesting, so `crate::cell::…` and `super::…` paths inside it are unchanged,
//! and `cimmeria-services` re-exports each module at its old path.

#![warn(unreachable_pub)]

pub mod cell;

// Lower crates, at the crate-root paths the moved code names them by.
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
