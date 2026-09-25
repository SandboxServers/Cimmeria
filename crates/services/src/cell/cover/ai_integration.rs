//! NPC AI ↔ cover-system bridge.
//!
//! The NPC AI tick calls [`maintain_cover_for_npc`] once per tick on
//! every NPC in `Fighting` state with a known top threat. Cover is a
//! **firing position** (D-NA05, NA22): a slot is only worth holding or
//! taking if the NPC can shoot its target from it. The function:
//!
//! 1. If the NPC holds a slot:
//!    - **Flank test** via [`is_flanked`]. The threat has moved outside
//!      the cover's defensive arc → release ([`ReleaseReason::Flanked`]);
//!      the NPC re-picks next tick.
//!    - **Reach test.** The target is no longer inside the attack range
//!      from the slot → release ([`ReleaseReason::OutOfRange`]). Measured
//!      from the NPC itself once it stands at the slot (the distance the
//!      attack uses), from the slot while it is still walking there.
//!    - Otherwise [`CoverDecision::StayInCover`]: an NPC spawned in cover,
//!      or one that reached its slot, holds it through the fight.
//! 2. If it holds none, it looks for the best free slot **within attack
//!    range of the target** ([`PICK_RANGE_MARGIN`] inside it, so a slot is
//!    not taken at the edge and dropped a step later), whether or not the
//!    target is in range right now (audit C3: it used to look only when
//!    out of range). An NPC that already has a shot takes a short walk
//!    ([`IN_RANGE_MAX_MOVE`]) and, when it finds nothing, does not look
//!    again for [`SEEK_RETRY`] — the seek hysteresis. On success the slot
//!    is reserved and the decision is [`CoverDecision::MoveToCover`].
//! 3. Otherwise [`CoverDecision::NoCover`], with a [`NoCoverReason`].
//!
//! This function does not mutate the NPC's `nav_path`, velocity or Cover
//! Stance — those are caller concerns (`npc_ai::fight_cover`,
//! [`super::stance`]). Reservation state (and the seek deferral that rides
//! on it) is the only thing it touches, because the reservation table is
//! the load-bearing invariant for "no two NPCs in the same slot" and has
//! to be atomically consistent with the decision.

use cimmeria_common::{EntityId, Vector3};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use super::reservation::CoverReservations;
use super::scoring::{is_flanked, pick_best_traced, CoverWeights, PickTrace, ScoringContext};
use super::types::{Cover, CoverSlotKey};

/// A slot must sit at least this far inside the attack range from the
/// target to be picked. With the reach test releasing only past the range
/// itself, this is the hysteresis band that keeps a strafing target from
/// flipping an NPC between "take the slot" and "leave it" every tick.
pub const PICK_RANGE_MARGIN: f32 = 2.0;
/// The longest walk to cover an NPC that already has a shot will take.
pub const IN_RANGE_MAX_MOVE: f32 = 10.0;
/// After a seek finds nothing (in range) or the chosen slot turns out to
/// be unreachable, the NPC does not look again for this long.
pub const SEEK_RETRY: Duration = Duration::from_secs(4);
/// An NPC within this horizontal distance of its slot stands at it: the
/// reach test switches to the NPC's own distance, and (with no path left
/// to walk) the fight tick treats it as arrived. Also the spawn-hold
/// radius: an NPC authored within this of a node spawns holding it.
pub const COVER_ARRIVE_RADIUS: f32 = 1.5;

