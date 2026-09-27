//! Summon via ability (pets campaign PT-03, issue #570).
//!
//! A player ability with a `resources.pet_summons` row (PT-S;
//! `SpaceManager::pet_summons`) summons a pet instead of hitting a target.
//! The 2009 data never linked a summon ability to a template: 2826 Summon
//! Straegis and its siblings carry no effects (audit A-26), so there is no
//! effect script to run and the summon is keyed on the ability id.
//!
//! The summon rides the normal cast, with three diversions:
//!
//! 1. **Launch** ([`player_summon`], [`refuse_summon_launch`]). The client's
//!    `target_id` is discarded: the summon is a Self ability and whatever
//!    target the client names plays no part in it. With target 0 the #444
//!    target-validity gate in `handle.rs` never sees the cast, so the gate
//!    itself is unchanged and a non-summon ability aimed at the caster still
//!    fails there. The summon must be in the trained set (a weapon grant
//!    does not count) and its template must be cached, or the press is
//!    refused with visible feedback before the cooldown is charged.
//! 2. **Warmup.** The ability's own warmup is the spawn timer, scaled by the
//!    caster's `speedPet` stat when the ability has the `SpeedPet` flag
//!    (D-PT10, `warmup::effective_warmup`). The warmup's move, death, space
//!    and respec interrupts apply unchanged, and an interrupted warmup never
//!    reaches [`fire_summon`], so it spawns nothing.
//! 3. **Fire** ([`fire_summon`], called from `fire::fire_cast` ahead of the
//!    damage pipeline). Re-check that the pet can be spawned, play the
//!    ability's `Ability_End` (event set 1121 -> 2292 for 2826), despawn the
//!    owner's current pet when the cap is reached (D-PT04), spawn the pet
//!    beside the owner, and queue the target VFX (event set 1122
//!    `Effect_Init` -> 2293) for when the pet has been introduced
//!    (`cimmeria_cell_world::cell::pets::arrival`).
//!
//! NPC casters never summon: [`player_summon`] only answers for players,
//! so an NPC with a summon ability in its set casts it as today.

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::pets::{despawn_pet, PetArrival, PetDespawnReason};
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::super::super::combat;
use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::super::spawner::PetSummon;
use super::sequence::{ability_sequence_args, play_ability_sequence, AbilityPhase, PhaseSequence};

/// "Goauld summon target" (`resources.event_sets` 1122): the ground effect
/// at the summoned pet. It is an effect-level set (only `Effect_Init`), so
/// it cannot sit on the ability row, which carries the source set 1121
/// (PT-S worknote, evidence 3). Every seeded summon today is a Goa'uld
/// summon; a turret summon (later) may need its own set.
pub(crate) const SUMMON_TARGET_EVENT_SET: i32 = 1122;

/// `Effect_Init` (`entities/defs/enumerations.xml`), the event a target
/// effect set plays when the effect lands.
pub(crate) const EVENT_EFFECT_INIT: i32 = 2000;

/// `CONDITION_FEEDBACK_EntityDoesNotHaveAbility`: the summon is not trained
/// (weapon grants do not summon). Same code as the launch's not-known press.
const CONDITION_FEEDBACK_ENTITY_DOES_NOT_HAVE_ABILITY: u16 = 167;

/// `CONDITION_FEEDBACK_InvalidEntity`: the generic refusal, used when the
/// pet cannot be spawned (template missing, spawn failed). The client has no
/// "summon failed" token; the chat line says what happened.
const CONDITION_FEEDBACK_INVALID_ENTITY: u16 = 0;

/// The chat line a player gets when a summon is refused. `onErrorCode`
/// rendering is unverified (AT-E1 worknote, row 2); the `CHAN_FEEDBACK`
/// line is the route known to render.
pub(crate) const SUMMON_FAILED_TEXT: &str = "Your pet could not be summoned.";

/// The chat line for a summon the player has not trained.
pub(crate) const SUMMON_NOT_TRAINED_TEXT: &str = "You have not trained that summon.";

