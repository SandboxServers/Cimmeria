//! Observability for the client-authoritative position path: what a
//! rejection says, how often it is allowed to say it, and a low-rate
//! positive sample of where players actually walk.
//!
//! This module changes **no** movement decision. `client_move.rs` decides
//! what is accepted and what is snapped back; everything here reports on
//! that decision after the fact.
//!
//! Layout:
//!
//! - this file — the per-entity state ([`MovementTelemetry`]), the
//!   [`LogThrottle`] primitive both it and the NPC path-failure log sit
//!   behind, the shared label constants, and the once-per-space
//!   [`log_navmesh_loaded`] line.
//! - [`reject`] — every **hard reject**, whichever of the three outcomes
//!   the validator chose for it.
//! - [`position_sample`] — the positive-space counterpart: accepted
//!   player positions, sampled.
//!
//! # Why the throttle exists
//!
//! Three days of production logs in September 2026 carried 146,760
//! `movement.validation_reject` WARN rows. 103,818 of them — 71% — came
//! from **one entity** in Harset, stuck against a wall and re-reporting
//! the same rejected position at its 10 Hz update rate. The signal (which
//! worlds reject players, where, and why) was buried under one player's
//! repetition, and the retention budget paid for all of it.
//!
//! [`LogThrottle`] fixes the volume without losing the fact: the first
//! reject for an entity emits immediately, subsequent ones inside the
//! window are counted rather than written, and the next row that *is*
//! written carries `suppressed = N`. A stuck player becomes one row per
//! second with an explicit count of what was elided, instead of ten rows
//! per second. The `movement_validation_rejects_total` counter is
//! incremented on **every** reject regardless — the throttle governs log
//! lines, never the metric.
//!
//! # Why the reporting lives here and not in the handler
//!
//! The reject log needs three things that are only reachable through the
//! [`SpaceManager`]: the world name for a space id, the navmesh diagnosis
//! for the rejected point, and the per-entity throttle state. Inlining
//! that into `base_messages/movement.rs` would put a three-step
//! borrow-ordering dance into the middle of a message handler — three
//! times over, once per outcome, which is exactly how the accounting
//! came to be applied to only one of them (see [`reject`]).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::NavMeshFingerprint;

mod position_sample;
mod reject;

pub(crate) use reject::{HardReject, RecoveryReport, RejectReport, SuppressionReport};

#[cfg(test)]
mod tests;

/// The one line that says which mesh a space is running, emitted once
/// per space creation from
/// [`SpaceManager::create_space_instance`](super::SpaceManager).
///
/// The pre-existing line carried `space_id`, `world` and a polygon
/// count. A poly count cannot answer "which mesh build was this session
/// running on?" — which is exactly the question the September 2026
/// Castle_CellBlock rebuild had to reconstruct by hand from deploy
/// timestamps, after SigNoz showed real players being snapped back
/// where the 2013 mesh had holes. Every per-event navmesh log carries
/// `navmesh_hash` (the 8-digit short form); **this** line is what those
/// join back to, and the only place the full 16-digit hash, the file
/// size and the agent parameters appear.
///
/// A free function rather than an inline `tracing::info!` for two
/// reasons: the `.nav` path `create_space_instance` builds is relative
/// to the process CWD, so a test cannot make that branch fire at all —
/// and this is the only navmesh log line in the codebase with no other
/// caller to prove its field set.
pub(crate) fn log_navmesh_loaded(space_id: u32, world_name: &str, fp: &NavMeshFingerprint) {
    tracing::info!(
        target: "movement.navmesh",
        event = "navmesh_loaded",
        space_id,
        world = %world_name,
        path = %fp.path,
        polys = fp.npolys,
        verts = fp.nverts,
        file_bytes = fp.file_bytes,
        navmesh_hash = %fp.content_hash,
        navmesh_short_hash = %fp.short_hash,
        agent_height = fp.agent_height,
        agent_climb = fp.agent_climb,
        agent_radius = fp.agent_radius,
        "NavMesh loaded for space"
    );
}

/// Minimum gap between two emitted hard-reject rows for the same
/// entity. Rejects inside the window are counted into `suppressed`
/// instead of written.
///
/// One second at the client's 10 Hz update rate means a continuously
/// rejecting player costs ~1 row/s instead of ~10 — a 90% cut — while a
/// one-off reject (the interesting case) is still written the instant it
/// happens.
pub(crate) const REJECT_LOG_MIN_INTERVAL: Duration = Duration::from_secs(1);

/// Minimum gap between accepted-position samples for one player.
///
/// 5 s is ~720 rows/hour/player, against the ~2,000 rows/hour/player the
/// existing 1-in-10 `movement.player` sample produces. See
/// [`SpaceManager::sample_accepted_position_at`](super::SpaceManager::sample_accepted_position_at)
/// for the volume budget.
pub(crate) const POSITION_SAMPLE_MIN_INTERVAL: Duration = Duration::from_secs(5);

/// Minimum distance a player must have covered since their last sample
/// before another one is written, in world units.
///
/// A standing player is not new information — without this, an AFK
/// client parked in a safe room would emit the same coordinates every
/// 5 s forever and dominate the "walked cells" map with one point.
pub(crate) const POSITION_SAMPLE_MIN_DISTANCE: f32 = 1.0;

/// Label used when a space id cannot be resolved to a world. Should not
/// happen on a live reject (the space was just read to fetch bounds),
/// but a metric label must have *some* value and an empty string is
/// indistinguishable from a missing label in ClickHouse.
pub(crate) const UNKNOWN_WORLD: &str = "unknown";

