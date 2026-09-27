//! `PetRegistry`: the server's owner <-> pet maps, and the two questions
//! every other pet packet asks of them.
//!
//! This is the ownership source of truth (CAT-C-11 / #462). A client
//! command that names a pet id is untrusted until [`PetRegistry::owned_pet`]
//! says the caller owns it; `CellEntity::pet.owner_id` is a convenience copy
//! the spawn path writes in the same step.

use std::collections::HashMap;

use super::super::space_manager::SpaceManager;

/// Why a claimed pet id was refused. `reason()` is the `reason` field of the
/// WARN a rejecting handler logs (negative-logging convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetReject {
    /// The id is not a live pet (an NPC, a player, or nothing).
    NotAPet,
    /// The pet exists but belongs to another player.
    NotOwner {
        /// The real owner.
        owner_id: u32,
    },
    /// The registry still lists the pet but its entity is gone. The
    /// teardown sweep scrubs these; a command in the gap is refused.
    PetGone,
}

impl PetReject {
    /// Stable `reason` value for logs.
    pub fn reason(self) -> &'static str {
        match self {
            Self::NotAPet => "not_a_pet",
            Self::NotOwner { .. } => "not_owner",
            Self::PetGone => "pet_gone",
        }
    }
}

/// Owner <-> pet maps. Lives on `SpaceManager::pets`.
#[derive(Debug, Default)]
pub struct PetRegistry {
    /// pet entity id -> owner entity id.
    owner_of: HashMap<u32, u32>,
    /// owner entity id -> pet entity ids, in summon order.
    pets_by_owner: HashMap<u32, Vec<u32>>,
}

impl PetRegistry {
    /// Record that `owner` owns `pet`. Re-registering a pet moves it.
    pub fn register(&mut self, owner: u32, pet: u32) {
        self.forget_pet(pet);
        self.owner_of.insert(pet, owner);
        self.pets_by_owner.entry(owner).or_default().push(pet);
    }

    /// Drop `pet` from both maps. Returns its owner, if it was registered.
    /// Called from `destroy_entity` and `destroy_space`, so every teardown
    /// path scrubs the registry whatever triggered it.
    pub fn forget_pet(&mut self, pet: u32) -> Option<u32> {
        let owner = self.owner_of.remove(&pet)?;
        if let Some(list) = self.pets_by_owner.get_mut(&owner) {
            list.retain(|&p| p != pet);
            if list.is_empty() {
                self.pets_by_owner.remove(&owner);
            }
        }
        Some(owner)
    }

    /// The owner of `pet`, if it is a registered pet.
    pub fn owner_of(&self, pet: u32) -> Option<u32> {
        self.owner_of.get(&pet).copied()
    }

    /// Whether `entity_id` is a registered pet.
    pub fn is_pet(&self, entity_id: u32) -> bool {
        self.owner_of.contains_key(&entity_id)
    }

    /// The pets `owner` has out, in summon order. Empty for a non-owner.
    pub fn pets_of(&self, owner: u32) -> Vec<u32> {
        self.pets_by_owner.get(&owner).cloned().unwrap_or_default()
    }

    /// The ownership guard: `Ok(claimed)` only when `caller` owns `claimed`.
    ///
    /// Map-only; [`SpaceManager::owned_pet`] adds the "entity still exists"
    /// check and is what handlers call.
    pub fn owned_pet(&self, caller: u32, claimed: u32) -> Result<u32, PetReject> {
        match self.owner_of.get(&claimed) {
            Some(&owner) if owner == caller => Ok(claimed),
            Some(&owner_id) => Err(PetReject::NotOwner { owner_id }),
            None => Err(PetReject::NotAPet),
        }
    }

    /// Every `(pet, owner)` pair, for the teardown sweep.
    pub fn pairs(&self) -> Vec<(u32, u32)> {
        self.owner_of.iter().map(|(&p, &o)| (p, o)).collect()
    }

    /// Number of live pets.
    pub fn len(&self) -> usize {
        self.owner_of.len()
    }

    /// No pets anywhere. The per-tick sweep returns at once when true.
    pub fn is_empty(&self) -> bool {
        self.owner_of.is_empty()
    }
}

impl SpaceManager {
    /// The ownership guard every pet command handler calls first
    /// (CAT-C-11 / #462): `Ok(claimed)` only when `caller` owns the
    /// live pet `claimed`. A mismatch is the caller's to report as a WARN
    /// with `reason` plus visible feedback, never a silent drop.
    pub fn owned_pet(&self, caller: u32, claimed: u32) -> Result<u32, PetReject> {
        let pet = self.pets.owned_pet(caller, claimed)?;
        if self.get_entity(pet).is_none_or(|e| e.pet.is_none()) {
            return Err(PetReject::PetGone);
        }
        Ok(pet)
    }

    /// Who is credited for something `attacker` did (XP, kill credit, loot
    /// ownership): a pet credits its owner, a player itself, any other NPC
    /// nobody. The one seam PT-06 routes `grant_kill_xp` and kill credit
    /// through, so a pet kill reaches its owner and an NPC never gets
    /// `GrantXP`.
    pub fn credit_recipient(&self, attacker: u32) -> Option<u32> {
        if let Some(owner) = self.pets.owner_of(attacker) {
            return Some(owner);
        }
        self.get_entity(attacker)
            .filter(|e| e.is_player)
            .map(|_| attacker)
    }
}