/// The summon row for `ability_id` when `entity_id` is a player, else
/// `None`. The one question every summon diversion asks.
pub(super) fn player_summon(
    space_mgr: &SpaceManager,
    entity_id: u32,
    ability_id: i32,
) -> Option<PetSummon> {
    let summon = space_mgr.pet_summons.pet_summon_for(ability_id)?;
    space_mgr
        .get_entity(entity_id)
        .filter(|e| e.is_player)
        .map(|_| summon)
}

/// The `TargetID` a summon's phase sequences carry: the caster. Python sent
/// `targetId or ent.entityId` (`AbilityManager.py:888-892`), so a Self cast
/// went out targeting its caster; the summon's own cast target stays 0.
/// Anything else keeps the target it was given.
pub(super) fn phase_sequence_target(
    space_mgr: &SpaceManager,
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
) -> i32 {
    if target_id <= 0 && player_summon(space_mgr, entity_id, ability_id).is_some() {
        entity_id as i32
    } else {
        target_id
    }
}

/// Launch-time refusals for a summon, checked after the common validation
/// and before the cooldown is charged. Returns true (feedback sent) when the
/// press is refused.
pub(super) async fn refuse_summon_launch(
    entity_id: u32,
    ability_id: i32,
    summon: PetSummon,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    let trained = space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| e.abilities.has_ability(ability_id));
    let refusal = if !trained {
        Some((
            "not_trained",
            CONDITION_FEEDBACK_ENTITY_DOES_NOT_HAVE_ABILITY,
            SUMMON_NOT_TRAINED_TEXT,
        ))
    } else if !space_mgr.spawn_templates.contains_key(&summon.template_id) {
        Some((
            "unknown_template",
            CONDITION_FEEDBACK_INVALID_ENTITY,
            SUMMON_FAILED_TEXT,
        ))
    } else {
        None
    };
    let Some((reason, code, text)) = refusal else {
        return false;
    };
    tracing::warn!(
        target: "pets.lifecycle",
        decision_outcome = "summon_refused",
        owner_id = entity_id,
        summon_ability_id = ability_id,
        template_id = summon.template_id,
        reason,
        "summon refused at launch; no cooldown charged"
    );
    send_summon_feedback(entity_id, ability_id, code, text, tx).await;
    true
}

/// Why a summon that finished its warmup cannot spawn, if it cannot.
///
/// Checked before the current pet is despawned, so a summon that would fail
/// never costs the player the pet it has.
fn fire_refusal(space_mgr: &SpaceManager, owner: u32, summon: PetSummon) -> Option<&'static str> {
    let Some(caster) = space_mgr.get_entity(owner) else {
        return Some("owner_not_found");
    };
    if combat::is_dead_state(caster.state_field) {
        return Some("owner_dead");
    }
    if space_mgr.get_entity_space_id(owner).is_none() {
        return Some("owner_not_found");
    }
    if !space_mgr.spawn_templates.contains_key(&summon.template_id) {
        return Some("unknown_template");
    }
    None
}

