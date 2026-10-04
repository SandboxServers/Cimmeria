//! Pet AI (pets PT-05, issue #570): a pet is an ordinary NPC that the AI tick
//! drives like a mob, with an owner-relative pre-pass in front of the state
//! match (`dispatch::npc_ai_tick`).
//!
//! What the pre-pass adds (decisions D-PT06, D-PT07, D-PT09 in
//! `docs/analysis/pets/README.md`):
//!
//! - [`owner_follow`]: a pet out of a fight is always in `Follow` with its
//!   owner as the target (band 2-5 u, through the ordinary follow handler).
//!   When it falls more than 40 u behind, or its owner is on another floor, it
//!   teleports beside the owner, at most once every 5 s.
//! - [`stance`]: which target, if any, the pet's stance engages. Passive
//!   never engages, even when hit. Defensive engages whatever attacks the
//!   owner or the pet. Aggressive also engages the owner's current target once
//!   the owner is in combat, and hostile NPCs within 15 u of the pet.
//! - [`engage`]: seeding that engagement on both sides
//!   ([`engage_pet_target`], public: an owner's attack order uses it too).
//! - [`defend`]: mirroring the pet's fights into the owner's combat state
//!   (`threatened_mobs`, `BSF_InCombat`).
//!
//! Outside the pre-pass, three seams:
//!
//! - The fight's leash is measured from the owner, never from
//!   `spawn_position` ([`leash_anchor`], called from `fight_target`).
//! - A fight that ends goes straight back to `Follow`, unhealed and without
//!   the walk home or the evade ([`rearm_after_fight`], called from
//!   `leash::begin_leash`).
//! - The ability selector skips abilities the owner toggled off, and
//!   abilities that do nothing on this server yet ([`ability_allowed`]).
//!
//! Ownership is the pet's summoner identity, never the bare owner id: entity
//! ids are reused, and a player given a destroyed owner's id must not be
//! followed, defended, leashed to or put in combat by the old pet
//! ([`live_owner`], PT-01's `PetRegistry::summoner_matches`). A pet whose
//! owner id no longer holds its summoner holds still until
//! `pet_owner_sweep` despawns it.
//!
//! Log target `pets.ai`; the `decision_outcome` values are listed in
//! `docs/architecture/observability.md`.

mod defend;
mod disengage;
mod engage;
mod owner_follow;
mod stance;
#[cfg(test)]
mod tests;

pub(in crate::cell::service::npc_ai) use disengage::rearm_after_fight;
pub use engage::{
    engage_pet_target, target_state_refusal, PetEngagement, OWNER_ORDER_THREAT_CAP,
    PET_ENGAGE_THREAT,
};

use cimmeria_common::Vector3;
use cimmeria_entity::abilities::ability_is_unimplemented;
use cimmeria_entity::cell_entity::PetState;
use cimmeria_entity::cell_entity::{AiState, CellEntity, PetStance, PlayerIdentity};
use tokio::sync::mpsc;

use crate::cell::combat;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{EntityNames, SpaceManager};

/// Whether `entity_id` is a pet (it carries `PetState`).
pub(in crate::cell) fn is_pet(space_mgr: &SpaceManager, entity_id: u32) -> bool {
    space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| e.extensions.contains::<PetState>())
}

/// The owner's identity for a `pets.ai` row about `pet_id`: the one captured
/// when the pet was summoned, which still names the player once the owner id
/// has gone or been reused; else the live owner's while it is a player.
/// `UNKNOWN` (both fields omitted), never a zero.
pub(in crate::cell) fn owner_identity(
    space_mgr: &SpaceManager,
    pet_id: u32,
    owner_id: u32,
) -> PlayerIdentity {
    let captured = space_mgr.pets.summoner_identity(pet_id);
    if captured.is_known() {
        return captured;
    }
    match space_mgr.get_entity(owner_id) {
        Some(o) if o.is_player => o.identity(),
        _ => PlayerIdentity::UNKNOWN,
    }
}

/// A pet's [`EntityNames`] for a `pets.ai` DEBUG row, or none when that
/// level is off. Several of those rows repeat every tick (a pet walking
/// back, a Passive pet being shot), and the names cost NameBook and
/// interner reads (Rule 6: resolve only for a row that is written).
pub(in crate::cell) fn debug_row_names(space_mgr: &SpaceManager, entity_id: u32) -> EntityNames {
    if tracing::enabled!(target: "pets.ai", tracing::Level::DEBUG) {
        space_mgr.entity_names(entity_id)
    } else {
        EntityNames::default()
    }
}