/// Label used on the `gate` axis of `movement_validation_rejects_total`
/// for rejects that are not navmesh rejects at all (bounds, teleport).
/// Explicit rather than omitted so the label set is uniform across every
/// increment of the counter — a label that appears on only some series
/// makes `sum by (gate)` silently drop the rest.
pub(crate) const GATE_NOT_APPLICABLE: &str = "n/a";

/// "First one now, then at most one per `interval`, and say how many
/// were skipped" — keyed by entity **and kind**.
///
/// Deliberately not a rate limiter: nothing is dropped silently. Every
/// suppressed occurrence is counted and reported on the next row that
/// gets through, so the log still answers "how bad was it" even though
/// it no longer contains one line per occurrence.
///
/// # Why `kind` is part of the key
///
/// A window per entity alone hides **transitions**. The three hard-reject
/// outcomes are reached in sequence: an entity that is going to end up
/// `CorrectionSuppressed` first spends its correction budget as ordinary
/// `Rejected` rows, which at 10 Hz takes ~0.5 s — inside the first
/// window. A single shared window therefore swallowed the one row that
/// says "the server has stopped correcting this client", which is the
/// row an operator is meant to act on, and only let it through a second
/// later.
///
/// Per-kind windows cost nothing in the steady state (a stuck entity
/// repeats *one* kind, so it is still ~1 row/s) and bound the pathological
/// alternating case at one row per kind per interval.
#[derive(Debug, Default)]
pub(crate) struct LogThrottle {
    entries: HashMap<(u32, &'static str), ThrottleEntry>,
}

#[derive(Debug)]
struct ThrottleEntry {
    last_emit: Instant,
    /// Occurrences since `last_emit` that were not written.
    suppressed: u32,
}

impl LogThrottle {
    /// Decide whether this occurrence should be logged.
    ///
    /// `Some(n)` means "emit, and report `suppressed = n`" (`n == 0` for
    /// the first occurrence and for any occurrence after a quiet
    /// window). `None` means "count it, write nothing".
    ///
    /// `kind` is a stable low-cardinality token naming what is being
    /// throttled for this entity; each gets its own window. A caller
    /// with only one row shape passes one constant.
    pub(crate) fn admit(
        &mut self,
        entity_id: u32,
        kind: &'static str,
        now: Instant,
        interval: Duration,
    ) -> Option<u32> {
        match self.entries.get_mut(&(entity_id, kind)) {
            None => {
                self.entries.insert(
                    (entity_id, kind),
                    ThrottleEntry {
                        last_emit: now,
                        suppressed: 0,
                    },
                );
                Some(0)
            }
            Some(e) => {
                // `saturating_duration_since`, not `-`: `Instant`
                // subtraction panics on a non-monotonic sample, and a
                // test that rewinds the clock (or a platform with a
                // coarse timer) must not take the server down over a
                // log-throttle decision.
                if now.saturating_duration_since(e.last_emit) >= interval {
                    let suppressed = e.suppressed;
                    e.last_emit = now;
                    e.suppressed = 0;
                    Some(suppressed)
                } else {
                    e.suppressed = e.suppressed.saturating_add(1);
                    None
                }
            }
        }
    }

    /// Drop **every** kind's state for an entity. Called from every
    /// teardown path, so the map cannot grow past the live entity
    /// population and a recycled `entity_id` never inherits a
    /// predecessor's suppression count.
    pub(crate) fn forget(&mut self, entity_id: u32) {
        self.entries.retain(|(id, _), _| *id != entity_id);
    }

    /// Number of tracked (entity, kind) slots. Test-only: the leak guard
    /// asserts this returns to zero after teardown.
    #[cfg(test)]
    pub(crate) fn tracked(&self) -> usize {
        self.entries.len()
    }
}

/// Where the last accepted sample for a player was taken.
#[derive(Debug)]
struct PositionSample {
    at: Instant,
    position: Vector3,
}

/// All per-entity state the movement telemetry keeps. One struct so
/// every teardown path has one call to make and a future addition cannot
/// be forgotten in the cleanup path.
#[derive(Debug, Default)]
pub(crate) struct MovementTelemetry {
    /// Gates every hard-reject row — see [`reject`].
    pub(crate) reject_log: LogThrottle,
    /// Gates `npc_ai.path_fail` — see
    /// [`crate::cell::service::npc_ai::path_failure`].
    pub(crate) npc_path_fail_log: LogThrottle,
    /// Last accepted-position sample per player.
    position_samples: HashMap<u32, PositionSample>,
}

impl MovementTelemetry {
    /// Release every per-entity slot.
    ///
    /// Called from `SpaceManager::destroy_entity` alongside
    /// `movement_validator.forget`, **and** from `destroy_space`, which
    /// is the other teardown path: when the last player leaves an
    /// instanced space every remaining NPC goes away without
    /// `destroy_entity` ever being called for it. Missing that path
    /// leaked one `npc_path_fail_log` slot per NPC per instance for the
    /// process lifetime, and left a recycled entity id inheriting a
    /// stale throttle window — which silently swallows the *first*
    /// failure of the new occupant, the one row an incident timeline
    /// most needs.
    pub(crate) fn forget(&mut self, entity_id: u32) {
        self.reject_log.forget(entity_id);
        self.npc_path_fail_log.forget(entity_id);
        self.position_samples.remove(&entity_id);
    }

    /// Test-only: total tracked slots across all three maps.
    #[cfg(test)]
    pub(crate) fn tracked(&self) -> usize {
        self.reject_log.tracked() + self.npc_path_fail_log.tracked() + self.position_samples.len()
    }
}
