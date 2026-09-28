//! "The owner's pet": the target an owner ability resolves to (pets PT-08).
//!
//! Owner abilities that act on a pet (Holy Warrior, To The Death, Lord's
//! Concentration, the pet heals) never trust a target id from the client.
//! They ask [`SpaceManager::owner_pet_targets`], which walks the registry
//! (`PetRegistry::pets_of`) and keeps a pet only when the summon-time
//! identity says the caster summoned it (`summoner_matches`), it is alive,
//! and it is in the caster's space. A bare owner id is never enough: entity
//! ids are reused, and a player given a destroyed owner's id must not buff
//! or kill that owner's pet.

use cimmeria_entity::cell_entity::PetState;
use cimmeria_wire::state_field::BSF_DEAD;

use super::super::space_manager::SpaceManager;

/// Why an owner ability found no pet to act on. `reason()` is the `reason`
/// field of the `pets.buff` refusal row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerPetRefusal {
    /// The caster has no pet out.
    NoPet,
    /// The caster's pets are all in another space (the teardown sweep has
    /// not run yet).
    PetOtherSpace,
    /// The caster's pets are all dead.
    PetDead,
    /// The registry lists pets under the caster's entity id, but the caster
    /// is not the player who summoned them (the id was reused).
    OwnerIdentityMismatch,
}

impl OwnerPetRefusal {
    /// Stable `reason` value for logs.
    pub fn reason(self) -> &'static str {
        match self {
            Self::NoPet => "no_pet",
            Self::PetOtherSpace => "pet_other_space",
            Self::PetDead => "pet_dead",
            Self::OwnerIdentityMismatch => "owner_identity_mismatch",
        }
    }

    /// The `onErrorCode` `ErrorCodeID` (`EConditionHandlerFeedback`):
    /// `NotLiving` (14) for a dead pet, `EntityDoesNotHavePet` (190)
    /// otherwise.
    pub fn error_code(self) -> u16 {
        match self {
            Self::PetDead => 14,
            _ => 190,
        }
    }

    /// The `CHAN_FEEDBACK` line the owner reads. `onErrorCode` has no Lua
    /// consumer in the shipped client (AT-E1), so this is what the player
    /// sees.
    pub fn feedback_text(self) -> &'static str {
        match self {
            Self::NoPet | Self::OwnerIdentityMismatch => "You have no pet to use that on.",
            Self::PetOtherSpace => "Your pet is not here.",
            Self::PetDead => "Your pet is dead.",
        }
    }
}

/// Which refusal wins when several pets fail for different reasons.
fn refusal_rank(r: OwnerPetRefusal) -> u8 {
    match r {
        OwnerPetRefusal::NoPet => 0,
        OwnerPetRefusal::OwnerIdentityMismatch => 1,
        OwnerPetRefusal::PetOtherSpace => 2,
        OwnerPetRefusal::PetDead => 3,
    }
}

impl SpaceManager {
    /// The live pets `owner` may act on, in summon order: each summoned by
    /// `owner` (checked against the summon-time identity), alive, and in
    /// `owner`'s space. `Err` names why there is none, the most specific
    /// reason winning (dead over elsewhere over reused id).
    ///
    /// Read-only and silent: the caller logs the refusal with its own
    /// context (the ability, the stage).
    pub fn owner_pet_targets(&self, owner: u32) -> Result<Vec<u32>, OwnerPetRefusal> {
        let listed = self.pets.pets_of(owner);
        if listed.is_empty() {
            return Err(OwnerPetRefusal::NoPet);
        }
        let live = self.player_identity(owner);
        let owner_space = self.get_entity_space_id(owner);
        // Every pet that fails says why; the most specific reason wins.
        let mut refusal = OwnerPetRefusal::NoPet;
        let mut note = |r: OwnerPetRefusal| {
            if refusal_rank(r) > refusal_rank(refusal) {
                refusal = r;
            }
        };
        let mut pets = Vec::with_capacity(listed.len());
        for pet in listed {
            if !self.pets.summoner_matches(pet, live) {
                note(OwnerPetRefusal::OwnerIdentityMismatch);
                continue;
            }
            // Listed but its entity is gone: the teardown gap. No pet.
            let Some(entity) = self
                .get_entity(pet)
                .filter(|e| e.extensions.contains::<PetState>())
            else {
                continue;
            };
            if owner_space.is_none() || self.get_entity_space_id(pet) != owner_space {
                note(OwnerPetRefusal::PetOtherSpace);
                continue;
            }
            if entity.state_field & BSF_DEAD != 0 {
                note(OwnerPetRefusal::PetDead);
                continue;
            }
            pets.push(pet);
        }
        if pets.is_empty() {
            Err(refusal)
        } else {
            Ok(pets)
        }
    }
}
