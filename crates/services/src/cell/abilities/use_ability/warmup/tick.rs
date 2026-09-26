//! The warmup sweep: interrupt moving casters, fire expired warmups.
//!
//! Runs every 100 ms AoI tick from the cell message loop. It walks
//! `SpaceManager::pending_casts`, so an idle cell pays one empty-set check.

use std::time::Instant;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::abilities::{AbilityDef, AF_CHANNEL_ALLOWS_MOVEMENT};
use cimmeria_entity::cell_entity::PendingCast;
use tokio::sync::mpsc;

use crate::cell::combat;
use crate::cell::effects::pulsing::CHANNEL_INTERRUPT_DISTANCE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::interrupt::{interrupt_pending_cast, InterruptReason};

/// `CONDITION_FEEDBACK_OutsideWeaponRange`, the code the launch range check
/// already sends.
const CONDITION_FEEDBACK_OUTSIDE_WEAPON_RANGE: u16 = 42;

/// Per-tick entry point, wired into the cell message loop.
pub(crate) async fn warmup_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    if space_mgr.pending_casts.is_empty() {
        return;
    }
    resolve_warmups(Instant::now(), tx, space_mgr, engine).await;
}

/// Interrupt every warming caster that has moved, then fire (or interrupt)
/// every cast whose warmup has expired by `now`. Returns how many casts
/// fired. `now` is a parameter so tests can step past a warmup without
/// sleeping.
pub(crate) async fn resolve_warmups(
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> usize {
    let mut casters: Vec<u32> = space_mgr.pending_casts.iter().copied().collect();
    casters.sort_unstable();

    let mut fired = 0;
    for entity_id in casters {
        let Some(pc) = space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.pending_cast.clone())
        else {
            // Entity destroyed, or its cast already resolved: drop the
            // stale candidate.
            space_mgr.pending_casts.remove(&entity_id);
            continue;
        };
        let ability_def = space_mgr.ability_defs.get(&pc.ability_id).cloned();

        if moved_off_anchor(space_mgr, entity_id, &pc, ability_def.as_ref()) {
            interrupt_pending_cast(entity_id, InterruptReason::CasterMoved, tx, space_mgr).await;
            continue;
        }
        if now < pc.fire_at {
            continue;
        }
        if let Some(reason) =
            fire_time_refusal(entity_id, &pc, ability_def.as_ref(), tx, space_mgr).await
        {
            interrupt_pending_cast(entity_id, reason, tx, space_mgr).await;
            continue;
        }
        fire_due_cast(entity_id, pc, &ability_def, tx, space_mgr, engine).await;
        fired += 1;
    }
    fired
}

/// The caster has moved more than [`CHANNEL_INTERRUPT_DISTANCE`] (planar)
/// from where the warmup started, and the ability does not allow movement;
/// or it is in another space, whatever the flag says.
///
/// Uses the channel rule and its exemption flag, because
/// `SGWAbilityManager.def` gives the warmup the same periodic interrupt
/// check it gives channels (`lastWarmUpInterruptTime` beside
/// `lastChannelInterruptTime`). The python server had no movement interrupt.
fn moved_off_anchor(
    space_mgr: &SpaceManager,
    entity_id: u32,
    pc: &PendingCast,
    ability_def: Option<&AbilityDef>,
) -> bool {
    let Some(caster) = space_mgr.get_entity(entity_id) else {
        return false;
    };
    if caster.space_id != pc.space_id {
        return true;
    }
    if ability_def.is_some_and(|d| d.flags & AF_CHANNEL_ALLOWS_MOVEMENT != 0) {
        return false;
    }
    let dx = caster.position.x - pc.anchor.x;
    let dz = caster.position.z - pc.anchor.z;
    (dx * dx + dz * dz).sqrt() >= CHANNEL_INTERRUPT_DISTANCE
}

