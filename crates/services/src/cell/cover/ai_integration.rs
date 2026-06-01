//! NPC AI ↔ cover-system bridge.
//!
//! The NPC AI tick calls [`maintain_cover_for_npc`] once per tick on
//! every NPC in `Fighting` state with a known top threat. The function:
//!
//! 1. Checks whether the NPC currently holds a cover slot. If yes:
//!    - **Flank test** via [`is_flanked`]. If the threat has moved
//!      outside the cover-orient defensive arc, the slot is released
//!      and the NPC re-evaluates next tick.
//!    - Otherwise the NPC stays in cover.
//! 2. If the NPC doesn't hold cover and the threat is out of optimal
//!    weapon range, attempts to [`pick_best`] a cover slot near the
//!    NPC. On success: reserves the slot and returns
//!    [`CoverDecision::MoveToCover`] — the caller sets the NPC's
//!    `nav_path` to the slot position and broadcasts
//!    `MobMovementType::Cover` on transition into cover-advance.
//! 3. Returns [`CoverDecision::NoCover`] when `use_cover` is false,
//!    when no cover candidate is available, or when the target is
//!    within optimal range (no need for cover — just fight).
//!
//! This function does not mutate the NPC's `nav_path` or broadcast
//! movement types — those are caller concerns. Reservation state is
//! the only thing this function touches, because the reservation
//! table is the load-bearing invariant for "no two NPCs in the same
//! slot" and has to be atomically consistent with the decision.

use cimmeria_common::{EntityId, Vector3};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use super::reservation::CoverReservations;
use super::scoring::{is_flanked, pick_best, CoverWeights, ScoringContext};
use super::types::{Cover, CoverSlotKey};

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
fn try_reserve_or_warn(
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
fn lock_or_recover(m: &Mutex<CoverReservations>) -> MutexGuard<'_, CoverReservations> {
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

/// The decision returned by [`maintain_cover_for_npc`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoverDecision {
    /// NPC is already in cover; the cover still defends. Caller should
    /// fire ability from current position (or path closer if out of
    /// weapon range, but not via the cover system — direct chase).
    StayInCover { slot: CoverSlotKey, pos: Vector3 },
    /// NPC was just released from a flank-detected slot. Caller should
    /// continue the fight cycle normally; cover will be re-picked next
    /// tick if appropriate.
    Released { prior_slot: CoverSlotKey },
    /// NPC should move to this cover slot. The slot is already reserved
    /// for this NPC at the time this enum is constructed; the caller
    /// must update `nav_path` to the slot position and (if desired)
    /// broadcast `MobMovementType::Cover` to AoI witnesses.
    MoveToCover { slot: CoverSlotKey, pos: Vector3 },
    /// No cover branch — caller falls back to existing pursue-target
    /// logic. This is the "in range and ok" path and the "no candidates
    /// available" path.
    NoCover,
}

