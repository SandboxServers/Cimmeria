//! What a **hard-rejected** client position reports — whichever of the
//! three outcomes the validator resolved it into.
//!
//! # Why all three live behind one seam
//!
//! `reject_outcome` ([`super::super::client_move`]) turns one refused
//! packet into [`ClientMoveOutcome::Rejected`], `Recovered` or
//! `CorrectionSuppressed`. Those differ only in *what the client is
//! told*: correct it to its own position, correct it to a relocation, or
//! tell it nothing at all. All three are the same event — a client
//! position the validator refused.
//!
//! The first cut of this module reported only `Rejected`, because that
//! was the branch the snap-back lived in. The other two therefore never
//! incremented `movement_validation_rejects_total` and never got the
//! navmesh diagnosis or the throttle, so the counter under-counted
//! exactly the rejects that matter most (a player stuck badly enough to
//! exhaust the correction budget produces `CorrectionSuppressed` at
//! 10 Hz, forever, and contributed *nothing* to the reject rate an
//! operator alerts on). [`SpaceManager::account_hard_reject`] is now the
//! one place that counts and diagnoses; each outcome's row is a
//! presentation of the same accounting.
//!
//! Nothing here changes what the client is sent — the handler still
//! emits `FORCED_POSITION` for `Rejected`/`Recovered` and nothing for
//! `CorrectionSuppressed`.
//!
//! [`ClientMoveOutcome::Rejected`]: super::super::ClientMoveOutcome::Rejected

use std::time::Instant;

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::movement_validation::{MovementReject, SpaceBounds};
use cimmeria_entity::navigation::PointVerdict;

use super::super::SpaceManager;
use super::{GATE_NOT_APPLICABLE, REJECT_LOG_MIN_INTERVAL, UNKNOWN_WORLD};

/// The part of a hard reject that is identical whichever outcome the
/// validator chose for it.
///
/// Grouped into a struct rather than passed positionally so a future
/// field (or a reordering of the `[f32; 3]`s, which the type system
/// would not catch) cannot silently swap two coordinates.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HardReject {
    pub entity_id: u32,
    pub space_id: u32,
    pub reason: MovementReject,
    /// Stable low-cardinality token for `reason` (`bounds` | `navmesh` |
    /// `teleport`), resolved by the caller so the log and the counter
    /// cannot disagree about it.
    pub reason_label: &'static str,
    /// The position the client claimed, and which was refused. **Every**
    /// outcome's navmesh diagnosis describes this point, so `gate`,
    /// `nav_horiz_dist` and `nav_dy` mean the same thing on all three
    /// rows.
    pub position: [f32; 3],
}

/// A reject the client is corrected back to its own position for.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RejectReport<'a> {
    pub common: HardReject,
    /// The authoritative position it will be snapped back to.
    pub last_valid: [f32; 3],
    pub bounds: &'a SpaceBounds,
}

/// A reject whose snap target was itself unusable, so the entity was
/// relocated before the client was corrected.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RecoveryReport {
    pub common: HardReject,
    /// The unusable position the entity was stuck at.
    pub from: [f32; 3],
    /// Where it now is — already written to the cell entity.
    pub recovered_to: [f32; 3],
}

/// A reject the client is told nothing about, because re-correcting is
/// what produced the loop.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SuppressionReport {
    pub common: HardReject,
    pub from: [f32; 3],
    /// Consecutive rejects for this entity, for the operator log.
    pub strikes: u32,
}

/// Everything resolved once per hard reject, shared by all three rows.
struct RejectAccounting {
    identity: PlayerIdentity,
    world: String,
    verdict: Option<PointVerdict>,
    gate: Option<&'static str>,
    navmesh_hash: Option<String>,
    /// `Some(n)` — emit a row reporting `suppressed = n`. `None` — the
    /// throttle absorbed this occurrence; it has still been counted.
    emit: Option<u32>,
}

impl SpaceManager {
    /// Count and diagnose one hard reject, and ask the throttle whether
    /// it may be written.
    ///
    /// The counter is incremented **before** the throttle decision:
    /// suppressing a log line must not suppress the count, or the
    /// throttle would silently deflate the reject rate an operator
    /// alerts on.
    fn account_hard_reject(&mut self, common: HardReject, now: Instant) -> RejectAccounting {
        let HardReject {
            entity_id,
            space_id,
            reason,
            reason_label,
            position,
        } = common;

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

        // Counted on every hard reject — `Rejected`, `Recovered` and
        // `CorrectionSuppressed` alike. The two outcome-specific
        // counters below (`..._recoveries_total`,
        // `..._corrections_suppressed_total`) are a breakdown *of* this
        // total, not alternatives to it.
        cimmeria_observability::counter!(
            "movement_validation_rejects_total",
            "reason" => reason_label,
            "world" => world.clone(),
            "gate" => gate.unwrap_or(GATE_NOT_APPLICABLE),
        );

        let emit =
            self.movement_telemetry
                .reject_log
                .admit(entity_id, now, REJECT_LOG_MIN_INTERVAL);

        RejectAccounting {
            identity,
            world,
            verdict,
            gate,
            navmesh_hash,
            emit,
        }
    }