/// Attempt to reserve `slot` for `npc_id` on a held reservation guard,
/// emitting a `warn!` per docs/architecture/negative-logging-convention.md
/// when the slot is already taken by another entity. Returns `Ok(())`
/// on success or when the slot was already reserved BY THIS NPC
/// (idempotent re-reserve); `Err(())` on the race-lost path so the
/// caller can fall back to `NoCover`.
///
/// Extracted from [`maintain_cover_for_npc`] so the negative-log
/// behavior is unit-testable without having to construct a race in
/// the guarded outer function (which is unreachable from the current
/// production code — the warn is purely defensive against future
/// async refactors that break the single-guard invariant).
pub(super) fn try_reserve_or_warn(
    reservations_guard: &mut MutexGuard<'_, CoverReservations>,
    npc_id: EntityId,
    slot: CoverSlotKey,
) -> Result<(), ()> {
    match reservations_guard.reserve_for_entity(npc_id, slot) {
        Ok(()) => Ok(()),
        Err(super::reservation::ReserveError::AlreadyReserved {
            holder: current_holder,
        }) => {
            // Per docs/architecture/negative-logging-convention.md:
            // "expectation unmet, player-visible, recoverable" — a
            // future async refactor that breaks the single-guard
            // invariant would silently degrade NPCs' cover decisions
            // without this warn surfacing the race. Greppable by
            // `reason = "cover_slot_taken"`.
            tracing::warn!(
                target: "cover.reservation",
                npc_id = npc_id.0,
                holder = current_holder.0,
                chunk_id = slot.chunk_id,
                node_id = slot.node_id,
                reason = "cover_slot_taken",
                "cover reserve_for_entity lost the race -- falling back to NoCover"
            );
            Err(())
        }
    }
}

/// Lock a `Mutex<CoverReservations>`, recovering gracefully from poisoning.
/// A poisoned mutex on the reservation table is not catastrophic — the
/// worst case is some live reservations are inconsistent and the next
/// scoring pass will re-evaluate. Better to log + continue than to
/// panic the cell process and kill every active player session.
pub(super) fn lock_or_recover(m: &Mutex<CoverReservations>) -> MutexGuard<'_, CoverReservations> {
    match m.lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!(
                "cover reservations mutex was poisoned — recovering inner state; \
                 a prior lock-holder panicked, reservation table may be inconsistent"
            );
            poisoned.into_inner()
        }
    }
}

/// Why a held slot was given up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseReason {
    /// The threat left the cover's defensive arc.
    Flanked,
    /// The target is out of attack range from the slot.
    OutOfRange,
    /// The fight tick could not route to the slot (set by the caller, not
    /// by [`maintain_cover_for_npc`]).
    Unreachable,
    /// The reservation named a node the index does not hold (defensive;
    /// unreachable while the index is immutable).
    Stale,
}

impl ReleaseReason {
    /// The `decision_outcome` value of the release row. Treat as API.
    pub fn outcome(self) -> &'static str {
        match self {
            Self::Flanked => "cover_released_flanked",
            Self::OutOfRange => "cover_released_out_of_range",
            Self::Unreachable => "cover_released_unreachable",
            Self::Stale => "cover_released_stale",
        }
    }
}

/// The decision returned by [`maintain_cover_for_npc`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoverDecision {
    /// NPC holds this slot and it still defends and reaches the target.
    /// The caller walks the NPC to `pos` if it is not there yet, and
    /// otherwise holds it there and fires.
    StayInCover { slot: CoverSlotKey, pos: Vector3 },
    /// NPC gave up its slot. Caller stops the walk toward it and continues
    /// the fight cycle normally; cover will be re-picked next tick if
    /// appropriate.
    Released {
        prior_slot: CoverSlotKey,
        reason: ReleaseReason,
    },
    /// NPC should move to this cover slot. The slot is already reserved
    /// for this NPC at the time this enum is constructed; the caller
    /// must route the NPC to the slot position.
    MoveToCover { slot: CoverSlotKey, pos: Vector3 },
    /// No cover branch — caller falls back to existing pursue-target
    /// logic.
    NoCover,
}

/// Why [`maintain_cover_for_npc`] returned [`CoverDecision::NoCover`]. The
/// `npc_ai decision_outcome=no_cover` row's `reason` (audit C7: this branch
/// used to be a silent `NoCover => {}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoCoverReason {
    /// The NPC does not use cover.
    UseCoverFalse,

    /// Out of range: nothing unreserved inside `MAX_COVER_DISTANCE`, the
    /// vertical band and the attack range of the target.
    NoCandidateInRadius,
    /// The chosen slot was taken between pick and reserve.
    ReserveLost,
    /// The target is in range and no free slot within
    /// [`IN_RANGE_MAX_MOVE`] reaches it: the NPC fights where it stands.
    InRangeNoBetterSlot,
    /// A recent seek found nothing (or an unreachable slot); the next one
    /// waits out [`SEEK_RETRY`].
    SeekCooldown,
    /// The NPC's world has no `resources.worlds` id, so it has no cover.
    NoWorld,
    /// The picked index did not resolve to a node (defensive; unreachable
    /// while the index is immutable).
    IndexMiss,
}