/// Run one tick of cover maintenance for the given NPC. See module
/// docs for the decision tree.
///
/// `use_cover` is read from the entity's `CellEntity.use_cover` field.
/// When false the function short-circuits to `NoCover` immediately.
pub fn maintain_cover_for_npc(
    npc_id: EntityId,
    npc_pos: Vector3,
    threat_pos: Vector3,
    in_range: bool,
    use_cover: bool,
    cover: &Cover,
    weights: &CoverWeights,
) -> CoverDecision {
    if !use_cover {
        return CoverDecision::NoCover;
    }

    // Hold the reservations lock for the entire pick+reserve sequence.
    // Three separate lock acquisitions previously opened a TOCTOU window
    // where another NPC could reserve the same slot between `pick_best`
    // and `reserve_for_entity`, and where `ally_counts` could go stale
    // between the count pass and the reserve. Single-guard scope below
    // closes both races. `lock_or_recover` returns the inner data even
    // on a poisoned mutex (a poisoned cover-reservation table is
    // recoverable — the worst case is some live reservations get lost,
    // not data corruption — so we log + continue rather than panic the
    // cell process).
    let mut reservations_guard = lock_or_recover(&cover.reservations);

    // Step 1: in cover already?
    if let Some(slot) = reservations_guard.slot_for_entity(npc_id) {
        if let Some(node) = cover.index.node_by_key(slot) {
            if is_flanked(node.pos, node.orient, threat_pos) {
                // Release and let the caller re-evaluate next tick.
                reservations_guard.release_for_entity(npc_id);
                return CoverDecision::Released { prior_slot: slot };
            }
            return CoverDecision::StayInCover {
                slot,
                pos: node.pos,
            };
        }
        // Stale reservation — slot index gone. The spatial index is
        // immutable post-startup; this branch defends against the
        // pathological case where a future feature removes nodes at
        // runtime (none today).
        reservations_guard.release_for_entity(npc_id);
        return CoverDecision::Released { prior_slot: slot };
    }

    // Step 2: target in range and no current cover → no need for cover.
    if in_range {
        return CoverDecision::NoCover;
    }

    // Step 3: pick + reserve a new slot under the same guard. Squad-
    // affinity counts walk the reservation table once; pick_best runs
    // immediately after; reserve happens before the guard drops. No
    // window for another caller to claim the same slot mid-sequence.
    let mut ally_counts: HashMap<i32, usize> = HashMap::new();
    for (other_id, slot) in reservations_guard.iter() {
        if other_id == npc_id {
            continue;
        }
        *ally_counts.entry(slot.chunk_id).or_insert(0) += 1;
    }

    let ctx = ScoringContext::new(npc_pos, threat_pos);
    let chosen_idx = pick_best(
        &cover.index,
        &reservations_guard,
        &ctx,
        weights,
        &ally_counts,
    );
    let chosen_idx = match chosen_idx {
        Some(i) => i,
        None => return CoverDecision::NoCover,
    };
    let node = match cover.index.node(chosen_idx) {
        Some(n) => n,
        None => return CoverDecision::NoCover,
    };
    let slot = node.key();

    // Reserve atomically under the held guard. With no intervening
    // lock-drop, only logic bugs (this NPC's own stale reservation
    // colliding) could fail this — `try_reserve_or_warn` handles the
    // negative path gracefully so a future async refactor that breaks
    // the single-guard invariant still surfaces the race via the
    // `cover.reservation` target.
    if try_reserve_or_warn(&mut reservations_guard, npc_id, slot).is_err() {
        return CoverDecision::NoCover;
    }

    CoverDecision::MoveToCover {
        slot,
        pos: node.pos,
    }
}

#[cfg(test)]
mod ai_integration_tests {
    use super::*;
    use crate::cell::cover::types::{CoverHeight, CoverNode, CoverQuality};

    fn cover_with(nodes: Vec<CoverNode>) -> Cover {
        Cover::from_loaded(Vec::new(), nodes)
    }

    fn n(chunk_id: i32, node_id: i32, x: f32, z: f32, orient: f32) -> CoverNode {
        CoverNode {
            chunk_id,
            node_id,
            pos: Vector3::new(x, 0.0, z),
            orient,
            height: CoverHeight::Mid,
            quality: CoverQuality::Best,
            tail: [0; 4],
        }
    }

    #[test]
    fn use_cover_false_returns_no_cover() {
        let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
        let dec = maintain_cover_for_npc(
            EntityId(1),
            Vector3::zero(),
            Vector3::new(20.0, 0.0, 0.0),
            false,
            false, // use_cover=false
            &cover,
            &CoverWeights::default(),
        );
        assert_eq!(dec, CoverDecision::NoCover);
    }

    #[test]
    fn in_range_no_current_cover_returns_no_cover() {
        let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
        let dec = maintain_cover_for_npc(
            EntityId(1),
            Vector3::zero(),
            Vector3::new(3.0, 0.0, 0.0),
            true, // in_range=true
            true,
            &cover,
            &CoverWeights::default(),
        );
        assert_eq!(dec, CoverDecision::NoCover);
    }

    #[test]
    fn out_of_range_with_candidate_returns_move_to_cover() {
        let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
        let dec = maintain_cover_for_npc(
            EntityId(1),
            Vector3::zero(),
            Vector3::new(20.0, 0.0, 0.0),
            false,
            true,
            &cover,
            &CoverWeights::default(),
        );
        match dec {
            CoverDecision::MoveToCover { slot, .. } => {
                assert_eq!(slot.chunk_id, 1);
                assert_eq!(slot.node_id, 0);
                // Slot must be reserved after the call.
                let r = cover.reservations.lock().unwrap();
                assert_eq!(r.holder(slot), Some(EntityId(1)));
            }
            other => panic!("expected MoveToCover, got {other:?}"),
        }
    }

