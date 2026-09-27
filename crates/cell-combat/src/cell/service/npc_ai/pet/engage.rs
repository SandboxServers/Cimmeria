//! Putting a pet into a fight: the one engagement entry every pet seam uses
//! (the stance picks here, the owner's attack order from PT-04).
//!
//! A fight is two-sided. `combat::generate_threat(attacker, victim)` writes
//! the attacker's id into the victim's threat list and preempts the victim
//! into Fighting, so a pet engagement needs both directions:
//!
//! - the target lists the pet (`generate_threat(pet, target)`): the mob
//!   fights back, and `sync_owner_combat`, which looks for mobs that list the
//!   pet, puts the owner in combat;
//! - the pet lists the target and is in Fighting: its fight handler picks
//!   the target from its own threat list.
//!
//! The first version seeded only the pet's side (the mob as attacker), so the
//! mob stayed idle until the pet's first hit landed, and the owner was not
//! mirrored into combat until then.

use cimmeria_entity::cell_entity::{AiState, CellEntity};
use cimmeria_entity::stats::HEALTH;

use crate::cell::combat;
use crate::cell::space_manager::SpaceManager;

use super::stance::EngageWhy;

/// Threat each side of a pet engagement is seeded with. The same tiny seed
/// proximity aggro and assist use, so real damage decides the targets as
/// soon as the fight starts.
pub const PET_ENGAGE_THREAT: f32 = 1.0;

/// Who is starting a pet's fight: the pet on its own, or its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetEngagement {
    /// The pet's own decision: a stance pick (defend owner or self, the
    /// owner's target, the Aggressive scan).
    Automatic,
    /// An explicit order from the owner (PT-04's attack order). Obeyed
    /// whatever the stance, and against any target the owner could attack
    /// itself, a surrendered (`Submit`) NPC included: `handle_use_ability`
    /// lets a player attack one.
    OwnerOrder,
}

/// Why `target`'s state rules out engaging it, or `None` when it is
/// engageable. Shared by the engagement and by the stance's candidate
/// filter, so the two cannot drift.
///
/// - `target_dead`: `BSF_DEAD`, no health left, or `AiState::Dead`.
/// - `target_resetting`: walking home (it evades and takes no threat) or
///   leaving the world.
/// - `target_not_engageable` (automatic engagement only): surrendered
///   (`Submit`: `npc_ai_submit` clears both sides of its fight every pass,
///   so re-engaging it would flap the owner in and out of combat), not yet
///   spawned, or in the error state. An owner order still reaches a
///   surrendered NPC, as a player's own attack does.
pub(super) fn target_state_refusal(
    target: &CellEntity,
    kind: PetEngagement,
) -> Option<&'static str> {
    if combat::is_dead_state(target.state_field)
        || target.stats.get(HEALTH).is_none_or(|h| h.cur <= 0)
        || target.ai_state() == AiState::Dead
    {
        return Some("target_dead");
    }
    match target.ai_state() {
        AiState::Leashing | AiState::Despawning => Some("target_resetting"),
        AiState::Submit | AiState::Spawning | AiState::Error
            if kind == PetEngagement::Automatic =>
        {
            Some("target_not_engageable")
        }
        _ => None,
    }
}

/// Engage `target_id` with the pet `pet_id`: the target lists the pet (and
/// fights it), and the pet lists the target and is in Fighting. The one
/// pet-engagement entry; stance engagement and an owner's attack order both
/// go through it. `Err` carries the refusal reason and changes nothing.
///
/// Refused (`Err(reason)`):
///
/// - `not_a_pet`: `pet_id` carries no `PetState`;
/// - `owner_gone` / `owner_identity_mismatch`: the owner is not the live
///   player who summoned the pet (`live_owner`);
/// - `target_gone`, and the state refusals of [`target_state_refusal`]
///   (`target_dead`, `target_resetting`, `target_not_engageable`);
/// - `target_not_combatant` / `target_not_hostile`: the pet may not fight it
///   (`fight_refusal`: a combatant `SGWMob` its owner could attack);
/// - `target_refused_threat`: `generate_threat` would not put the pet on
///   the target's threat list.
///
/// Stance is not checked here: the stance decides whether the pet picks a
/// fight on its own (a Passive pet never does), while an explicit order is
/// obeyed. `kind` says which it is ([`PetEngagement`]). Uses the `pet_stance` aggro cause, which never recruits
/// assisters. Logs nothing itself; callers log the outcome.
pub fn engage_pet_target(
    space_mgr: &mut SpaceManager,
    pet_id: u32,
    target_id: u32,
    kind: PetEngagement,
) -> Result<(), &'static str> {
    let owner_id = space_mgr
        .get_entity(pet_id)
        .and_then(|e| e.pet.as_deref())
        .map(|p| p.owner_id)
        .ok_or("not_a_pet")?;
    let owner = super::live_owner(space_mgr, pet_id, owner_id)?;
    let target = space_mgr.get_entity(target_id).ok_or("target_gone")?;
    if let Some(why) = super::fight_refusal(owner, target) {
        return Err(why);
    }
    if let Some(why) = target_state_refusal(target, kind) {
        return Err(why);
    }

    // The target's side: it lists the pet and fights back.
    let _ = combat::generate_threat(
        space_mgr,
        pet_id,
        target_id,
        PET_ENGAGE_THREAT,
        combat::AggroCause::PetStance,
    );
    if !space_mgr
        .get_entity(target_id)
        .is_some_and(|t| t.threat_list.contains_key(&pet_id))
    {
        return Err("target_refused_threat");
    }

    // The pet's side, written directly rather than through
    // `generate_threat`: that path would refuse a Passive pet, and an
    // owner's order is obeyed whatever the stance.
    let world = super::super::world_label(space_mgr, pet_id);
    if let Some(pet) = space_mgr.get_entity_mut(pet_id) {
        *pet.threat_list.entry(target_id).or_insert(0.0) += PET_ENGAGE_THREAT;
        if pet.ai_state() != AiState::Fighting {
            super::super::set_ai_state_on(
                pet,
                &world,
                AiState::Fighting,
                super::super::AiTransitionReason::PetEngage,
            );
        }
    }
    Ok(())
}

/// Act on a stance pick: [`engage_pet_target`], then the `pets.ai` row.
/// Returns whether the pet is now fighting.
pub(super) fn engage_stance_pick(
    space_mgr: &mut SpaceManager,
    pet_id: u32,
    owner_id: u32,
    target_id: u32,
    why: EngageWhy,
) -> bool {
    let result = engage_pet_target(space_mgr, pet_id, target_id, PetEngagement::Automatic);
    let id = super::owner_identity(space_mgr, pet_id, owner_id);
    match result {
        Ok(()) => {
            tracing::debug!(
                target: "pets.ai",
                entity_id = pet_id,
                event = "engaged",
                decision_outcome = "pet_engaged",
                pet_id,
                owner_id,
                account_id = id.account_id,
                player_id = id.player_id,
                target_id,
                why = why.label(),
                "pet: stance engaged a target"
            );
            true
        }
        // The stance only picks targets the engagement accepts (alive, not
        // resetting, a combatant mob its owner could attack), so a refusal
        // here is an invariant violation, not something a client can cause.
        Err(reason) => {
            tracing::warn!(
                target: "pets.ai",
                entity_id = pet_id,
                event = "engage_refused",
                decision_outcome = "pet_engage_refused",
                reason,
                pet_id,
                owner_id,
                account_id = id.account_id,
                player_id = id.player_id,
                target_id,
                why = why.label(),
                "pet: stance picked a target but the engagement was refused"
            );
            false
        }
    }
}
