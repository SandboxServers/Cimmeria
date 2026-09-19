//! Observability for the client-authoritative position path: what a
//! rejection says, how often it is allowed to say it, and a low-rate
//! positive sample of where players actually walk.
//!
//! This module changes **no** movement decision. `client_move.rs` decides
//! what is accepted and what is snapped back; everything here reports on
//! that decision after the fact.
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
//! borrow-ordering dance into the middle of a message handler. Keeping it
//! behind one function also leaves the door open for a second caller —
//! an advisory-mode world that wants the diagnosis without the
//! rejection — to reuse it rather than copy it.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::movement_validation::{MovementReject, SpaceBounds};
use cimmeria_entity::navigation::NavMeshFingerprint;

use super::SpaceManager;

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

/// Minimum gap between two emitted `movement.validation_reject` rows for
/// the same entity. Rejects inside the window are counted into
/// `suppressed` instead of written.
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
/// [`Self::sample_accepted_position_at`](SpaceManager::sample_accepted_position_at)
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
const UNKNOWN_WORLD: &str = "unknown";

/// Label used on the `gate` axis of `movement_validation_rejects_total`
/// for rejects that are not navmesh rejects at all (bounds, teleport).
/// Explicit rather than omitted so the label set is uniform across every
/// increment of the counter — a label that appears on only some series
/// makes `sum by (gate)` silently drop the rest.
const GATE_NOT_APPLICABLE: &str = "n/a";

