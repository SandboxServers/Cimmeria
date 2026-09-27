//! The NA02 detector tests that drive the service loop's movement tick
//! (`ticks::npc_movement_tick`), so they cannot move to `cimmeria-cell-combat`
//! with the rest of the detector tests (wave C2 of
//! `docs/architecture/services-crate-split.md`). Each WARN test reproduces
//! the bug shape the detector exists for and fails if the detector is removed.
//!
//! - [`stale_velocity`] — running in place after `attack_in_place` and after
//!   a leash snap, the throttle, and the moving-NPC negative.
//! - [`ground`] — `ground_deviation` on a lerped chord over the real
//!   `castle_cellblock.nav`.
//! - [`state`] — the avatar-update flag (`npc_moved_since_last`); the rest of
//!   `state` moved.
//!
//! The fixtures are `cimmeria_cell_combat::test_fixtures::npc_detectors`,
//! shared with the detector tests in the combat crate.

use crate::cell::space_manager::SpaceManager;
use crate::test_support::{Captured, LogCaptureGuard};

pub(super) use cimmeria_cell_combat::test_fixtures::npc_detectors::*;

mod ground;
mod stale_velocity;
mod state;

/// Every captured row with this `target` and `event`.
pub(super) fn rows(logs: &LogCaptureGuard, target: &str, event: &str) -> Vec<Captured> {
    logs.all()
        .into_iter()
        .filter(|c| c.target == target && c.has_field("event", event))
        .collect()
}

/// One 100 ms movement tick plus the detector pass that follows it in the
/// message loop.
pub(super) fn movement_tick(mgr: &mut SpaceManager) {
    crate::cell::service::ticks::npc_movement_tick(mgr);
    super::detectors::movement::after_movement_tick(mgr, std::time::Instant::now());
}
