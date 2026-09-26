//! NA02 detector tests. Each WARN test reproduces the bug shape the
//! detector exists for, on today's code, and fails if the detector is
//! removed (see the revert notes on each test).
//!
//! - [`leash`] — `enter`, `snap_fallback`, the aggro/leash `loop`, and that
//!   the S5 guard no longer leashes (NA12).
//! - [`path`] — `npc_ai.path` rows and the partial path across two mesh
//!   islands.
//! - [`cover`] — `no_cover` reasons and the `cover.coverage` WARN on
//!   today's seed.
//! - [`state`] — `idle_parked`, `cleared_without_exit`, the aggro-scan
//!   rejects, `spawn_off_mesh`, `stuck`, `npc_ai.los`, and teardown.
//! - [`off_mesh`] — `npc_off_mesh` warns once for an NPC parked off the mesh
//!   since spawn, then samples at DEBUG (NA24).
//!
//! `stale_velocity` (running in place) and `ground` (`ground_deviation`)
//! drive the service loop's movement tick, so they stay in
//! `cimmeria-services` under the same module path. The fixtures both halves
//! share are `crate::test_fixtures::npc_detectors`.

use crate::test_support::{Captured, LogCaptureGuard};

pub(super) use crate::test_fixtures::npc_detectors::*;

mod cover;
mod leash;
mod off_mesh;
mod path;
mod state;

/// Every captured row with this `target` and `event`.
pub(super) fn rows(logs: &LogCaptureGuard, target: &str, event: &str) -> Vec<Captured> {
    logs.all()
        .into_iter()
        .filter(|c| c.target == target && c.has_field("event", event))
        .collect()
}