/// [`debug_row_names`] for an entity already in hand.
fn debug_row_names_of(e: &CellEntity) -> EntityNames {
    if tracing::enabled!(target: "pets.ai", tracing::Level::DEBUG) {
        EntityNames::of(e)
    } else {
        EntityNames::default()
    }
}

/// The pet's owner entity, only while it is the player who summoned the pet
/// (`Err` carries the `reason` label otherwise). Entity ids are reused, so the
/// entity at `owner_id` is checked against the pet's summoner capture, not
/// trusted by id.
pub(in crate::cell) fn live_owner(
    space_mgr: &SpaceManager,
    pet_id: u32,
    owner_id: u32,
) -> Result<&CellEntity, &'static str> {
    let Some(owner) = space_mgr.get_entity(owner_id) else {
        return Err("owner_gone");
    };
    if !owner.is_player || !space_mgr.pets.summoner_matches(pet_id, owner.identity()) {
        return Err("owner_identity_mismatch");
    }
    Ok(owner)
}

/// Why a pet may not fight `target` on behalf of `owner`, or `None` when it
/// may. The one rule every pet seam applies (stance picks, targets kept,
/// threat accepted, the owner combat mirror):
///
/// - `target_not_combatant`: not a combatant mob. Only an `SGWMob` fights; an
///   `SGWBeing` (class 0x01) is a non-combatant prop or story actor even with
///   a hostile faction (`combat::generate_threat` refuses it too), and
///   players and pets are never a pet's targets today.
/// - `target_not_hostile`: its owner could not attack it
///   ([`combat::player_may_attack`], the #444 rule, the seam duels widen).
pub fn fight_refusal(owner: &CellEntity, target: &CellEntity) -> Option<&'static str> {
    if target.is_player || target.class_id != crate::mercury::SGWMOB_CLASS_ID {
        return Some("target_not_combatant");
    }
    // The no-duel form of the rule: a pet never joins its owner's duel (the
    // default until the owner decides otherwise), so no duel can widen it.
    if !combat::player_may_attack_pve(owner, target) {
        return Some("target_not_hostile");
    }
    None
}

/// Whether the pet `target` refuses threat from `attacker`: something it may
/// not fight ([`fight_refusal`]: a player, another pet, a being, a
/// non-hostile NPC), or anything at all while Passive (D-PT09). A
/// friendly player's hit or a content chain aiming threat at a pet never
/// turns the pet on them. `None` to accept, else the `reason`.
/// `combat::generate_threat` asks before it adds threat or preempts the pet
/// into Fighting.
pub(in crate::cell) fn threat_refusal(
    space_mgr: &SpaceManager,
    target: &CellEntity,
    attacker_id: u32,
) -> Option<&'static str> {
    let pet = target.extensions.get::<PetState>()?;
    if pet.stance == PetStance::Passive {
        return Some("passive_stance");
    }
    let owner = live_owner(space_mgr, target.entity_id.0 as u32, pet.owner_id).ok();
    let attacker = space_mgr.get_entity(attacker_id);
    // An attacker outside the pet's space never gets on its list.
    if attacker.is_some_and(|a| a.space_id != target.space_id) {
        return Some("attacker_other_space");
    }
    match (owner, attacker) {
        (Some(o), Some(a)) if fight_refusal(o, a).is_none() => None,
        _ => Some("attacker_not_hostile"),
    }
}

/// The row for a threat a pet refused ([`threat_refusal`]). DEBUG: a mob
/// shooting a Passive pet writes one per hit, the owner chooses the stance,
/// and a refused attacker is not client-triggerable at will.
pub(in crate::cell) fn log_threat_refusal(
    space_mgr: &SpaceManager,
    target: &CellEntity,
    attacker_id: u32,
    reason: &'static str,
    cause: &str,
) {
    let Some(pet) = target.extensions.get::<PetState>() else {
        return;
    };
    let id = owner_identity(space_mgr, target.entity_id.0 as u32, pet.owner_id);
    let names = debug_row_names_of(target);
    tracing::debug!(
        target: "pets.ai",
        entity_id = target.entity_id.0,
        event = if reason == "passive_stance" {
            "passive_ignored"
        } else {
            "threat_refused"
        },
        decision_outcome = if reason == "passive_stance" {
            "pet_passive_ignored"
        } else {
            "pet_threat_refused"
        },
        reason,
        pet_id = target.entity_id.0,
        owner_id = pet.owner_id,
        account_id = id.account_id,
        player_id = id.player_id,
        entity_name = names.entity_name,
        pet_name = names.entity_name,
        template_id = names.template_id,
        template_name = names.template_name,
        owner_name = id.player_name,
        account_name = id.account_name,
        player_name = id.player_name,
        target_id = attacker_id,
        target_name = space_mgr.entity_label(attacker_id),
        cause,
        "pet: threat refused -- the pet keeps following"
    );
}

