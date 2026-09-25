//! Cover-slot reservation lifecycle.
//!
//! `reserve_for_entity` is atomic-swap per `entities/defs/SGWCoverSet.def`:
//! "requests a cover slot to be reserved, this will automatically release
//! any slots already reserved by entity". So an NPC re-picking cover does
//! not need to release first.
//!
//! Two parallel maps stay consistent because every mutation goes through
//! one of the methods below — the invariant a future refactor must
//! preserve.
//!
//! Two pieces of per-NPC cover state ride along (NA22), because they
//! live and die with the reservation:
//!
//! - **`in_stance`**: the NPC reached its slot and holds Cover Stance
//!   (ability 1451). Only `cover::stance` writes it, so the buff is
//!   granted once per arrival and revoked exactly once.
//! - **`seek_after`**: an NPC that looked for cover and found none (or
//!   found its slot unreachable) does not look again before this instant.
//!   This is the seek hysteresis: an NPC that already has a shot re-checks
//!   cover every few seconds, not every tick.
//!
//! And two more from NA23:
//!
//! - **`blind_since`**: the NPC stands at its slot and has had no line of
//!   sight to its target from the slot's peek point since this instant. It
//!   gives the slot up after a grace period (`cover_released_no_shot`).
//!   Dropped with the reservation.
//! - **`slot_cooldown`**: the slot this NPC last gave up as flanked or blind,
//!   and until when it may not pick it again. Outlives the reservation on
//!   purpose: it is what stops the MessHall release-and-re-pick churn
//!   (UAT-1).

use cimmeria_common::EntityId;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use super::types::CoverSlotKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReserveError {
    AlreadyReserved { holder: EntityId },
}

#[derive(Debug, Default)]
pub struct CoverReservations {
    slot_to_entity: HashMap<CoverSlotKey, EntityId>,
    entity_to_slot: HashMap<EntityId, CoverSlotKey>,
    in_stance: HashSet<EntityId>,
    seek_after: HashMap<EntityId, Instant>,
    blind_since: HashMap<EntityId, Instant>,
    slot_cooldown: HashMap<EntityId, (CoverSlotKey, Instant)>,
}