/// "First one now, then at most one per `interval`, and say how many
/// were skipped" — keyed by entity.
///
/// Deliberately not a rate limiter: nothing is dropped silently. Every
/// suppressed occurrence is counted and reported on the next row that
/// gets through, so the log still answers "how bad was it" even though
/// it no longer contains one line per occurrence.
#[derive(Debug, Default)]
pub(crate) struct LogThrottle {
    entries: HashMap<u32, ThrottleEntry>,
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
    pub(crate) fn admit(
        &mut self,
        entity_id: u32,
        now: Instant,
        interval: Duration,
    ) -> Option<u32> {
        match self.entries.get_mut(&entity_id) {
            None => {
                self.entries.insert(
                    entity_id,
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

    /// Drop an entity's state. Called from `destroy_entity`, so the map
    /// cannot grow past the live entity population and a recycled
    /// `entity_id` never inherits a predecessor's suppression count.
    pub(crate) fn forget(&mut self, entity_id: u32) {
        self.entries.remove(&entity_id);
    }

    /// Number of tracked entities. Test-only: the leak guard asserts
    /// this returns to zero after teardown.
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
/// `destroy_entity` has one call to make and a future addition cannot be
/// forgotten in the cleanup path.
#[derive(Debug, Default)]
pub(crate) struct MovementTelemetry {
    /// Gates `movement.validation_reject`.
    pub(crate) reject_log: LogThrottle,
    /// Gates `npc_ai.path_fail` — see
    /// [`crate::cell::service::npc_ai::path_failure`].
    pub(crate) npc_path_fail_log: LogThrottle,
    /// Last accepted-position sample per player.
    position_samples: HashMap<u32, PositionSample>,
}

impl MovementTelemetry {
    /// Release every per-entity slot. Called from
    /// `SpaceManager::destroy_entity` alongside
    /// `movement_validator.forget`.
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

/// Everything the caller already knows about a rejected client position.
/// Grouped into a struct rather than eight positional parameters so a
/// future field (or a reordering of the two `[f32; 3]`s, which the type
/// system would not catch) can't silently swap `position` and
/// `last_valid`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RejectReport<'a> {
    pub entity_id: u32,
    pub space_id: u32,
    pub reason: MovementReject,
    /// Stable low-cardinality token for `reason` (`bounds` | `navmesh` |
    /// `teleport`), resolved by the caller so the log and the counter
    /// cannot disagree about it.
    pub reason_label: &'static str,
    /// The position the client claimed, and which was refused.
    pub position: [f32; 3],
    /// The authoritative position it will be snapped back to.
    pub last_valid: [f32; 3],
    pub bounds: &'a SpaceBounds,
}

impl SpaceManager {
    /// Emit the `movement.validation_reject` negative log for one refused
    /// client position, throttled per entity, and increment
    /// `movement_validation_rejects_total`.
    ///
    /// Returns the caller's [`PlayerIdentity`] so the snap-back path
    /// doesn't pay for a second lookup.
    ///
    /// Three things happen here that did not before:
    ///
    /// 1. **`world`.** The reject log carried `space_id` only, so every
    ///    "which worlds are rejecting players" question needed a manual
    ///    space-id → world join. `world` is a metric label too, so the
    ///    counter can be broken down the same way.
    /// 2. **The navmesh diagnosis.** For `reason = "navmesh"` the row now
    ///    names which containment gate failed and by how far, which is
    ///    what separates "the mesh has a hole here" from "this player is
    ///    clipped into the floor". See
    ///    [`cimmeria_entity::navigation::NavMesh::diagnose_point`].
    /// 3. **The throttle.** See the module doc.
    ///
    /// The counter is incremented **before** the throttle decision:
    /// suppressing a log line must not suppress the count, or the
    /// throttle would silently deflate the reject rate an operator
    /// alerts on.
    pub(crate) fn report_movement_reject(
        &mut self,
        report: RejectReport<'_>,
        now: Instant,
    ) -> PlayerIdentity {
        let RejectReport {
            entity_id,
            space_id,
            reason,
            reason_label,
            position,
            last_valid,
            bounds,
        } = report;

        // Resolve before the `&mut self` throttle call: `world_name_for_space`
        // borrows `self` immutably and the borrow cannot outlive it.
        let identity = self.player_identity(entity_id);
        let verdict = match reason {
            // Only the navmesh layer has a gate to report. Running the
            // diagnosis for a bounds/teleport reject would cost two
            // Detour queries per rejected packet to produce a field
            // nobody can act on.
            MovementReject::OffNavmesh => self.diagnose_point(
                entity_id,
                &Vector3::new(position[0], position[1], position[2]),
            ),
            _ => None,
        };
        let gate = verdict.and_then(|v| v.gate_label());
        let world = self
            .world_name_for_space(space_id)
            .unwrap_or(UNKNOWN_WORLD)
            .to_string();
        let navmesh_hash = self.navmesh_short_hash(entity_id).map(str::to_owned);

        // Counted on every reject, logged on some of them.
        cimmeria_observability::counter!(
            "movement_validation_rejects_total",
            "reason" => reason_label,
            "world" => world.clone(),
            "gate" => gate.unwrap_or(GATE_NOT_APPLICABLE),
        );

        let Some(suppressed) = self.reject_log_admit(entity_id, now) else {
            return identity;
        };

        tracing::warn!(
            target: "movement.validation",
            entity_id,
            account_id = identity.account_id,
            player_id = identity.player_id,
            space_id,
            world = %world,
            client_x = position[0],
            client_y = position[1],
            client_z = position[2],
            last_valid_x = last_valid[0],
            last_valid_y = last_valid[1],
            last_valid_z = last_valid[2],
            bounds_min_x = bounds.min[0],
            bounds_min_y = bounds.min[1],
            bounds_min_z = bounds.min[2],
            bounds_max_x = bounds.max[0],
            bounds_max_y = bounds.max[1],
            bounds_max_z = bounds.max[2],
            reason = reason_label,
            reject = ?reason,
            gate,
            nav_horiz_dist = verdict.and_then(|v| v.horizontal_dist),
            nav_dy = verdict.and_then(|v| v.dy),
            navmesh_hash = navmesh_hash.as_deref(),
            suppressed,
            "movement.validation_reject: client position rejected by the \
             {reason_label} layer — snapping back to last valid via FORCED_POSITION"
        );
        identity
    }

    /// Split out so the `&mut self` borrow for the throttle is a single
    /// short statement, distinct from the immutable lookups above it.
    fn reject_log_admit(&mut self, entity_id: u32, now: Instant) -> Option<u32> {
        self.movement_telemetry
            .reject_log
            .admit(entity_id, now, REJECT_LOG_MIN_INTERVAL)
    }

    /// Low-rate positive sample of an **accepted** player position.
    ///
    /// Rejects tell us where players are stopped; nothing told us where
    /// they successfully walk. Without that, a navmesh hole is only
    /// visible once somebody falls into it — there is no map of the
    /// surface that actually works. One sample per player per
    /// [`POSITION_SAMPLE_MIN_INTERVAL`], and only after they have moved
    /// [`POSITION_SAMPLE_MIN_DISTANCE`], builds that map from ordinary
    /// play.
    ///
    /// **Volume.** 1 row / 5 s / moving player = 720 rows/hour/player; at
    /// 20 concurrent players, 14,400 rows/hour. For scale, the reject
    /// stream this PR throttles was running at ~2,000 rows/hour on its
    /// own. Emitted at DEBUG, matching the sibling `movement.player`
    /// sample — see
    /// `docs/architecture/instrumentation-discipline.md`.
    ///
    /// **Players only.** NPC positions are already covered by
    /// `movement.npc` and `npc_ai.tick`, and NPCs outnumber players by
    /// an order of magnitude in a populated zone — sampling them would
    /// swamp the signal this exists to produce.
    pub(crate) fn sample_accepted_position_at(
        &mut self,
        entity_id: u32,
        position: [f32; 3],
        now: Instant,
    ) {
        let pos = Vector3::new(position[0], position[1], position[2]);

        // Cheapest gate first: NPCs never sample, and this is the accept
        // path of every inbound position packet.
        //
        // Two independent player signals, same as the despawn path:
        // `is_player` is stamped by `connect_entity`, and space
        // membership in `players` is the other half. Checking both means
        // a position update that somehow arrives before the flag is
        // stamped still samples, rather than the player silently
        // contributing nothing to the walked-surface map for their
        // first packets.
        let space_id = self.get_entity_space_id(entity_id);
        let is_player = self.get_entity(entity_id).is_some_and(|e| e.is_player)
            || space_id
                .and_then(|sid| self.spaces.get(&sid))
                .is_some_and(|s| s.players.contains(&entity_id));
        if !is_player {
            return;
        }

        match self.movement_telemetry.position_samples.get(&entity_id) {
            Some(prev)
                if now.saturating_duration_since(prev.at) < POSITION_SAMPLE_MIN_INTERVAL
                    || prev.position.distance_to(&pos) < POSITION_SAMPLE_MIN_DISTANCE =>
            {
                return;
            }
            _ => {}
        }

        let world = space_id
            .and_then(|sid| self.world_name_for_space(sid))
            .unwrap_or(UNKNOWN_WORLD)
            .to_string();
        // `None` = meshless space (nothing to be on or off), which the
        // absent field encodes correctly. `Some(false)` = there is a
        // mesh and this accepted position is not on it — which for a
        // non-GM player should be impossible, and is therefore one of
        // the more interesting rows this sampler can produce.
        let verdict = self.diagnose_point(entity_id, &pos);
        let navmesh_hash = self.navmesh_short_hash(entity_id).map(str::to_owned);
        let identity = self.player_identity(entity_id);

        self.movement_telemetry.position_samples.insert(
            entity_id,
            PositionSample {
                at: now,
                position: pos,
            },
        );

        tracing::debug!(
            target: "movement.position_sample",
            event = "position_sample",
            entity_id,
            account_id = identity.account_id,
            player_id = identity.player_id,
            space_id,
            world = %world,
            x = position[0],
            y = position[1],
            z = position[2],
            on_navmesh = verdict.map(|v| v.valid),
            nav_dy = verdict.and_then(|v| v.dy),
            navmesh_hash = navmesh_hash.as_deref(),
            "movement.position_sample: accepted player position (sampled) — \
             the positive-space counterpart to movement.validation_reject"
        );
    }
}
