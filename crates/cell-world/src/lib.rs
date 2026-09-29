//! # cimmeria-cell-world
//!
//! The cell's world state: everything the cell systems above it read and
//! mutate through one `&mut SpaceManager`.
//!
//! - [`cell::space_manager`]: spaces and their entity populations, AoI and
//!   witnesses, client movement validation and its telemetry, line of sight
//!   and occlusion, spawning, and the per-entity pending-work queues.
//! - [`cell::service::npc_ai`]: the NPC AI's state primitives (the single
//!   `ai_state` writer, movement stop, the leash policy) and the NA02
//!   detectors.
//! - [`cell::ring_transport`]: the ring-transporter state machine, its regions
//!   and wire payloads, and the departing-player hook.
//! - [`cell::effects`]: the synchronous effect-script layer (Cover Stance runs
//!   through it at spawn): the trait, dispatch and the registry type. The
//!   scripts are in `cimmeria-cell-effect-scripts`, registered at startup.
//! - [`cell::cover`]: the cover crate re-exported, plus the spawn-time cover
//!   hold and Cover Stance grant/revoke.
//! - [`cell::combat`]: NPC aggression, the faction reaction table and the
//!   health-percentage sample.
//! - [`cell::dispatch::gm_gate`], [`cell::arrival`], the playtest friction
//!   watch, and [`cell::CellError`].
//! - [`cell::content_events::ContentEvents`]: the trait the combat layer
//!   raises content events through, so it never calls up into the content
//!   executor.
//!
//! Split out of `cimmeria-services` (wave C1 of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its old
//! nesting, so `crate::cell::…` and `super::…` paths inside it are unchanged,
//! and `cimmeria-services` re-exports each module at its old path.

#![warn(unreachable_pub)]

pub mod cell;

// Lower crates, at the crate-root paths the moved code names them by.
pub(crate) use cimmeria_cell_catalog::ability_tree;
pub(crate) use cimmeria_wire::mercury;

/// Fixtures for this crate's tests and, behind the `test-support` feature,
/// the tests of the crates above it.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod test_fixtures;

// Generic helpers come from `cimmeria-test-support` (a dev-dependency) and the
// world fixtures from `test_fixtures`, so the moved tests keep importing both
// from `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use crate::test_fixtures::*;
    pub(crate) use cimmeria_test_support::*;
}

/// The `mercury::aoi` test that drives a `SpaceManager`: the AoI builders are
/// in `cimmeria-wire`, below this crate. Test-only.
#[cfg(test)]
mod mercury_aoi_tests;
