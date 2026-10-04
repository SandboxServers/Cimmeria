//! A pet leaving a fight, and the mobs it leaves behind (pets PT-05).
//!
//! Two exits: dropping one target that is no longer worth fighting
//! ([`drop_targets_not_worth_fighting`], every Fighting turn), and ending the
//! whole fight ([`rearm_after_fight`], the pet's branch of `begin_leash`, a
//! switch to Passive, or a content-forced Leashing).
//!
//! A fight is two-sided (`engage::engage_pet_target`), so leaving one also
//! decides what happens to the mob's entry for the pet. Per reason:
//!
//! - **Released** (the pet is removed from the mob's threat list, and the
//!   owner's `threatened_mobs` reconciled at once): the target stopped being
//!   fightable (`target_not_combatant`, `target_not_hostile`,
//!   `target_resetting`, `target_just_reset`), or the owner called the pet
//!   off (switched to Passive, or content forced Leashing). The mob has no
//!   business chasing a pet that left a fight it may not be in.
//! - **Kept** (the mob still lists the pet): the pet is pulled back by
//!   distance (the owner-anchored leash, `target_far_from_owner` after an
//!   owner teleport) or lost its target (`target_lost`, `unreachable`).
//!   The mob is still legitimately fighting; it chases the pet the way it
//!   chases a fleeing player, until its own leash resets it and drains the
//!   owner's combat entry. A dead target or an empty list leaves nothing to
//!   release.
//! - A **dismissed or despawned** pet needs no step here: the mob's fight
//!   handler prunes a vanished target, and an emptied list leashes the mob,
//!   whose drain takes the owner out of combat.

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::cell_entity::PetState;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::{defend, fight_refusal, live_owner, owner_follow, owner_identity};