/// Fire a summon whose warmup has completed (or that had none).
///
/// Runs in place of target resolution: a summon deals no damage and enters
/// neither the damage pipeline nor kill credit. The cooldown was charged at
/// launch and stays charged when the spawn is refused here.
pub(super) async fn fire_summon(
    entity_id: u32,
    ability_id: i32,
    effect_seq: i32,
    summon: PetSummon,
    ability_def: &Option<AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let event_set_id = ability_def.as_ref().and_then(|d| d.event_set_id);
    let phase = |phase| PhaseSequence {
        phase,
        entity_id,
        ability_id,
        target_id: 0,
        instance_id: effect_seq,
        event_set_id,
    };

    if let Some(reason) = fire_refusal(space_mgr, entity_id, summon) {
        tracing::warn!(
            target: "pets.lifecycle",
            decision_outcome = "summon_refused",
            owner_id = entity_id,
            summon_ability_id = ability_id,
            template_id = summon.template_id,
            reason,
            "summon refused after its warmup; nothing spawned"
        );
        // The player watched the warmup; show it cancelled, then say why.
        play_ability_sequence(phase(AbilityPhase::Interrupt), tx, space_mgr).await;
        send_summon_feedback(
            entity_id,
            ability_id,
            CONDITION_FEEDBACK_INVALID_ENTITY,
            SUMMON_FAILED_TEXT,
            tx,
        )
        .await;
        return;
    }

    play_ability_sequence(phase(AbilityPhase::End), tx, space_mgr).await;

    // D-PT04: one active pet per owner. Count every pet the owner has, not
    // only this ability's, or cycling through summon abilities would stack
    // pets. Bounded by the list taken up front, so a despawn that refuses
    // cannot spin the loop.
    let current = space_mgr.pets.pets_of(entity_id);
    let cap = summon.max_active.max(1) as usize;
    let excess = (current.len() + 1).saturating_sub(cap);
    for &old_pet in current.iter().take(excess) {
        // `despawn_pet` logs its own outcome and scrubs the registry
        // either way, so the new pet never counts against a stale entry.
        let _outcome = despawn_pet(space_mgr, old_pet, PetDespawnReason::Dismissed, tx).await;
    }

    let pet_id = match space_mgr.spawn_pet_from_template(entity_id, summon.template_id, ability_id)
    {
        Ok(pet_id) => pet_id,
        Err(_) => {
            // `spawn_pet_from_template` has logged the WARN with its reason.
            send_summon_feedback(
                entity_id,
                ability_id,
                CONDITION_FEEDBACK_INVALID_ENTITY,
                SUMMON_FAILED_TEXT,
                tx,
            )
            .await;
            return;
        }
    };

    queue_arrival_vfx(space_mgr, entity_id, pet_id);
}

/// Queue the summon's target VFX for `pet_id`. It is sent once the owner
/// witnesses the pet, i.e. after the pet's CREATE_ENTITY, never ahead of it.
fn queue_arrival_vfx(space_mgr: &mut SpaceManager, owner: u32, pet_id: u32) {
    let Some(&sequence_id) = space_mgr
        .sequence_map
        .get(&(SUMMON_TARGET_EVENT_SET, EVENT_EFFECT_INIT))
    else {
        tracing::debug!(
            target: "pets.lifecycle",
            owner_id = owner,
            pet_id,
            event_set_id = SUMMON_TARGET_EVENT_SET,
            "summon target VFX not in the sequence map; the pet arrives without it"
        );
        return;
    };
    // Python effect sequences: played on the effect's target, source = the
    // invoker, InstanceId 0 (`AbilityManager.py:300-308`, `playSequence`).
    let args = ability_sequence_args(sequence_id, owner, pet_id as i32, 0);
    space_mgr
        .pets
        .queue_arrival(pet_id, PetArrival::new(owner, sequence_id, args));
}

/// `onErrorCode(ERRORCODE_SYSTEM_Ability, ability_id, code)` and a
/// `CHAN_FEEDBACK` line to the summoning player. Both self-only.
async fn send_summon_feedback(
    entity_id: u32,
    ability_id: i32,
    code: u16,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut err = Vec::with_capacity(7);
    err.push(0u8); // SystemID: ERRORCODE_SYSTEM_Ability
    err.extend_from_slice(&ability_id.to_le_bytes()); // InstanceID
    err.extend_from_slice(&code.to_le_bytes());
    let chat = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    for (method_index, args) in [
        (crate::mercury::method_idx::ON_ERROR_CODE, err),
        (crate::mercury::method_idx::ON_PLAYER_COMMUNICATION, chat),
    ] {
        if tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            })
            .await
            .is_err()
        {
            tracing::warn!(
                target: "pets.lifecycle",
                decision_outcome = "summon_feedback_send_failed",
                owner_id = entity_id,
                summon_ability_id = ability_id,
                method_index,
                "summon feedback could not be queued (base channel closed)"
            );
        }
    }
}
