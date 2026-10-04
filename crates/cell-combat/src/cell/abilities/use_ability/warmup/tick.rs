//! The warmup sweep: interrupt moving casters, fire expired warmups.
//!
//! Runs every 100 ms AoI tick from the cell message loop. It walks
//! `SpaceManager::pending_casts`, so an idle cell pays one empty-set check.

use cimmeria_cell_world::cell::duel::DuelResources;
use cimmeria_entity::cell_entity::PetState;
use std::time::Instant;

use cimmeria_entity::abilities::{AbilityDef, AF_CHANNEL_ALLOWS_MOVEMENT};
use cimmeria_entity::cell_entity::PendingCast;
use tokio::sync::mpsc;

use crate::cell::combat;
use crate::cell::content_events::ContentEvents;
use crate::cell::effects::pulsing::CHANNEL_INTERRUPT_DISTANCE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::interrupt::{interrupt_pending_cast, InterruptReason};

/// Per-tick entry point, wired into the cell message loop.
pub async fn warmup_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    events: &dyn ContentEvents,
) {
    if space_mgr.pending_casts.is_empty() {
        return;
    }
    resolve_warmups(Instant::now(), tx, space_mgr, events).await;
}

/// Interrupt every warming caster that has moved, then fire (or interrupt)
/// every cast whose warmup has expired by `now`. Returns how many casts
/// fired. `now` is a parameter so tests can step past a warmup without
/// sleeping.
pub(crate) async fn resolve_warmups(
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    events: &dyn ContentEvents,
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
            let who = space_mgr.player_identity(entity_id);
            tracing::debug!(
                target: "abilities",
                event = "warmup_candidate_stale",
                stage = "warmup",
                reason = if space_mgr.get_entity(entity_id).is_some() {
                    "no_pending_cast"
                } else {
                    "caster_gone"
                },
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id,
                "warmup tick: expected a warming cast on this caster, found none; the stale candidate is dropped and nothing fires"
            );
            space_mgr.pending_casts.remove(&entity_id);
            continue;
        };
        let ability_def = space_mgr.ability_defs.get(&pc.ability_id).cloned();

        if moved_off_anchor(space_mgr, entity_id, &pc, ability_def.as_ref()) {
            interrupt_pending_cast(entity_id, InterruptReason::CasterMoved, tx, space_mgr).await;
            continue;
        }
        if now < pc.fire_at {
            // Still warming: not a refusal, so TRACE (one row per 100 ms
            // tick per warming caster). The interrupt and fire rows carry
            // the outcome.
            tracing::trace!(
                target: "abilities",
                event = "warmup_pending",
                stage = "warmup",
                account_id = space_mgr.player_identity(entity_id).account_id,
                player_id = space_mgr.player_identity(entity_id).player_id,
                entity_id,
                cast_id = pc.cast_id(),
                ability_id = pc.ability_id,
                remaining_ms = pc.fire_at.saturating_duration_since(now).as_millis() as u64,
                "warmup tick: cast still warming"
            );
            continue;
        }
        if let Some(reason) =
            fire_time_refusal(entity_id, &pc, ability_def.as_ref(), tx, space_mgr).await
        {
            interrupt_pending_cast(entity_id, reason, tx, space_mgr).await;
            continue;
        }
        record_warmup_fire(&pc, now, entity_id, space_mgr);
        fire_due_cast(entity_id, pc, &ability_def, tx, space_mgr, events).await;
        fired += 1;
    }
    fired
}