    #[test]
    fn flanked_npc_releases_and_returns_released() {
        // Cover at (5,0,0) facing +X (orient=0). NPC reserves it.
        // Threat starts at (20,0,0) — in defensive arc.
        let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
        cover
            .reservations
            .lock()
            .unwrap()
            .reserve_for_entity(EntityId(42), CoverSlotKey::new(1, 0))
            .unwrap();

        // Threat moves to (-20, 0, 0) — flanked.
        let dec = maintain_cover_for_npc(
            EntityId(42),
            Vector3::new(5.0, 0.0, 0.0),
            Vector3::new(-20.0, 0.0, 0.0),
            false,
            true,
            &cover,
            &CoverWeights::default(),
        );
        match dec {
            CoverDecision::Released { prior_slot } => {
                assert_eq!(prior_slot, CoverSlotKey::new(1, 0));
                // Slot must be freed.
                let r = cover.reservations.lock().unwrap();
                assert!(r.holder(prior_slot).is_none());
            }
            other => panic!("expected Released, got {other:?}"),
        }
    }

    #[test]
    fn squad_affinity_routes_two_npcs_to_different_chunks() {
        // Two chunks of equal quality, each with one cover slot. NPC #1
        // reserves a slot in chunk 1; NPC #2 then calls maintain_cover.
        // The squad-affinity penalty must steer NPC #2 away from chunk
        // 1's remaining slots and into chunk 2 instead. Pre-fix (no
        // affinity), NPC #2 picks whichever chunk happens to score
        // first on quality+distance — clustering all NPCs into one
        // chunk and breaking AI-pacing variety.
        //
        // Layout: NPC at origin, threat at +X (20,0,0).
        //  - chunk 1 has two slots — one for NPC #1 (pre-reserved on
        //    node 0), one open on node 1. Open slot at (5,0,0) is the
        //    closest cover, best baseline geometry.
        //  - chunk 2 has one slot at (8,0,0): same orient, worse move.
        // Without affinity: NPC #2 picks chunk 1's open slot (node 1)
        // because it has the highest score (closer to NPC).
        // With affinity (1 ally already in chunk 1 from the pre-seeded
        // reservation), chunk 1's open slot is penalised and chunk 2
        // wins. Removing the `ally_counts` wiring in maintain_cover
        // sends NPC #2 back to chunk 1's open slot.
        let cover = cover_with(vec![
            n(1, 0, 5.5, 0.0, 0.0), // chunk 1 slot 0 — reserved by ally
            n(1, 1, 5.0, 0.0, 0.0), // chunk 1 slot 1 — open, best baseline
            n(2, 0, 8.0, 0.0, 0.0), // chunk 2 slot — worse baseline
        ]);
        // Pin NPC #1 in chunk 1 (on slot 0) so chunk 1 carries an
        // ally count of 1; node 1 stays available.
        cover
            .reservations
            .lock()
            .unwrap()
            .reserve_for_entity(EntityId(1), CoverSlotKey::new(1, 0))
            .unwrap();

        let dec = maintain_cover_for_npc(
            EntityId(2),
            Vector3::zero(),
            Vector3::new(20.0, 0.0, 0.0),
            false,
            true,
            &cover,
            &CoverWeights::default(),
        );
        match dec {
            CoverDecision::MoveToCover { slot, .. } => {
                assert_eq!(
                    slot.chunk_id, 2,
                    "squad-affinity must penalise chunk 1 (already has ally) so the \
                     second NPC routes to chunk 2 instead; got {slot:?}. Removing the \
                     penalty term in scoring.rs would pick chunk 1 here (the slot is \
                     marginally closer to the NPC by 1 z-unit but has the affinity \
                     penalty against it)."
                );
            }
            other => panic!("expected MoveToCover into chunk 2, got {other:?}"),
        }
    }