/// The row for a pet that entered Fighting through the ordinary threat entry
/// (it was hit, or a content chain aimed threat at it): the Follow -> Fighting
/// edge a stance engagement logs as `engaged`. Called by
/// `combat::generate_threat` after the transition; a no-op for a non-pet.
pub(in crate::cell) fn log_fight_entered(
    space_mgr: &SpaceManager,
    pet_id: u32,
    attacker_id: u32,
    from: AiState,
    cause: &str,
) {
    let Some(owner_id) = space_mgr
        .get_entity(pet_id)
        .and_then(|e| e.extensions.get::<PetState>())
        .map(|p| p.owner_id)
    else {
        return;
    };
    let id = owner_identity(space_mgr, pet_id, owner_id);
    let names = debug_row_names(space_mgr, pet_id);
    tracing::debug!(
        target: "pets.ai",
        entity_id = pet_id,
        event = "fight_entered",
        decision_outcome = "pet_fight_entered",
        pet_id,
        owner_id,
        account_id = id.account_id,
        player_id = id.player_id,
        entity_name = names.entity_name,
        pet_name = names.entity_name,
        template_id = names.template_id,
        template_name = names.template_name,
        owner_name = id.player_name,
        account_name = id.account_name,
        player_name = id.player_name,
        target_id = attacker_id,
        target_name = space_mgr.entity_label(attacker_id),
        from = from.label(),
        cause,
        "pet: entered a fight"
    );
}

/// Whether the AI may pick `ability_id` for `npc`: always for a mob. For a
/// pet, not when its owner toggled the ability off (`SGWPet.toggledAbilities`),
/// and not when the ability has no visible result at all: no damage, no
/// effect script, no event set (`ability_is_unimplemented`, the predicate
/// CM 88 refuses an owner order with). Such a cast is an empty, silent hit,
/// so a pet whose kit is all of them (the Lo'taur, pets PT-11) holds fire
/// instead of standing at its enemy "attacking" with nothing. An ability with
/// no definition is left to the caller, as for a mob.
pub(super) fn ability_allowed(space_mgr: &SpaceManager, npc: &CellEntity, ability_id: i32) -> bool {
    let Some(pet) = npc.extensions.get::<PetState>() else {
        return true;
    };
    !pet.toggled_off.contains(&ability_id)
        && !space_mgr
            .ability_defs
            .get(&ability_id)
            .is_some_and(|d| ability_is_unimplemented(d, &space_mgr.effect_defs))
}

/// What a fighting NPC's leash is measured from: its spawn point, or for a pet
/// its owner's current position. `None` for a pet whose owner is not in its
/// space, or whose owner id now belongs to someone other than its summoner:
/// no leash then, and the owner sweep despawns the pet within one AoI tick.
pub(super) fn leash_anchor(space_mgr: &SpaceManager, npc: &CellEntity) -> Option<Vector3> {
    let Some(pet) = npc.extensions.get::<PetState>() else {
        return npc.spawn_position;
    };
    live_owner(space_mgr, npc.entity_id.0 as u32, pet.owner_id)
        .ok()
        .filter(|o| o.space_id == npc.space_id)
        .map(|o| o.position)
}

