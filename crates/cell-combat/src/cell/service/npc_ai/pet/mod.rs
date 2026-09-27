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
//! - [`defend`]: seeding that engagement, and mirroring the pet's fights into
//!   the owner's combat state (`threatened_mobs`, `BSF_InCombat`).
//!
//! Outside the pre-pass, three seams:
//!
//! - The fight's leash is measured from the owner, never from
//!   `spawn_position` ([`leash_anchor`], called from `fight_target`).
//! - A fight that ends goes straight back to `Follow`, unhealed and without
//!   the walk home or the evade ([`rearm_after_fight`], called from
//!   `leash::begin_leash`).
//! - The ability selector skips abilities the owner toggled off
//!   ([`ability_allowed`]).
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
mod owner_follow;
mod stance;
#[cfg(test)]
mod tests;

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::{AiState, CellEntity, PetStance, PlayerIdentity};
use tokio::sync::mpsc;

use crate::cell::combat;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Whether `entity_id` is a pet (it carries `PetState`).
pub(in crate::cell) fn is_pet(space_mgr: &SpaceManager, entity_id: u32) -> bool {
    space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| e.pet.is_some())
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

/// Whether the pet `target` refuses threat from `attacker`: something its
/// owner could not attack ([`combat::player_may_attack`]: a player, another
/// pet, a non-hostile NPC), or anything at all while Passive (D-PT09). A
/// friendly player's hit or a content chain aiming threat at a pet never
/// turns the pet on them. `None` to accept, else the `reason`.
/// `combat::generate_threat` asks before it adds threat or preempts the pet
/// into Fighting.
pub(in crate::cell) fn threat_refusal(
    space_mgr: &SpaceManager,
    target: &CellEntity,
    attacker_id: u32,
) -> Option<&'static str> {
    let pet = target.pet.as_deref()?;
    if pet.stance == PetStance::Passive {
        return Some("passive_stance");
    }
    let owner = live_owner(space_mgr, target.entity_id.0 as u32, pet.owner_id).ok();
    let attacker = space_mgr.get_entity(attacker_id);
    match (owner, attacker) {
        (Some(o), Some(a)) if combat::player_may_attack(o, a) => None,
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
    let Some(pet) = target.pet.as_deref() else {
        return;
    };
    let id = owner_identity(space_mgr, target.entity_id.0 as u32, pet.owner_id);
    tracing::debug!(
        target: "pets.ai",
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
        target_id = attacker_id,
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
        .and_then(|e| e.pet.as_deref())
        .map(|p| p.owner_id)
    else {
        return;
    };
    let id = owner_identity(space_mgr, pet_id, owner_id);
    tracing::debug!(
        target: "pets.ai",
        event = "fight_entered",
        decision_outcome = "pet_fight_entered",
        pet_id,
        owner_id,
        account_id = id.account_id,
        player_id = id.player_id,
        target_id = attacker_id,
        from = from.label(),
        cause,
        "pet: entered a fight"
    );
}

/// Whether the AI may pick `ability_id` for `npc`: always for a mob, and for a
/// pet unless its owner toggled the ability off (`SGWPet.toggledAbilities`).
pub(super) fn ability_allowed(npc: &CellEntity, ability_id: i32) -> bool {
    npc.pet
        .as_deref()
        .is_none_or(|p| !p.toggled_off.contains(&ability_id))
}

/// What a fighting NPC's leash is measured from: its spawn point, or for a pet
/// its owner's current position. `None` for a pet whose owner is not in its
/// space, or whose owner id now belongs to someone other than its summoner:
/// no leash then, and the owner sweep despawns the pet within one AoI tick.
pub(super) fn leash_anchor(space_mgr: &SpaceManager, npc: &CellEntity) -> Option<Vector3> {
    let Some(pet) = npc.pet.as_deref() else {
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
        e.pet
            .as_deref()
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
        tracing::debug!(
            target: "pets.ai",
            event = "owner_missing",
            decision_outcome = "pet_owner_missing",
            pet_id = npc_id,
            owner_id,
            account_id = id.account_id,
            player_id = id.player_id,
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
            drop_targets_not_worth_fighting(space_mgr, npc_id, owner_id);
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
        if defend::engage(space_mgr, npc_id, owner_id, target_id, why) {
            return Some(AiState::Fighting);
        }
    }
    Some(AiState::Follow)
}

/// Why a fighting pet drops a target, or `None` to keep it. The label is the
/// `reason` on the `target_dropped` row.
fn not_worth_fighting(
    mob: &CellEntity,
    owner: Option<&CellEntity>,
    now: std::time::Instant,
) -> Option<&'static str> {
    // Something its owner could not attack: a player, a pet, or an NPC that
    // is not hostile (content turned it friendly mid-fight, or it reached the
    // threat list some other way). The #444 rule, via the shared predicate.
    if owner.is_none_or(|o| !combat::player_may_attack(o, mob)) {
        return Some("target_not_hostile");
    }
    let owner_pos = owner.map(|o| o.position);
    if matches!(
        mob.ai_state(),
        AiState::Leashing | AiState::Despawning | AiState::Dead
    ) {
        // It evades, is leaving, or is a corpse.
        return Some("target_resetting");
    }
    if mob.leash.reaggro_suppressed(now) {
        // Just finished its reset: hitting it would pull it straight back.
        return Some("target_just_reset");
    }
    // Far from the owner (the owner teleported, or the fight drifted): the
    // pet does not chase what its owner has left behind.
    owner_pos
        .and_then(|o| owner_follow::left_behind(&mob.position, &o))
        .map(|_| "target_far_from_owner")
}

/// Drop the pet's threat entries that are [`not_worth_fighting`]. The
/// ordinary target selection keeps them, since only a dead, gone or
/// out-of-perception target is pruned there. With nobody left the fight
/// handler ends the fight through the pet branch of the leash.
fn drop_targets_not_worth_fighting(space_mgr: &mut SpaceManager, pet_id: u32, owner_id: u32) {
    let now = std::time::Instant::now();
    let owner = space_mgr.get_entity(owner_id);
    let Some(pet) = space_mgr.get_entity(pet_id) else {
        return;
    };
    let dropped: Vec<(u32, &'static str)> = pet
        .threat_list
        .keys()
        .filter_map(|&t| {
            space_mgr
                .get_entity(t)
                .and_then(|m| not_worth_fighting(m, owner, now))
                .map(|why| (t, why))
        })
        .collect();
    if dropped.is_empty() {
        return;
    }
    if let Some(pet) = space_mgr.get_entity_mut(pet_id) {
        for (t, _) in &dropped {
            pet.threat_list.remove(t);
        }
    }
    let id = owner_identity(space_mgr, pet_id, owner_id);
    for (target_id, reason) in dropped {
        tracing::debug!(
            target: "pets.ai",
            event = "target_dropped",
            decision_outcome = "pet_target_dropped",
            pet_id,
            owner_id,
            account_id = id.account_id,
            player_id = id.player_id,
            target_id,
            reason,
            "pet: dropped a target not worth fighting"
        );
    }
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

/// A pet's fight is over: clear it and follow the owner again. This replaces
/// `begin_leash` for a pet, so a pet never walks to `spawn_position`, never
/// evades, and is not healed to full the way a leash reset heals a mob.
///
/// `reason` and `trigger` are what ended the fight, as `begin_leash` got them;
/// they go on the `pet_follow_rearmed` row. The caller has already recorded
/// the tick's `decision_outcome`.
pub(in crate::cell::service::npc_ai) async fn rearm_after_fight(
    npc_id: u32,
    reason: super::AiTransitionReason,
    trigger: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use super::detectors::threat::ThreatClear;

    // No player lists a pet as a threat, but the drain is idempotent and
    // keeps the S7 detector below honest.
    super::leash::drain_player_combat(npc_id, tx, space_mgr).await;
    let Some(owner_id) = space_mgr
        .get_entity(npc_id)
        .and_then(|e| e.pet.as_deref())
        .map(|p| p.owner_id)
    else {
        return;
    };
    let threat_count = {
        let Some(npc) = space_mgr.get_entity_mut(npc_id) else {
            return;
        };
        let n = npc.threat_list.len();
        npc.threat_list.clear();
        npc.ai_retry_at = None;
        npc.leash.target_lost_since = None;
        npc.leash.walk_started_at = None;
        npc.leash.home_route_partial = false;
        n
    };
    super::detectors::threat::check_cleared(
        space_mgr,
        npc_id,
        ThreatClear::ThreatEmpty,
        std::time::Instant::now(),
    );
    crate::cell::cover::release_npc_cover(space_mgr, npc_id, "pet_rearm");
    // Only toward the player who summoned it: a reused owner id gets
    // neither a follower nor combat edits (the pre-pass holds the pet until
    // the sweep takes it).
    if live_owner(space_mgr, npc_id, owner_id).is_ok() {
        owner_follow::arm_follow(space_mgr, npc_id, owner_id);
        // The owner's mirrored combat entries for this fight go now, not on
        // the next pre-pass.
        defend::sync_owner_combat(npc_id, owner_id, tx, space_mgr).await;
    }
    let id = owner_identity(space_mgr, npc_id, owner_id);
    tracing::debug!(
        target: "pets.ai",
        event = "follow_rearmed",
        decision_outcome = "pet_follow_rearmed",
        pet_id = npc_id,
        owner_id,
        account_id = id.account_id,
        player_id = id.player_id,
        reason = reason.label(),
        trigger,
        threat_count,
        "pet: fight over, following the owner again"
    );
}
