//! # cimmeria-cell-combat
//!
//! The cell's combat, and the NPC AI that fights it.
//!
//! - [`cell::abilities`]: `useAbility` resolution, damage application, the
//!   ordered death burst, loot, cone AoE, and the kill-credit wrapper.
//! - [`cell::combat`]: the QR and damage pipeline, threat and player combat
//!   state, the auto-cycle, dead-state flags, and the holster timings.
//! - [`cell::effects`]: the async effect pulsing (DoT / HoT / channels), beside
//!   the world crate's synchronous effect scripts, which it re-exports.
//! - [`cell::service::npc_ai`]: the NPC AI's behaviour (the tick, fighting,
//!   chasing, cover, assist, leashing, patrol, wander, follow), on top of the
//!   world crate's state primitives.
//! - `cell::cell_methods`: the bandolier slot operations and the reload and
//!   item-sequence handlers the combat path drives.
//!
//! Combat raises content events (a kill, a health-threshold crossing, a
//! flank) through `cell::content_events::ContentEvents`, which the content
//! layer above implements; it never calls the content executor.
//!
//! Split out of `cimmeria-services` (wave C2 of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its old
//! nesting, so `crate::cell::…` and `super::…` paths inside it are unchanged,
//! and `cimmeria-services` re-exports each module at its old path.

#![warn(unreachable_pub)]

pub mod cell;

// Lower crates, at the crate-root paths the moved code names them by.
pub(crate) use cimmeria_wire::{firehose, mercury};

/// Fixtures for this crate's tests and, behind the `test-support` feature,
/// the tests of the crates above it.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod test_fixtures;

// Generic helpers come from `cimmeria-test-support` (a dev-dependency) and the
// world fixtures from `cimmeria_cell_world::test_fixtures`, so the moved tests
// keep importing both from `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_cell_world::test_fixtures::*;
    pub(crate) use cimmeria_test_support::*;
}