/// AB-T6: count a warmed cast's fire and its press-to-fire time, from the
/// cell's receipt of the press (`PendingCast::received_at`) to `now`, the
/// tick's clock: any launch delay, the warmup, and the tick's lateness.
fn record_warmup_fire(pc: &PendingCast, now: Instant, entity_id: u32, space_mgr: &SpaceManager) {
    use crate::cell::abilities::metrics;
    metrics::fired(
        metrics::FirePath::Warmup,
        now.saturating_duration_since(pc.received_at),
        metrics::caster_kind(space_mgr, entity_id),
        metrics::world_of(space_mgr, entity_id),
    );
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

    // A beneficial cast (AB-01) re-resolves from the client's target through
    // the launch's resolver: on the caster it fires with no target checks; on
    // an ally it takes the range and sight checks below, against that ally,
    // but not the hostility gate.
    let beneficial =
        super::super::beneficial::is_player_beneficial(space_mgr, entity_id, ability_def);
    let mut checked_target = pc.target_id;
    if beneficial {
        use super::super::beneficial::{resolve_cast_target, CastTarget};
        match resolve_cast_target(space_mgr, entity_id, ability_def, pc.wire_target_id) {
            CastTarget::Caster => return None,
            CastTarget::Ally(id) => checked_target = id as i32,
            CastTarget::Hostile(_) | CastTarget::None => return Some(InterruptReason::TargetLost),
        }
    }

    if checked_target <= 0 {
        return None;
    }
    let target_eid = checked_target as u32;
    let Some(target) = space_mgr.get_entity(target_eid) else {
        return Some(InterruptReason::TargetLost);
    };
    if combat::is_dead_state(target.state_field)
        || space_mgr.get_entity_space_id(entity_id) != space_mgr.get_entity_space_id(target_eid)
    {
        return Some(InterruptReason::TargetLost);
    }
    // The launch's #444 target-validity rule, again (`player_may_attack`: a
    // hostile NPC or the engaged duel partner). Content can turn an NPC
    // friendly, and a duel can end, during the warmup.
    // A support shot at an ally (beneficial ammo, AM-11d) is the one
    // exception; `fire_support` re-classifies the target when it fires.
    if caster.is_player
        && !beneficial
        && !super::super::support_shot::is_support_ally(space_mgr, caster, target, ability_def)
        && !combat::player_may_attack(caster, target, space_mgr.resources.duels())
    {
        return Some(InterruptReason::TargetLost);
    }
    // A pet casts on its owner's order (`petInvokeAbility`, pets PT-04),
    // which applied the pet rule at launch; re-check it here. An AI-driven
    // mob is not (its fight tick picks targets).
    if caster.extensions.contains::<PetState>()
        && super::pet_order::pet_fire_refusal(space_mgr, caster, target).is_some()
    {
        return Some(InterruptReason::TargetLost);
    }

    // Same range rule as the launch check in `handle_use_ability`,
    // `min_range` included for a player (#1016) and the weapon's reach for a
    // `UseWeaponRange` ability (#1017).
    if let Some(failure) = super::super::cast_range::check_cast_range(
        cimmeria_entity::abilities::caster_range_bounds(
            ability_def,
            caster,
            &space_mgr.weapon_ranges,
        ),
        caster.position.distance_to(&target.position),
        caster.is_player,
    ) {
        super::super::cast_range::refuse_out_of_range(
            entity_id,
            pc.ability_id,
            target_eid,
            failure,
            "warmup_fire",
            tx,
            space_mgr,
        )
        .await;
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
    .is_some()
    {
        return Some(InterruptReason::NoLineOfSight);
    }
    // `fire_los` skips NPC shooters, trusting the fight tick's sight check.
    // A pet's owner-ordered cast never went through the fight tick, so its
    // delayed fire gets the fight tick's own test (pets PT-04).
    if caster.extensions.contains::<PetState>()
        && !space_mgr.attack_line_of_sight(entity_id, target_eid, false)
    {
        return Some(InterruptReason::NoLineOfSight);
    }
    None
}

/// Fire an expired, re-validated cast through the post-warmup path.
///
/// The `combat.cast_fire` span and the cast scope (AB-T1) wrap the whole
/// fire, kill credit included, so every row it causes carries the cast's
/// `cast_id`, the one its launch row logged. One span per fired cast, not
/// per tick: the tick itself opens nothing (instrumentation-discipline
/// rule 3).
#[tracing::instrument(
    name = "combat.cast_fire",
    level = "info",
    skip_all,
    fields(
        entity_id = entity_id,
        cast_id = pc.cast_id(),
        ability_id = pc.ability_id,
        target_id = pc.target_id
    )
)]
async fn fire_due_cast(
    entity_id: u32,
    pc: PendingCast,
    ability_def: &Option<AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    events: &dyn ContentEvents,
) {
    let outer_cast = space_mgr.enter_cast_scope(Some(pc.cast_id()));
    fire_and_credit(entity_id, pc, ability_def, tx, space_mgr, events).await;
    space_mgr.exit_cast_scope(outer_cast);
    // The cast's debug lines (AB-N1), now its scope is closed.
    cimmeria_cell_world::cell::combat_debug::flush(tx, space_mgr).await;
}

/// The body of [`fire_due_cast`].
///
/// Player casts go through the same kill credit the launch entry points
/// use (`EntityDeath` for tagged kills, the `entity_health_below` drain), so
/// a quest kill made by a charged ability still counts. NPC casts do not:
/// NPC kills credit nothing, as with the bare `handle_use_ability` the NPC
/// fight tick calls.
async fn fire_and_credit(
    entity_id: u32,
    pc: PendingCast,
    ability_def: &Option<AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    events: &dyn ContentEvents,
) {
    // Take the cast before firing, so a re-entrant launch from inside the
    // resolution (a content chain, an auto-reload) is not refused as busy
    // and the cast can never fire twice.
    if let Some(e) = space_mgr.get_entity_mut(entity_id) {
        e.pending_cast = None;
    }
    space_mgr.pending_casts.remove(&entity_id);
    // Mission kill credit runs for a caster that credits a player: a player
    // itself, or a pet (credited to its owner, pets PT-06). A plain NPC's
    // warmed-up cast credits nobody.
    let is_player = space_mgr.credit_recipient_quiet(entity_id).is_some();

    let who = space_mgr.player_identity(entity_id);
    tracing::debug!(
        target: "abilities",
        event = "warmup_complete",
        stage = "fire",
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id,
        cast_id = pc.cast_id(),
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
                entity_id, deaths, events, tx, space_mgr,
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
        pc.wire_target_id,
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
            events,
            tx,
            space_mgr,
        )
        .await;
    }
    // A pet's owner order engages its target once the cast has fired
    // (pets PT-04); an interrupted warmup never gets here.
    super::pet_order::engage_fired_order(entity_id, pc.ability_id, pc.target_id, tx, space_mgr)
        .await;
}