impl CoverReservations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reserve_for_entity(
        &mut self,
        entity_id: EntityId,
        slot: CoverSlotKey,
    ) -> Result<(), ReserveError> {
        if let Some(&current_holder) = self.slot_to_entity.get(&slot) {
            if current_holder == entity_id {
                return Ok(());
            }
            // `race_lost` counter — paired with the warn line in
            // `ai_integration::try_reserve_or_warn`. Tracks how often
            // an NPC wanted a slot another entity already holds (today
            // unreachable in production due to the single-guard
            // invariant — defensive against future async refactors).
            cimmeria_observability::counter!(
                "cover_reservation_state",
                "state" => "race_lost",
            );
            return Err(ReserveError::AlreadyReserved {
                holder: current_holder,
            });
        }

        // SGWCoverSet.def: re-reserving must release any prior slot held
        // by this entity. Emit the `released` counter for the implicit
        // release so derived totals like "currently held = held −
        // released" stay balanced. Without this, every re-reserve into
        // a different slot would inflate `held` without a matching
        // `released`, drifting the cover-occupancy dashboard by the
        // number of slot moves per NPC over the process lifetime.
        if let Some(prior_slot) = self.entity_to_slot.remove(&entity_id) {
            self.slot_to_entity.remove(&prior_slot);
            self.blind_since.remove(&entity_id);
            cimmeria_observability::counter!(
                "cover_reservation_state",
                "state" => "released",
            );
        }

        self.slot_to_entity.insert(slot, entity_id);
        self.entity_to_slot.insert(entity_id, slot);
        cimmeria_observability::counter!(
            "cover_reservation_state",
            "state" => "held",
        );
        Ok(())
    }

    pub fn release_slot(&mut self, slot: CoverSlotKey) -> bool {
        if let Some(entity_id) = self.slot_to_entity.remove(&slot) {
            self.entity_to_slot.remove(&entity_id);
            self.blind_since.remove(&entity_id);
            cimmeria_observability::counter!(
                "cover_reservation_state",
                "state" => "released",
            );
            true
        } else {
            false
        }
    }

    pub fn release_for_entity(&mut self, entity_id: EntityId) -> Option<CoverSlotKey> {
        let slot = self.entity_to_slot.remove(&entity_id)?;
        self.slot_to_entity.remove(&slot);
        self.blind_since.remove(&entity_id);
        cimmeria_observability::counter!(
            "cover_reservation_state",
            "state" => "released",
        );
        Some(slot)
    }

    pub fn is_reserved(&self, slot: CoverSlotKey) -> bool {
        self.slot_to_entity.contains_key(&slot)
    }

    pub fn holder(&self, slot: CoverSlotKey) -> Option<EntityId> {
        self.slot_to_entity.get(&slot).copied()
    }

    pub fn slot_for_entity(&self, entity_id: EntityId) -> Option<CoverSlotKey> {
        self.entity_to_slot.get(&entity_id).copied()
    }

    pub fn reserved_count(&self) -> usize {
        self.slot_to_entity.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, CoverSlotKey)> + '_ {
        self.entity_to_slot.iter().map(|(&e, &s)| (e, s))
    }

    /// Record that `entity_id` holds Cover Stance. Returns `true` when it
    /// did not already, i.e. when the caller should apply the buff.
    pub fn enter_stance(&mut self, entity_id: EntityId) -> bool {
        self.in_stance.insert(entity_id)
    }

    /// Forget `entity_id`'s Cover Stance. Returns `true` when it held one,
    /// i.e. when the caller should remove the buff.
    pub fn leave_stance(&mut self, entity_id: EntityId) -> bool {
        self.in_stance.remove(&entity_id)
    }

    pub fn in_stance(&self, entity_id: EntityId) -> bool {
        self.in_stance.contains(&entity_id)
    }

    /// Do not look for a new slot for `entity_id` before `until`.
    pub fn defer_seek(&mut self, entity_id: EntityId, until: Instant) {
        self.seek_after.insert(entity_id, until);
    }

    /// Whether a seek for `entity_id` is still deferred at `now`. An
    /// expired deferral is dropped here, so the map only holds live ones.
    pub fn seek_deferred(&mut self, entity_id: EntityId, now: Instant) -> bool {
        match self.seek_after.get(&entity_id) {
            Some(&until) if now < until => true,
            Some(_) => {
                self.seek_after.remove(&entity_id);
                false
            }
            None => false,
        }
    }

    /// Drop the seek deferral and the slot cooldown (death, leash,
    /// surrender: the next fight starts fresh).
    pub fn clear_seek(&mut self, entity_id: EntityId) {
        self.seek_after.remove(&entity_id);
        self.slot_cooldown.remove(&entity_id);
    }

    /// Record that `entity_id` has no shot from its slot at `now`, and
    /// return since when it has had none. Only meaningful while it holds a
    /// slot; every release forgets it.
    pub fn note_blind(&mut self, entity_id: EntityId, now: Instant) -> Instant {
        *self.blind_since.entry(entity_id).or_insert(now)
    }

    /// `entity_id` has a shot from its slot again.
    pub fn clear_blind(&mut self, entity_id: EntityId) {
        self.blind_since.remove(&entity_id);
    }

    /// `entity_id` may not pick `slot` again before `until`. One slot per
    /// NPC: a newer cooldown replaces the older one.
    pub fn cool_slot(&mut self, entity_id: EntityId, slot: CoverSlotKey, until: Instant) {
        self.slot_cooldown.insert(entity_id, (slot, until));
    }

    /// The slot `entity_id` may not pick at `now`, if any. An expired
    /// cooldown is dropped here.
    pub fn cooling_slot(&mut self, entity_id: EntityId, now: Instant) -> Option<CoverSlotKey> {
        match self.slot_cooldown.get(&entity_id) {
            Some(&(slot, until)) if now < until => Some(slot),
            Some(_) => {
                self.slot_cooldown.remove(&entity_id);
                None
            }
            None => None,
        }
    }
}