/// Why a fighting pet drops a target, or `None` to keep it. The label is the
/// `reason` on the `target_dropped` row.
fn not_worth_fighting(
    mob: &CellEntity,
    pet_space: cimmeria_common::SpaceId,
    owner: Option<&CellEntity>,
    now: std::time::Instant,
) -> Option<&'static str> {
    // Not in the pet's space (an instance is its own space): however it got
    // on the list, the pet cannot fight it, and it cannot fight the pet.
    if mob.space_id != pet_space {
        return Some("target_other_space");
    }
    // Something the pet may not fight: not a combatant mob, or an NPC its
    // owner could not attack (content turned it friendly mid-fight, or it
    // reached the threat list some other way). Checked every pet turn before
    // the fight handler's `select_target`, so a target that stopped being
    // fightable is dropped before the pet acts on it.
    let Some(o) = owner else {
        return Some("target_not_hostile");
    };
    if let Some(why) = fight_refusal(o, mob) {
        return Some(why);
    }
    let owner_pos = owner.map(|o| o.position);
    // Dead, resetting, leaving, or surrendered / not engageable: the same
    // state rule the pet's automatic engagement applies. A surrendered NPC an
    // owner order sent the pet at is dropped here after that cast, as the
    // pet would not keep fighting it on its own.
    if let Some(why) = super::engage::target_state_refusal(mob, super::PetEngagement::Automatic) {
        return Some(why);
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

/// Whether dropping a target for `reason` also releases the pet from that
/// target's threat list: the target stopped being fightable. A target left
/// behind by distance (`target_far_from_owner`) keeps chasing the pet, and a
/// dead one (`target_dead`) keeps its list for kill credit; the death path
/// already took every player, the owner included, out of combat with it.
fn releases_target(reason: &str) -> bool {
    matches!(
        reason,
        "target_other_space"
            | "target_not_combatant"
            | "target_not_hostile"
            | "target_resetting"
            | "target_just_reset"
            | "target_not_engageable"
    )
}

/// Remove the pet from the threat list of each of `mobs` that lists it.
/// Returns the mobs it was removed from. The caller reconciles the owner's
/// `threatened_mobs` (`defend::sync_owner_combat`).
fn release_pet_from(space_mgr: &mut SpaceManager, pet_id: u32, mobs: &[u32]) -> Vec<u32> {
    mobs.iter()
        .copied()
        .filter(|&m| {
            space_mgr
                .get_entity_mut(m)
                .is_some_and(|e| e.threat_list.remove(&pet_id).is_some())
        })
        .collect()
}

/// The `pets.ai` row for mobs a pet was released from.
fn log_released(
    space_mgr: &SpaceManager,
    pet_id: u32,
    owner_id: u32,
    released: &[u32],
    reason: &'static str,
) {
    if released.is_empty() {
        return;
    }
    let id = owner_identity(space_mgr, pet_id, owner_id);
    let names = super::debug_row_names(space_mgr, pet_id);
    tracing::debug!(
        target: "pets.ai",
        entity_id = pet_id,
        event = "attackers_released",
        decision_outcome = "pet_attackers_released",
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
        reason,
        released = released.len(),
        target_ids = ?released,
        "pet: removed from the threat lists of mobs it left"
    );
}

/// Drop the pet's threat entries that are [`not_worth_fighting`]. The
/// ordinary target selection keeps them, since only a dead, gone or
/// out-of-perception target is pruned there. With nobody left the fight
/// handler ends the fight through the pet branch of the leash.
///
/// A target dropped because it stopped being fightable also forgets the pet
/// (see the module doc); returns whether any mob did, so the caller
/// reconciles the owner's combat state this turn.
pub(super) fn drop_targets_not_worth_fighting(
    space_mgr: &mut SpaceManager,
    pet_id: u32,
    owner_id: u32,
) -> bool {
    let now = std::time::Instant::now();
    let owner = space_mgr.get_entity(owner_id);
    let Some(pet) = space_mgr.get_entity(pet_id) else {
        return false;
    };
    let dropped: Vec<(u32, &'static str)> = pet
        .threat_list
        .keys()
        .filter_map(|&t| {
            space_mgr
                .get_entity(t)
                .and_then(|m| not_worth_fighting(m, pet.space_id, owner, now))
                .map(|why| (t, why))
        })
        .collect();
    if dropped.is_empty() {
        return false;
    }
    if let Some(pet) = space_mgr.get_entity_mut(pet_id) {
        for (t, _) in &dropped {
            pet.threat_list.remove(t);
        }
    }
    let invalid: Vec<u32> = dropped
        .iter()
        .filter(|(_, why)| releases_target(why))
        .map(|&(t, _)| t)
        .collect();
    let released = release_pet_from(space_mgr, pet_id, &invalid);
    log_released(space_mgr, pet_id, owner_id, &released, "target_invalid");
    let id = owner_identity(space_mgr, pet_id, owner_id);
    let names = super::debug_row_names(space_mgr, pet_id);
    for (target_id, reason) in dropped {
        tracing::debug!(
            target: "pets.ai",
            entity_id = pet_id,
            event = "target_dropped",
            decision_outcome = "pet_target_dropped",
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
            target_id,
            target_name = space_mgr.entity_label(target_id),
            reason,
            "pet: dropped a target not worth fighting"
        );
    }
    !released.is_empty()
}

/// A pet's fight is over: clear it and follow the owner again. This replaces
/// `begin_leash` for a pet, so a pet never walks to `spawn_position`, never
/// evades, and is not healed to full the way a leash reset heals a mob.
///
/// `reason` and `trigger` are what ended the fight, as `begin_leash` got them;
/// they go on the `pet_follow_rearmed` row. The caller has already recorded
/// the tick's `decision_outcome`.
///
/// The owner calling the pet off (`trigger` = `passive_stance`, or a
/// content-forced `leashing`) also releases the pet from every mob that
/// lists it. The leash's own triggers (pulled back by distance, target
/// lost or dead, nothing left) keep the mobs' entries: see the module doc.
pub(in crate::cell::service::npc_ai) async fn rearm_after_fight(
    npc_id: u32,
    reason: super::super::AiTransitionReason,
    trigger: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use super::super::detectors::threat::ThreatClear;

    // No player lists a pet as a threat, but the drain is idempotent and
    // keeps the S7 detector below honest.
    super::super::leash::drain_player_combat(npc_id, tx, space_mgr).await;
    let Some(owner_id) = space_mgr
        .get_entity(npc_id)
        .and_then(|e| e.extensions.get::<PetState>())
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
    super::super::detectors::threat::check_cleared(
        space_mgr,
        npc_id,
        ThreatClear::ThreatEmpty,
        std::time::Instant::now(),
    );
    crate::cell::cover::release_npc_cover(space_mgr, npc_id, "pet_rearm");
    if matches!(trigger, "passive_stance" | "leashing") {
        let attackers: Vec<u32> = space_mgr
            .npc_ids_in_space_of(npc_id)
            .into_iter()
            .filter(|&m| {
                space_mgr
                    .get_entity(m)
                    .is_some_and(|e| e.threat_list.contains_key(&npc_id))
            })
            .collect();
        let released = release_pet_from(space_mgr, npc_id, &attackers);
        log_released(space_mgr, npc_id, owner_id, &released, "called_off");
    }
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
    let names = super::debug_row_names(space_mgr, npc_id);
    tracing::debug!(
        target: "pets.ai",
        entity_id = npc_id,
        event = "follow_rearmed",
        decision_outcome = "pet_follow_rearmed",
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
        reason = reason.label(),
        trigger,
        threat_count,
        "pet: fight over, following the owner again"
    );
}