    /// Emit the `movement.validation_reject` negative log for one refused
    /// client position, throttled per entity.
    ///
    /// Returns the caller's [`PlayerIdentity`] so the snap-back path
    /// doesn't pay for a second lookup.
    ///
    /// Three things the row carries beyond the raw coordinates:
    ///
    /// 1. **`world`.** The reject log carried `space_id` only, so every
    ///    "which worlds are rejecting players" question needed a manual
    ///    space-id → world join. `world` is a metric label too, so the
    ///    counter can be broken down the same way.
    /// 2. **The navmesh diagnosis.** For `reason = "navmesh"` the row
    ///    names which containment gate failed and by how far, which is
    ///    what separates "the mesh has a hole here" from "this player is
    ///    clipped into the floor". See
    ///    [`cimmeria_entity::navigation::NavMesh::diagnose_point`].
    /// 3. **The throttle.** See the module doc of [`super`].
    pub(crate) fn report_movement_reject(
        &mut self,
        report: RejectReport<'_>,
        now: Instant,
    ) -> PlayerIdentity {
        let RejectReport {
            common,
            last_valid,
            bounds,
        } = report;
        let acct = self.account_hard_reject(common, now);
        let Some(suppressed) = acct.emit else {
            return acct.identity;
        };
        let reason_label = common.reason_label;
        let position = common.position;

        tracing::warn!(
            target: "movement.validation",
            entity_id = common.entity_id,
            account_id = acct.identity.account_id,
            player_id = acct.identity.player_id,
            space_id = common.space_id,
            world = %acct.world,
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
            reject = ?common.reason,
            gate = acct.gate,
            nav_horiz_dist = acct.verdict.and_then(|v| v.horizontal_dist),
            nav_dy = acct.verdict.and_then(|v| v.dy),
            navmesh_hash = acct.navmesh_hash.as_deref(),
            suppressed,
            "movement.validation_reject: client position rejected by the \
             {reason_label} layer — snapping back to last valid via FORCED_POSITION"
        );
        acct.identity
    }

    /// The rubber-band loop this outcome exists to break: the entity's
    /// own authoritative position was not somewhere the validator would
    /// accept, so snapping the client back to it guaranteed the next
    /// packet would be rejected too. The relocation has already been
    /// written cell-side; the caller only has to tell the owning client
    /// where it now is.
    pub(crate) fn report_movement_recovered(
        &mut self,
        report: RecoveryReport,
        now: Instant,
    ) -> PlayerIdentity {
        let RecoveryReport {
            common,
            from,
            recovered_to,
        } = report;
        let acct = self.account_hard_reject(common, now);
        let Some(suppressed) = acct.emit else {
            return acct.identity;
        };

        tracing::warn!(
            target: "movement.validation",
            entity_id = common.entity_id,
            account_id = acct.identity.account_id,
            player_id = acct.identity.player_id,
            space_id = common.space_id,
            world = %acct.world,
            client_x = common.position[0],
            client_y = common.position[1],
            client_z = common.position[2],
            from_x = from[0],
            from_y = from[1],
            from_z = from[2],
            recovered_x = recovered_to[0],
            recovered_y = recovered_to[1],
            recovered_z = recovered_to[2],
            reason = common.reason_label,
            gate = acct.gate,
            nav_horiz_dist = acct.verdict.and_then(|v| v.horizontal_dist),
            nav_dy = acct.verdict.and_then(|v| v.dy),
            navmesh_hash = acct.navmesh_hash.as_deref(),
            suppressed,
            "movement.validation_recovered: the entity's own position was not a \
             usable snap-back target — relocated to the nearest safe point \
             instead of re-issuing the correction"
        );
        cimmeria_observability::counter!(
            "movement_validation_recoveries_total",
            "reason" => common.reason_label,
        );
        acct.identity
    }

    /// Deliberately produces no `FORCED_POSITION` — re-sending one is
    /// exactly what produced the loop, and there is nowhere better to
    /// send the client to. The cell entity is untouched and
    /// authoritative for AoI, so witnesses still see the truth; the
    /// offending client stays desynced until it sends a position the
    /// validator accepts, which clears the budget.
    ///
    /// ERROR level, and still throttled: this is the one outcome that
    /// can repeat at the full 10 Hz client rate indefinitely, which is
    /// precisely the shape Pattern D exists for.
    pub(crate) fn report_correction_suppressed(&mut self, report: SuppressionReport, now: Instant) {
        let SuppressionReport {
            common,
            from,
            strikes,
        } = report;
        let acct = self.account_hard_reject(common, now);

        cimmeria_observability::counter!(
            "movement_validation_corrections_suppressed_total",
            "reason" => common.reason_label,
        );

        let Some(suppressed) = acct.emit else {
            return;
        };

        tracing::error!(
            target: "movement.validation",
            entity_id = common.entity_id,
            account_id = acct.identity.account_id,
            player_id = acct.identity.player_id,
            space_id = common.space_id,
            world = %acct.world,
            client_x = common.position[0],
            client_y = common.position[1],
            client_z = common.position[2],
            from_x = from[0],
            from_y = from[1],
            from_z = from[2],
            strikes,
            reason = common.reason_label,
            gate = acct.gate,
            nav_horiz_dist = acct.verdict.and_then(|v| v.horizontal_dist),
            nav_dy = acct.verdict.and_then(|v| v.dy),
            navmesh_hash = acct.navmesh_hash.as_deref(),
            suppressed,
            "movement.correction_suppressed: correction budget exhausted with no \
             safe position to recover to — no further FORCED_POSITION will be \
             sent for this entity until it reports an acceptable position"
        );
    }
}
