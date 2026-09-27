//! `PetRegistry`: the server's owner <-> pet maps, and the two questions
//! every other pet packet asks of them.
//!
//! This is the ownership source of truth (CAT-C-11 / #462). A client
//! command that names a pet id is untrusted until [`PetRegistry::owned_pet`]
//! says the caller owns it; `CellEntity::pet.owner_id` is a convenience copy
//! the spawn path writes in the same step.

use std::collections::{HashMap, HashSet};

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
    /// pet entity id -> the identity of the player who summoned it,
    /// captured at summon. Keyed by pet, not owner: entity ids are reused,
    /// and a later player given the owner's id can summon a pet of its own
    /// before the sweep. An owner-keyed capture would be overwritten by that
    /// summon and vouch for the new holder on the OLD pet (Copilot, #870).
    /// Teardown logs read it too, because the owner may already be gone.
    summoner: HashMap<u32, PlayerIdentity>,
    /// Owners moved by a same-world ring whose pets have not followed yet:
    /// the ring moves the owner while it is hidden and the pets follow at
    /// `ShowPlayer`, which an aborted trip also sends. See
    /// [`PetRegistry::note_owner_moved`] (pets PT-02).
    moved_owners: HashSet<u32>,
}

impl PetRegistry {
    /// Record that `owner` owns `pet`, summoned by the player `summoner`
    /// (the owner's identity at summon). Re-registering a pet moves it.
    pub fn register(&mut self, owner: u32, pet: u32, summoner: PlayerIdentity) {
        self.forget_pet(pet);
        self.owner_of.insert(pet, owner);
        self.pets_by_owner.entry(owner).or_default().push(pet);
        self.summoner.insert(pet, summoner);
    }

    /// Drop `pet` from both maps. Returns its owner, if it was registered.
    /// Called from `destroy_entity` and `destroy_space`, so every teardown
    /// path scrubs the registry whatever triggered it.
    pub fn forget_pet(&mut self, pet: u32) -> Option<u32> {
        let owner = self.owner_of.remove(&pet)?;
        self.summoner.remove(&pet);
        if let Some(list) = self.pets_by_owner.get_mut(&owner) {
            list.retain(|&p| p != pet);
            if list.is_empty() {
                self.pets_by_owner.remove(&owner);
                self.moved_owners.remove(&owner);
            }
        }
        Some(owner)
    }

    /// Record that `owner` was really moved (a same-world ring's
    /// `TeleportPlayer` went out) and its pets should follow when it
    /// reappears. A no-op for an owner without pets.
    pub fn note_owner_moved(&mut self, owner: u32) {
        if self.pets_by_owner.contains_key(&owner) {
            self.moved_owners.insert(owner);
        }
    }

    /// Whether `owner` was moved since the last call, clearing the mark. A
    /// `ShowPlayer` with no mark (an aborted or failed trip) moves no pet.
    pub fn take_owner_moved(&mut self, owner: u32) -> bool {
        self.moved_owners.remove(&owner)
    }

    /// The identity of the player who summoned `pet`, captured at summon,
    /// or `UNKNOWN`.
    pub fn summoner_identity(&self, pet: u32) -> PlayerIdentity {
        self.summoner
            .get(&pet)
            .copied()
            .unwrap_or(PlayerIdentity::UNKNOWN)
    }

    /// Whether `live` (the identity of whoever holds the pet's owner id
    /// now) is the player who summoned `pet`.
    ///
    /// Entity ids are reused, so the id alone cannot tell the summoner from
    /// a later player given the same id. The summon-time capture decides:
    /// the character (`player_id`) when it was known, otherwise the account.
    /// A capture with neither half known cannot vouch for anyone, so it
    /// never matches: the pet is refused commands and swept. Every summon
    /// goes through `spawn_pet_from_template`, which captures the owner's
    /// identity, so this only strands a pet whose owner had no session
    /// identity at summon, which is not a real player.
    pub fn summoner_matches(&self, pet: u32, live: PlayerIdentity) -> bool {
        let captured = self.summoner_identity(pet);
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
                .summoner_matches(pet, self.player_identity(caller))
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
    ///
    /// Call this once per kill resolution, where the refusal is the answer
    /// (PT-06: `grant_kill_xp`, which every XP-paying kill reaches). Routing
    /// gates and the second credit lookup of the same kill use
    /// [`Self::credit_recipient_quiet`], so a refused kill leaves exactly
    /// one `credit_refused` row.
    pub fn credit_recipient(&self, attacker: u32) -> Option<u32> {
        match self.resolve_credit(attacker) {
            Ok(recipient) => recipient,
            Err(refusal) => {
                let summoner = self.pets.summoner_identity(attacker);
                tracing::warn!(
                    target: "pets.credit",
                    event = "credit_refused",
                    reason = refusal.reason,
                    entity_id = attacker,
                    pet_id = attacker,
                    owner_id = refusal.owner_id,
                    account_id = summoner.account_id,
                    player_id = summoner.player_id,
                    holder_account_id = refusal.holder.account_id,
                    holder_player_id = refusal.holder.player_id,
                    "pet kill credit withheld: the owner's entity id no longer belongs to the summoner"
                );
                None
            }
        }
    }

    /// [`Self::credit_recipient`] without the `credit_refused` log: the
    /// same decision, for callers that only route on it (the NPC AI and
    /// warmup kill-credit gates, which run on every cast) or that ask again
    /// about a kill `grant_kill_xp` has already logged (mission credit).
    pub fn credit_recipient_quiet(&self, attacker: u32) -> Option<u32> {
        self.resolve_credit(attacker).unwrap_or(None)
    }

    /// The credit decision. `Err` only for a pet whose owner id no longer
    /// belongs to its summoner.
    fn resolve_credit(&self, attacker: u32) -> Result<Option<u32>, CreditRefusal> {
        if let Some(owner) = self.pets.owner_of(attacker) {
            let live = self.player_identity(owner);
            let holder = self.get_entity(owner);
            if holder.is_some_and(|e| e.is_player) && self.pets.summoner_matches(attacker, live) {
                return Ok(Some(owner));
            }
            return Err(CreditRefusal {
                owner_id: owner,
                reason: if holder.is_none() {
                    "owner_gone"
                } else {
                    "owner_identity_mismatch"
                },
                holder: live,
            });
        }
        Ok(self
            .get_entity(attacker)
            .filter(|e| e.is_player)
            .map(|_| attacker))
    }
}

/// Why [`SpaceManager::credit_recipient`] refused a pet's credit.
#[derive(Debug, Clone, Copy)]
struct CreditRefusal {
    owner_id: u32,
    /// `owner_gone` | `owner_identity_mismatch`.
    reason: &'static str,
    /// Whoever holds the owner id now (`UNKNOWN` when nobody does).
    holder: PlayerIdentity,
}
