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

use cimmeria_common::EntityId;
use std::collections::HashMap;

use super::types::CoverSlotKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReserveError {
    AlreadyReserved { holder: EntityId },
}

#[derive(Debug, Default)]
pub struct CoverReservations {
    slot_to_entity: HashMap<CoverSlotKey, EntityId>,
    entity_to_slot: HashMap<EntityId, CoverSlotKey>,
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
            return Err(ReserveError::AlreadyReserved {
                holder: current_holder,
            });
        }

        // SGWCoverSet.def: re-reserving must release any prior slot held
        // by this entity.
        if let Some(prior_slot) = self.entity_to_slot.remove(&entity_id) {
            self.slot_to_entity.remove(&prior_slot);
        }

        self.slot_to_entity.insert(slot, entity_id);
        self.entity_to_slot.insert(entity_id, slot);
        Ok(())
    }

    pub fn release_slot(&mut self, slot: CoverSlotKey) -> bool {
        if let Some(entity_id) = self.slot_to_entity.remove(&slot) {
            self.entity_to_slot.remove(&entity_id);
            true
        } else {
            false
        }
    }

    pub fn release_for_entity(&mut self, entity_id: EntityId) -> Option<CoverSlotKey> {
        let slot = self.entity_to_slot.remove(&entity_id)?;
        self.slot_to_entity.remove(&slot);
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
}
