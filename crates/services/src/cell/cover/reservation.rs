//! Cover-slot reservation lifecycle.
//!
//! Mirrors the semantics declared in `entities/defs/SGWCoverSet.def`:
//!
//! > "requests a cover slot to be reserved, this will automatically
//! >  release any slots already reserved by entity"
//!
//! So `reserve_for_entity(entity_id, slot)` is **atomic-swap**: if the
//! entity already holds another slot, that prior slot is released as a
//! side effect of the new reservation. The auto-release-prior is what
//! makes the NPC AI's "I want to move to a different cover" path safe —
//! the AI doesn't have to explicitly release before re-reserving.
//!
//! The reservation table is small (one entry per NPC actively in cover —
//! probably ≤ a few dozen at peak) so a flat `HashMap` keyed on the slot
//! is sufficient. Two parallel maps are maintained:
//!
//! - `slot_to_entity`: who owns this slot? (collision check on reserve)
//! - `entity_to_slot`: which slot does this entity hold? (for
//!   auto-release-prior and `release_for_entity`)
//!
//! Both maps stay consistent because every mutation goes through one of
//! the four methods below.

use cimmeria_common::EntityId;
use std::collections::HashMap;

use super::types::CoverSlotKey;

/// Errors from `reserve_for_entity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReserveError {
    /// The slot is already reserved by a different entity.
    AlreadyReserved { holder: EntityId },
}

/// In-process reservation table. Guarded by a single `Mutex` on the
/// containing `Cover` service — cover reservations are not hot enough
/// to justify finer-grained sharding.
#[derive(Debug, Default)]
pub struct CoverReservations {
    slot_to_entity: HashMap<CoverSlotKey, EntityId>,
    entity_to_slot: HashMap<EntityId, CoverSlotKey>,
}

impl CoverReservations {
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to reserve `slot` for `entity_id`. Automatically releases any
    /// prior slot held by `entity_id` (the SGWCoverSet.def-declared
    /// behavior). Returns `Err(AlreadyReserved { holder })` if a
    /// *different* entity owns the slot — the caller should re-score and
    /// pick a different node.
    ///
    /// If `entity_id` already holds `slot`, returns `Ok(())` immediately
    /// (idempotent re-reserve, e.g. tick-after-tick while still in cover).
    pub fn reserve_for_entity(
        &mut self,
        entity_id: EntityId,
        slot: CoverSlotKey,
    ) -> Result<(), ReserveError> {
        if let Some(&current_holder) = self.slot_to_entity.get(&slot) {
            if current_holder == entity_id {
                // Already ours — idempotent.
                return Ok(());
            }
            return Err(ReserveError::AlreadyReserved {
                holder: current_holder,
            });
        }

        // Auto-release any prior slot held by this entity.
        if let Some(prior_slot) = self.entity_to_slot.remove(&entity_id) {
            self.slot_to_entity.remove(&prior_slot);
        }

        self.slot_to_entity.insert(slot, entity_id);
        self.entity_to_slot.insert(entity_id, slot);
        Ok(())
    }

    /// Release a specific `slot`. Idempotent: returns `false` if the slot
    /// wasn't reserved.
    pub fn release_slot(&mut self, slot: CoverSlotKey) -> bool {
        if let Some(entity_id) = self.slot_to_entity.remove(&slot) {
            self.entity_to_slot.remove(&entity_id);
            true
        } else {
            false
        }
    }

    /// Release whatever slot `entity_id` currently holds, if any.
    /// Returns the released slot if there was one. Used on combat-end,
    /// death, leash, AoI exit, and flank re-pick.
    pub fn release_for_entity(&mut self, entity_id: EntityId) -> Option<CoverSlotKey> {
        let slot = self.entity_to_slot.remove(&entity_id)?;
        self.slot_to_entity.remove(&slot);
        Some(slot)
    }

    /// Is `slot` reserved by any entity? Used by the scorer to filter
    /// occupied slots before ranking.
    pub fn is_reserved(&self, slot: CoverSlotKey) -> bool {
        self.slot_to_entity.contains_key(&slot)
    }

    /// Look up the holder of `slot`, if any.
    pub fn holder(&self, slot: CoverSlotKey) -> Option<EntityId> {
        self.slot_to_entity.get(&slot).copied()
    }

    /// What slot, if any, does `entity_id` currently hold?
    pub fn slot_for_entity(&self, entity_id: EntityId) -> Option<CoverSlotKey> {
        self.entity_to_slot.get(&entity_id).copied()
    }

    /// Number of currently-reserved slots. Diagnostic.
    pub fn reserved_count(&self) -> usize {
        self.slot_to_entity.len()
    }

    /// Iterate (entity_id, slot) pairs of every active reservation.
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, CoverSlotKey)> + '_ {
        self.entity_to_slot.iter().map(|(&e, &s)| (e, s))
    }
}
