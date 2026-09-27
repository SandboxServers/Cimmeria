//! `PetRegistry`: the server's owner <-> pet maps, and the two questions
//! every other pet packet asks of them.
//!
//! This is the ownership source of truth (CAT-C-11 / #462). A client
//! command that names a pet id is untrusted until [`PetRegistry::owned_pet`]
//! says the caller owns it; `CellEntity::pet.owner_id` is a convenience copy
//! the spawn path writes in the same step.

use std::collections::HashMap;

use cimmeria_entity::cell_entity::PlayerIdentity;

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
    /// The caller has the owner's entity id but is not the player who
    /// summoned the pet: the owner was destroyed and its id reused before
    /// the sweep removed the pet (Copilot, #870).
    OwnerIdentityMismatch,
}

impl PetReject {
    /// Stable `reason` value for logs.
    pub fn reason(self) -> &'static str {
        match self {
            Self::NotAPet => "not_a_pet",
            Self::NotOwner { .. } => "not_owner",
            Self::PetGone => "pet_gone",
            Self::OwnerIdentityMismatch => "owner_identity_mismatch",
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
    /// owner entity id -> the owner's log identity, captured at summon.
    /// Teardown logs read it because the owner may already be destroyed
    /// (or its id reused) by the time its pet is swept. Dropped with the
    /// owner's last pet.
    owner_identity: HashMap<u32, PlayerIdentity>,
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
                self.owner_identity.remove(&owner);
            }
        }
        Some(owner)
    }

    /// Remember `owner`'s log identity for its pets' later teardown logs.
    pub fn note_owner_identity(&mut self, owner: u32, identity: PlayerIdentity) {
        self.owner_identity.insert(owner, identity);
    }

    /// The owner's identity as captured at summon, or `UNKNOWN`.
    pub fn owner_identity(&self, owner: u32) -> PlayerIdentity {
        self.owner_identity
            .get(&owner)
            .copied()
            .unwrap_or(PlayerIdentity::UNKNOWN)
    }

    /// Whether `live` (the identity of whoever holds `owner`'s entity id
    /// now) is the player who summoned `owner`'s pets.
    ///
    /// Entity ids are reused, so the id alone cannot tell the summoner from
    /// a later player given the same id. The summon-time capture decides:
    /// the character (`player_id`) when it was known, otherwise the account.
    /// A capture with neither half known cannot vouch for anyone, so it
    /// never matches: the pet is refused commands and swept. Every summon
    /// goes through `spawn_pet_from_template`, which captures the owner's
    /// identity, so this only strands a pet whose owner had no session
    /// identity at summon, which is not a real player.
    pub fn owner_identity_matches(&self, owner: u32, live: PlayerIdentity) -> bool {
        let captured = self.owner_identity(owner);
        match (captured.player_id, captured.account_id) {
            (Some(player), _) => live.player_id == Some(player),
            (None, Some(account)) => live.account_id == Some(account),
            (None, None) => false,
        }
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
    ///
    /// Every rejection is logged here once, at DEBUG on `pets.command`
    /// (`event = "ownership_rejected"`, `reason` = [`PetReject::reason`]):
    /// a client can name any id at will, so it is not a WARN
    /// (negative-logging convention). Handlers add feedback, not a second
    /// log of the same seam.
    pub fn owned_pet(&self, caller: u32, claimed: u32) -> Result<u32, PetReject> {
        let result = self.pets.owned_pet(caller, claimed).and_then(|pet| {
            if self.get_entity(pet).is_none_or(|e| e.pet.is_none()) {
                Err(PetReject::PetGone)
            } else if !self
                .pets
                .owner_identity_matches(caller, self.player_identity(caller))
            {
                Err(PetReject::OwnerIdentityMismatch)
            } else {
                Ok(pet)
            }
        });
        if let Err(reject) = result {
            let id = self.player_identity(caller);
            let owner_id = match reject {
                PetReject::NotOwner { owner_id } => Some(owner_id),
                _ => None,
            };
            tracing::debug!(
                target: "pets.command",
                event = "ownership_rejected",
                reason = reject.reason(),
                entity_id = caller,
                caller_id = caller,
                account_id = id.account_id,
                player_id = id.player_id,
                pet_id = claimed,
                owner_id,
                "pet command names a pet the caller does not own"
            );
        }
        result
    }

    /// Who is credited for something `attacker` did (XP, kill credit, loot
    /// ownership): a pet credits its owner, a player itself, any other NPC
    /// nobody. The one seam PT-06 routes `grant_kill_xp` and kill credit
    /// through, so a pet kill reaches its owner and an NPC never gets
    /// `GrantXP`.
    ///
    /// A pet whose owner's entity id now belongs to someone else (the
    /// id-reuse window before the sweep) credits nobody: the id's new holder
    /// did not earn it, and neither does a destroyed owner. That refusal is
    /// a WARN on `pets.credit` (`event = credit_refused`,
    /// `reason = owner_identity_mismatch | owner_gone`).
    pub fn credit_recipient(&self, attacker: u32) -> Option<u32> {
        if let Some(owner) = self.pets.owner_of(attacker) {
            let live = self.player_identity(owner);
            let holder = self.get_entity(owner);
            if holder.is_some_and(|e| e.is_player) && self.pets.owner_identity_matches(owner, live)
            {
                return Some(owner);
            }
            let reason = if holder.is_none() {
                "owner_gone"
            } else {
                "owner_identity_mismatch"
            };
            let summoner = self.pets.owner_identity(owner);
            tracing::warn!(
                target: "pets.credit",
                event = "credit_refused",
                reason,
                entity_id = attacker,
                pet_id = attacker,
                owner_id = owner,
                account_id = summoner.account_id,
                player_id = summoner.player_id,
                holder_account_id = live.account_id,
                holder_player_id = live.player_id,
                "pet kill credit withheld: the owner's entity id no longer belongs to the summoner"
            );
            return None;
        }
        self.get_entity(attacker)
            .filter(|e| e.is_player)
            .map(|_| attacker)
    }
}