impl NoCoverReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::UseCoverFalse => "use_cover_false",

            Self::NoCandidateInRadius => "no_candidate_in_radius",
            Self::ReserveLost => "reserve_lost",
            Self::InRangeNoBetterSlot => "in_range_no_better_slot",
            Self::SeekCooldown => "seek_cooldown",
            Self::NoWorld => "no_world",
            Self::IndexMiss => "index_miss",
        }
    }
}

/// The working behind one [`maintain_cover_for_npc_traced`] decision.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CoverTrace {
    /// Set exactly when the decision is `NoCover`.
    pub no_cover: Option<NoCoverReason>,
    /// The scoring pass, when one ran.
    pub pick: Option<PickTrace>,
}

/// The NPC-side inputs of one cover decision.
#[derive(Debug, Clone, Copy)]
pub struct CoverQuery {
    pub npc_id: EntityId,
    pub npc_pos: Vector3,
    /// The NPC's `resources.worlds.world_id`; a new slot is only picked
    /// from that world's cover (`None` never picks one). A slot already
    /// held is kept or released on its own merits.
    pub world_id: Option<i32>,
    pub threat_pos: Vector3,
    /// Whether the target is inside `attack_range` of the NPC now.
    pub in_range: bool,
    /// The chosen ability's reach (`max_range`).
    pub attack_range: f32,
    /// `CellEntity.use_cover`. False short-circuits to `NoCover`.
    pub use_cover: bool,
    pub now: Instant,
}