/// The owner-relative pre-pass for one AI turn. Returns the state whose
/// handler should run this turn, or `None` when the pre-pass used the turn
/// itself (a teleport, or an owner who is gone). A non-pet NPC gets `Some`
/// of the state it came in with, untouched.
pub(super) async fn pre_pass(
    npc_id: u32,
    ai_state: AiState,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<AiState> {
    let Some((owner_id, stance, current)) = space_mgr.get_entity(npc_id).and_then(|e| {
        e.extensions
            .get::<PetState>()
            .map(|p| (p.owner_id, p.stance, e.ai_state()))
    }) else {
        return Some(ai_state);
    };
    // The dispatcher snapshots every NPC's state before the loop, and an NPC
    // that ran earlier in the loop may have changed this pet's (a mob's hit
    // preempts it into Fighting). Re-arming Follow on the stale snapshot
    // would leave a Follow pet with a live threat list, so decide on the
    // state the pet is in now.
    let ai_state = current;

    if let Some(reason) = owner_unavailable(space_mgr, npc_id, owner_id) {
        // `pet_owner_sweep` (every AoI tick) despawns it; the AI only holds.
        super::record_decision_outcome("pet_owner_missing");
        // The summoner captured at summon: still the right player when the
        // owner entity is gone or its id was reused.
        let id = owner_identity(space_mgr, npc_id, owner_id);
        let names = debug_row_names(space_mgr, npc_id);
        tracing::debug!(
            target: "pets.ai",
            entity_id = npc_id,
            event = "owner_missing",
            decision_outcome = "pet_owner_missing",
            pet_id = npc_id,
            owner_id,
            account_id = id.account_id,
            player_id = id.player_id,
            entity_name = names.entity_name,
            pet_name = names.entity_name,
            template_id = names.template_id,
            template_name = names.template_name,
            owner_name = id.player_name,
            account_name = id.account_name,
            player_name = id.player_name,
            reason,
            "pet: owner not available, holding until the owner sweep despawns it"
        );
        return None;
    }

    defend::sync_owner_combat(npc_id, owner_id, tx, space_mgr).await;

    match ai_state {
        AiState::Fighting if stance == PetStance::Passive => {
            // Switched to Passive mid-fight: drop the fight now.
            rearm_after_fight(
                npc_id,
                super::AiTransitionReason::PetFollow,
                "passive_stance",
                tx,
                space_mgr,
            )
            .await;
        }
        AiState::Fighting => {
            // A target that is walking home (it evades), leaving, or dead is
            // not worth fighting: chasing it only re-pulls it into Fighting
            // once it is home. Drop it; with nobody left the fight handler
            // ends the fight through the pet branch of the leash.
            if disengage::drop_targets_not_worth_fighting(space_mgr, npc_id, owner_id) {
                // A mob forgot the pet: the owner's entry for it goes now.
                defend::sync_owner_combat(npc_id, owner_id, tx, space_mgr).await;
            }
            return Some(ai_state);
        }
        AiState::Despawning
        | AiState::Submit
        | AiState::Error
        | AiState::Dead
        | AiState::Spawning => return Some(ai_state),
        AiState::Leashing => {
            // Leashing reached without `begin_leash` (content, the GM
            // console): a pet has no walk home.
            rearm_after_fight(
                npc_id,
                super::AiTransitionReason::PetFollow,
                "leashing",
                tx,
                space_mgr,
            )
            .await;
        }
        AiState::Idle
        | AiState::Follow
        | AiState::Patrol
        | AiState::Wander
        | AiState::Investigating => {}
    }

    owner_follow::arm_follow(space_mgr, npc_id, owner_id);

    let now = std::time::Instant::now();
    if owner_follow::teleport_if_left_behind(space_mgr, npc_id, owner_id, now, tx).await
        == owner_follow::TeleportCheck::Teleported
    {
        super::record_decision_outcome("pet_teleported");
        return None;
    }

    if let Some((target_id, why)) = stance::pick_engagement(space_mgr, npc_id, owner_id, stance) {
        if engage::engage_stance_pick(space_mgr, npc_id, owner_id, target_id, why) {
            // The mob now lists the pet: mirror the fight to the owner this
            // turn, not on the pet's next one.
            defend::sync_owner_combat(npc_id, owner_id, tx, space_mgr).await;
            return Some(AiState::Fighting);
        }
    }
    Some(AiState::Follow)
}

/// Why the owner cannot anchor the pet this turn, or `None` when it can.
fn owner_unavailable(space_mgr: &SpaceManager, pet_id: u32, owner_id: u32) -> Option<&'static str> {
    let owner = match live_owner(space_mgr, pet_id, owner_id) {
        Ok(owner) => owner,
        Err(reason) => return Some(reason),
    };
    if space_mgr.get_entity_space_id(owner_id) != space_mgr.get_entity_space_id(pet_id) {
        return Some("owner_other_space");
    }
    if crate::cell::combat::is_dead_state(owner.state_field) {
        return Some("owner_dead");
    }
    None
}