/// Re-validate a cast whose warmup expired. `None` means fire it.
///
/// Python re-checked nothing in `afterWarmup`: it spent the ammo and applied
/// the effects to the target it had validated at launch, dead or not. Each
/// check here is the conservative server-authoritative choice (AT-10
/// worknote): the world may have changed during the warmup, and a cast that
/// would have been refused at launch is refused at fire. Players get the
/// same `onErrorCode` the launch would have sent for range and line of
/// sight.
async fn fire_time_refusal(
    entity_id: u32,
    pc: &PendingCast,
    ability_def: Option<&AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> Option<InterruptReason> {
    let Some(caster) = space_mgr.get_entity(entity_id) else {
        return Some(InterruptReason::CasterDied);
    };
    // Death interrupts from the death transition; this is the backstop.
    if combat::is_dead_state(caster.state_field) {
        return Some(InterruptReason::CasterDied);
    }

    // The weapon the cast was launched with must still be in the active
    // slot. A slot change interrupts at request time; this catches a weapon
    // swapped into the slot through the inventory, which would otherwise
    // pay for the old weapon's shot (python `onBandolierSlotChange` fired
    // for "the active item was swapped/removed" too).
    if caster.is_player && super::active_weapon_instance(caster) != pc.weapon_instance {
        return Some(InterruptReason::BandolierSlotChange);
    }

    let required_ammo = ability_def.map_or(0, |d| d.required_ammo);
    if required_ammo > 0
        && caster.is_player
        && (caster.reload_complete_at.is_some() || caster.active_ammo() < required_ammo)
    {
        return Some(InterruptReason::AmmoUnavailable);
    }

    if pc.target_id <= 0 {
        return None;
    }
    let target_eid = pc.target_id as u32;
    let Some(target) = space_mgr.get_entity(target_eid) else {
        return Some(InterruptReason::TargetLost);
    };
    if combat::is_dead_state(target.state_field)
        || space_mgr.get_entity_space_id(entity_id) != space_mgr.get_entity_space_id(target_eid)
    {
        return Some(InterruptReason::TargetLost);
    }
    // The launch's #444 target-validity rule, again: a player may only hit a
    // hostile NPC. Content can turn an NPC friendly during the warmup.
    if caster.is_player && (target.is_player || target.faction != combat::HOSTILE_FACTION) {
        return Some(InterruptReason::TargetLost);
    }

    // Same range rule as the launch check in `handle_use_ability`.
    let max_range = ability_def.map_or(30.0, |d| {
        if d.max_range > 0 {
            d.max_range as f32
        } else {
            30.0
        }
    });
    if caster.position.distance_to(&target.position) > max_range {
        if caster.is_player {
            let mut err_args = Vec::with_capacity(7);
            err_args.push(0u8); // SystemID = ERRORCODE_SYSTEM_Ability
            err_args.extend_from_slice(&pc.ability_id.to_le_bytes()); // InstanceID
            err_args.extend_from_slice(&CONDITION_FEEDBACK_OUTSIDE_WEAPON_RANGE.to_le_bytes());
            let _ = tx
                .send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: crate::mercury::method_idx::ON_ERROR_CODE,
                    args: err_args,
                })
                .await;
        }
        return Some(InterruptReason::TargetOutOfRange);
    }

    // Players only, inside; sends onErrorCode 39 itself when it refuses.
    if super::super::fire_los::refuse_without_line_of_sight(
        entity_id,
        pc.ability_id,
        target_eid,
        ability_def,
        tx,
        space_mgr,
    )
    .await
    {
        return Some(InterruptReason::NoLineOfSight);
    }
    None
}

/// Fire an expired, re-validated cast through the post-warmup path.
///
/// Player casts go through the same kill credit the launch entry points
/// use (`EntityDeath` for tagged kills, the `entity_health_below` drain), so
/// a quest kill made by a charged ability still counts. NPC casts do not:
/// NPC kills credit nothing, as with the bare `handle_use_ability` the NPC
/// fight tick calls.
async fn fire_due_cast(
    entity_id: u32,
    pc: PendingCast,
    ability_def: &Option<AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // Take the cast before firing, so a re-entrant launch from inside the
    // resolution (a content chain, an auto-reload) is not refused as busy
    // and the cast can never fire twice.
    let is_player = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => {
            e.pending_cast = None;
            e.is_player
        }
        None => false,
    };
    space_mgr.pending_casts.remove(&entity_id);

    tracing::debug!(
        target: "abilities",
        event = "warmup_complete",
        entity_id,
        ability_id = pc.ability_id,
        target_id = pc.target_id,
        warmup_secs = pc.warmup_secs,
        "ability warmup complete; firing the cast"
    );

    if let (Some(ground), true) = (pc.ground, pc.target_id > 0) {
        let deaths = crate::cell::abilities::dispatch::fire_ground_cast_after_warmup(
            entity_id,
            pc.ability_id,
            pc.target_id as u32,
            pc.effect_seq,
            ground,
            tx,
            space_mgr,
        )
        .await;
        if is_player {
            super::super::kill_credit::credit_ground_deaths(
                entity_id, deaths, engine, tx, space_mgr,
            )
            .await;
        }
        return;
    }

    let was_alive = is_player && super::super::kill_credit::is_live_npc(space_mgr, pc.target_id);
    super::super::fire::fire_cast(
        entity_id,
        pc.ability_id,
        pc.target_id,
        pc.effect_seq,
        ability_def,
        tx,
        space_mgr,
    )
    .await;
    if is_player {
        super::super::kill_credit::credit_single_target(
            entity_id,
            pc.target_id,
            true,
            was_alive,
            engine,
            tx,
            space_mgr,
        )
        .await;
    }
}