/// Horizontal (XZ) distance.
pub(crate) fn horizontal(a: &Vector3, b: &Vector3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// Run one tick of cover maintenance for the given NPC. See module
/// docs for the decision tree.
pub fn maintain_cover_for_npc(
    q: CoverQuery,
    cover: &Cover,
    weights: &CoverWeights,
) -> CoverDecision {
    maintain_cover_for_npc_traced(q, cover, weights).0
}

/// [`maintain_cover_for_npc`] plus why: the no-cover reason and the
/// scoring pass. The decision is the same.
pub fn maintain_cover_for_npc_traced(
    q: CoverQuery,
    cover: &Cover,
    weights: &CoverWeights,
) -> (CoverDecision, CoverTrace) {
    let no_cover = |reason, pick| {
        (
            CoverDecision::NoCover,
            CoverTrace {
                no_cover: Some(reason),
                pick,
            },
        )
    };
    let decided = |d| (d, CoverTrace::default());
    if !q.use_cover {
        return no_cover(NoCoverReason::UseCoverFalse, None);
    }
    let npc_id = q.npc_id;

    // Hold the reservations lock for the entire pick+reserve sequence.
    // Three separate lock acquisitions previously opened a TOCTOU window
    // where another NPC could reserve the same slot between `pick_best`
    // and `reserve_for_entity`, and where the ally positions could go
    // stale between the collection pass and the reserve. Single-guard
    // scope below closes both races.
    let mut reservations_guard = lock_or_recover(&cover.reservations);

    // Step 1: in cover already?
    if let Some(slot) = reservations_guard.slot_for_entity(npc_id) {
        let Some(node) = cover.index.node_by_key(slot) else {
            // Stale reservation — slot index gone. The spatial index is
            // immutable post-startup; this branch defends against a
            // future feature that removes nodes at runtime (none today).
            reservations_guard.release_for_entity(npc_id);
            return decided(CoverDecision::Released {
                prior_slot: slot,
                reason: ReleaseReason::Stale,
            });
        };
        let flanked = is_flanked(node.pos, node.orient, q.threat_pos);
        let at_slot = horizontal(&q.npc_pos, &node.pos) <= COVER_ARRIVE_RADIUS;
        let reaches = if at_slot {
            q.in_range
        } else {
            node.pos.distance_to(&q.threat_pos) <= q.attack_range
        };
        tracing::debug!(
            target: "cover.flank_check",
            npc_id = npc_id.0,
            slot = ?slot,
            node_x = node.pos.x,
            node_y = node.pos.y,
            node_z = node.pos.z,
            node_orient = node.orient,
            threat_x = q.threat_pos.x,
            threat_y = q.threat_pos.y,
            threat_z = q.threat_pos.z,
            flanked,
            at_slot,
            reaches,
            "cover flank check -- an NPC holding a cover slot tested whether its threat is outside the defensive arc"
        );
        let reason = if flanked {
            Some(ReleaseReason::Flanked)
        } else if !reaches {
            Some(ReleaseReason::OutOfRange)
        } else {
            None
        };
        if let Some(reason) = reason {
            // Release and let the caller re-evaluate next tick.
            reservations_guard.release_for_entity(npc_id);
            return decided(CoverDecision::Released {
                prior_slot: slot,
                reason,
            });
        }
        return decided(CoverDecision::StayInCover {
            slot,
            pos: node.pos,
        });
    }

    // Step 2: the seek hysteresis. An NPC that has a shot and found no
    // slot a moment ago keeps shooting rather than rescanning every tick.
    if q.in_range && reservations_guard.seek_deferred(npc_id, q.now) {
        return no_cover(NoCoverReason::SeekCooldown, None);
    }

    // Cover positions are per world, and an NPC in a world with no
    // `resources.worlds` id has no cover.
    let Some(world_id) = q.world_id else {
        return no_cover(NoCoverReason::NoWorld, None);
    };

    // Step 3: pick + reserve a new slot under the same guard. The ally
    // positions come from the reservation table; pick_best runs
    // immediately after; reserve happens before the guard drops.
    let ally_slots: Vec<Vector3> = reservations_guard
        .iter()
        .filter(|&(other_id, _)| other_id != npc_id)
        .filter_map(|(_, slot)| cover.index.node_by_key(slot).map(|n| n.pos))
        .collect();
    let max_move = if q.in_range {
        IN_RANGE_MAX_MOVE
    } else {
        super::MAX_COVER_DISTANCE
    };
    let ctx = ScoringContext::new(q.npc_pos, q.threat_pos)
        .with_limits((q.attack_range - PICK_RANGE_MARGIN).max(0.0), max_move);
    let pick = pick_best_traced(
        &cover.index,
        world_id,
        &reservations_guard,
        &ctx,
        weights,
        &ally_slots,
    );
    let chosen_idx = match pick.best {
        Some(c) => c.idx,
        None if q.in_range => {
            reservations_guard.defer_seek(npc_id, q.now + SEEK_RETRY);
            return no_cover(NoCoverReason::InRangeNoBetterSlot, Some(pick));
        }
        None => return no_cover(NoCoverReason::NoCandidateInRadius, Some(pick)),
    };
    let node = match cover.index.node(chosen_idx) {
        Some(n) => n,
        None => return no_cover(NoCoverReason::IndexMiss, Some(pick)),
    };
    let slot = node.key();

    // Reserve atomically under the held guard. With no intervening
    // lock-drop, only logic bugs (this NPC's own stale reservation
    // colliding) could fail this — `try_reserve_or_warn` handles the
    // negative path gracefully so a future async refactor that breaks
    // the single-guard invariant still surfaces the race via the
    // `cover.reservation` target.
    if try_reserve_or_warn(&mut reservations_guard, npc_id, slot).is_err() {
        return no_cover(NoCoverReason::ReserveLost, Some(pick));
    }

    (
        CoverDecision::MoveToCover {
            slot,
            pos: node.pos,
        },
        CoverTrace {
            no_cover: None,
            pick: Some(pick),
        },
    )
}