    /// **Negative-logging regression guard (audit #482 P0).** When the
    /// reservation table reports a slot already held by a *different*
    /// entity, the reserve attempt must emit a structured `warn!` with
    /// `target = "cover.reservation"` and `reason = "cover_slot_taken"`
    /// before falling back to `Err(())`. Without this warn, a future
    /// async refactor that breaks the single-guard invariant in
    /// `maintain_cover_for_npc` would silently degrade NPC cover
    /// decisions — no log line names the race, no SigNoz query catches
    /// the rate.
    ///
    /// Bug shape this catches: revert the warn to a bare `Err(())`
    /// return and the test asserts on a missing event.
    #[test]
    fn try_reserve_warns_when_slot_taken_by_other_holder() {
        use crate::test_support::LogCapture;
        use tracing::Level;

        let capture = LogCapture::install();

        let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
        // Pre-occupy the slot with a DIFFERENT entity so the
        // try_reserve call lands on the AlreadyReserved path.
        cover
            .reservations
            .lock()
            .unwrap()
            .reserve_for_entity(EntityId(99), CoverSlotKey::new(1, 0))
            .unwrap();

        let mut guard = cover.reservations.lock().unwrap();
        let result = try_reserve_or_warn(&mut guard, EntityId(42), CoverSlotKey::new(1, 0));
        drop(guard);

        assert!(
            result.is_err(),
            "race-lost path must return Err so the caller falls back to NoCover"
        );

        let event = capture
            .find_event(
                Level::WARN,
                "cover reserve_for_entity lost the race",
                "cover_slot_taken",
            )
            .unwrap_or_else(|| {
                panic!(
                    "must emit warn at target=cover.reservation with \
                     reason=cover_slot_taken; reverting the warn site \
                     breaks ops visibility of the cover-race condition. \
                     Captured: {:#?}",
                    capture.all()
                )
            });
        assert!(
            event.has_field("npc_id", "42"),
            "warn must carry the npc_id field for correlation: {event:#?}"
        );
        assert!(
            event.has_field("holder", "99"),
            "warn must carry the current slot holder for correlation: {event:#?}"
        );
        assert!(
            event.has_field("chunk_id", "1"),
            "warn must carry the slot's chunk_id: {event:#?}"
        );
    }

    /// Same helper, but called for the entity that ALREADY holds the
    /// slot — idempotent re-reserve must NOT emit a warn (the
    /// reservation contract treats this as Ok(())).
    #[test]
    fn try_reserve_no_warn_on_idempotent_reserve_by_same_holder() {
        use crate::test_support::LogCapture;
        use tracing::Level;

        let capture = LogCapture::install();

        let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
        cover
            .reservations
            .lock()
            .unwrap()
            .reserve_for_entity(EntityId(42), CoverSlotKey::new(1, 0))
            .unwrap();

        let mut guard = cover.reservations.lock().unwrap();
        let result = try_reserve_or_warn(&mut guard, EntityId(42), CoverSlotKey::new(1, 0));
        drop(guard);

        assert!(
            result.is_ok(),
            "same-entity re-reserve must return Ok per CoverReservations contract"
        );
        assert!(
            capture
                .find_event(
                    Level::WARN,
                    "cover reserve_for_entity lost the race",
                    "cover_slot_taken"
                )
                .is_none(),
            "idempotent re-reserve must NOT emit the race-lost warn"
        );
    }

    #[test]
    fn stay_in_cover_when_not_flanked() {
        let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
        cover
            .reservations
            .lock()
            .unwrap()
            .reserve_for_entity(EntityId(42), CoverSlotKey::new(1, 0))
            .unwrap();

        // Threat in defensive arc (still in front of cover orient).
        let dec = maintain_cover_for_npc(
            EntityId(42),
            Vector3::new(5.0, 0.0, 0.0),
            Vector3::new(20.0, 0.0, 0.0),
            false,
            true,
            &cover,
            &CoverWeights::default(),
        );
        match dec {
            CoverDecision::StayInCover { slot, .. } => {
                assert_eq!(slot, CoverSlotKey::new(1, 0));
                // Reservation must persist.
                let r = cover.reservations.lock().unwrap();
                assert_eq!(r.holder(slot), Some(EntityId(42)));
            }
            other => panic!("expected StayInCover, got {other:?}"),
        }
    }
}
