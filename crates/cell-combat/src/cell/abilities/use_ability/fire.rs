//! The post-warmup half of a cast: ammo, `Ability_End`, target resolution.
//!
//! Python's `AbilityInstance.afterWarmup` (`deprecated/python/cell/
//! AbilityManager.py:663-684`): consume ammo, play `Ability_End`, collect
//! the targets, apply the effects. [`super::handle::handle_use_ability`]
//! calls [`fire_cast`] in the same pass when the warmup is zero, and the
//! warmup tick ([`super::warmup::warmup_tick`]) calls it when a warmup
//! expires (AT-10). The launch half (validation, cooldown, `Ability_Begin`)
//! never runs here, so each of these steps runs exactly once per cast.
//!
//! The effects are routed per effect (AB-07, `abilities::effect_routing`).
//! The target pipeline runs the cast's target part first; then
//! `land_routed` lands the user halves on the caster and the beneficial area
//! halves on the caster's allies. That order is deliberate: a user buff
//! (say +Accuracy) landed first would change the same cast's QR roll.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::AbilityDef;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::effect_routing::{land_effects, plan_cast, RoutedCast};
use super::super::messaging::flush_attacker_ammo_stat;

use super::auto_reload::maybe_trigger_auto_reload;
use super::sequence::{
    play_ability_sequence, warn_unanimated_npc_attack, AbilityPhase, PhaseSequence,
};

/// Fire a committed cast.
///
/// `effect_seq` is the `InstanceId` minted at launch, so `Ability_End`
/// matches the `Ability_Begin` the launch sent. The launch has already
/// checked that a player has the ammo; the warmup tick re-checks it before
/// a delayed fire. `wire_target_id` is the target the client sent; a
/// beneficial cast re-resolves from it (AB-01), every other cast ignores it.
#[allow(clippy::too_many_arguments)]
pub(in crate::cell::abilities) async fn fire_cast(
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    wire_target_id: i32,
    effect_seq: i32,
    ability_def: &Option<AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // A pet summon spawns its pet instead of resolving a target (PT-03).
    if let Some(summon) = super::summon::player_summon(space_mgr, entity_id, ability_id) {
        super::summon::fire_summon(
            entity_id,
            ability_id,
            effect_seq,
            summon,
            ability_def,
            tx,
            space_mgr,
        )
        .await;
        return;
    }
    // A deployable places its object instead (deployables Phase 0).
    if let Some(spec) =
        super::super::deployable::player_deployable(space_mgr, entity_id, ability_id)
    {
        super::super::deployable::fire_deploy(
            entity_id,
            ability_id,
            effect_seq,
            spec,
            ability_def,
            tx,
            space_mgr,
        )
        .await;
        return;
    }
    // An owner ability runs its pet effects on the owner's pet (PT-08).
    if super::owner_pet::player_owner_pet_ability(space_mgr, entity_id, ability_id) {
        super::owner_pet::fire_owner_pet(
            entity_id,
            ability_id,
            effect_seq,
            ability_def,
            tx,
            space_mgr,
        )
        .await;
        return;
    }

    // Consume ammo (players only). Routes through `set_slot_ammo` so the
    // AmmoSlot{N} stat updates and the slot is marked dirty for batched
    // persistence (drained on reload completion / slot swap / ammo change /
    // logout — Stage D wires the swap and logout flushes). Python consumes
    // here too, in `afterWarmup`, not at launch.
    let required_ammo = ability_def.as_ref().map_or(0, |d| d.required_ammo);
    let mut needs_ammo_stat_send = false;
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        if required_ammo > 0 && entity.is_player {
            let new_ammo = entity.active_ammo() - required_ammo;
            let slot = entity.active_bandolier_slot;
            entity.set_slot_ammo(slot, new_ammo);
            needs_ammo_stat_send = true;
            let who = entity.identity();
            tracing::debug!(
                target: "abilities",
                event = "ammo_consumed",
                stage = "fire",
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id,
                cast_id = effect_seq,
                ability_id,
                ammo_remaining = entity.active_ammo(),
                "useAbility: consumed ammo"
            );
        }
    }

    // A beneficial cast (a heal, a buff: AB-01) resolves where it lands now,
    // before `Ability_End`, so the animation and the effects name the same
    // entity (an ally who died in the warmup takes the cast to the caster).
    let beneficial =
        super::beneficial::is_player_beneficial(space_mgr, entity_id, ability_def.as_ref()).then(
            || {
                super::beneficial::resolve_at_fire(
                    space_mgr,
                    entity_id,
                    ability_def.as_ref(),
                    wire_target_id,
                )
            },
        );
    let target_id = beneficial.map_or(target_id, |(t, _)| {
        super::beneficial::landing_id(entity_id, t)
    });
    // The in-game combat debug's cast record opens here (AB-N1): the fire is
    // where a zero-warmup cast and a warmup cast meet.
    space_mgr.note_combat_debug(
        entity_id,
        Some(effect_seq),
        ability_id,
        cimmeria_cell_world::cell::combat_debug::Note::Fire {
            target: u32::try_from(target_id).ok().filter(|&t| t != 0),
            beneficial: beneficial.is_some(),
        },
    );
    // Every other cast routes per effect (AB-07): its user halves and its
    // beneficial area halves land off the target, the rest stays for the
    // target pipeline below (`routed.target_def`).
    let routed = if beneficial.is_none() {
        plan_cast(space_mgr, entity_id, ability_def.as_ref(), None)
    } else {
        RoutedCast::default()
    };
    // A cast that only lands on its user (Combat Sprint) animates at the
    // user, as a beneficial Self cast does.
    let sequence_target =
        if target_id <= 0 && routed.target_has_nothing() && routed.lands_on(entity_id) {
            entity_id as i32
        } else {
            target_id
        };

    // ── Send the fire animation (Ability_End) to attacker + witnesses ──
    // The client expects the sequence_id from resources.sequences, NOT the
    // event_set_id. Owner + witnesses like Python `AbilityManager.playSequence`;
    // an NPC attack that cannot animate WARNs there (NA43).
    match ability_def.as_ref() {
        Some(def) => {
            play_ability_sequence(
                PhaseSequence {
                    phase: AbilityPhase::End,
                    entity_id,
                    ability_id,
                    target_id: sequence_target,
                    instance_id: effect_seq,
                    event_set_id: def.event_set_id,
                },
                tx,
                space_mgr,
            )
            .await;
        }
        None => warn_unanimated_npc_attack(
            space_mgr,
            entity_id,
            target_id,
            ability_id,
            None,
            "no_ability_def",
        ),
    }

    // A beneficial cast lands through its own path, never the damage
    // pipeline below: no QR, no threat, no in-combat state, no channel cancel.
    if let Some(resolved) = beneficial {
        super::beneficial::fire_beneficial(entity_id, resolved, ability_def, tx, space_mgr).await;
        if needs_ammo_stat_send {
            flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
        }
        maybe_trigger_auto_reload(entity_id, needs_ammo_stat_send, ability_id, tx, space_mgr).await;
        return;
    }

    fire_at_target(
        CastIds {
            entity_id,
            ability_id,
            target_id,
            effect_seq,
        },
        ability_def,
        &routed,
        needs_ammo_stat_send,
        tx,
        space_mgr,
    )
    .await;

    // The off-target effects land after the target has resolved, so a user
    // buff never changes its own cast's roll, and whatever the target part
    // did: no QR roll, so a miss never drops a user half, and no #444 gate
    // (the launch already dropped a target it would refuse).
    if !routed.landings.is_empty() {
        land_routed(entity_id, ability_id, &routed, tx, space_mgr).await;
    }

    maybe_trigger_auto_reload(entity_id, needs_ammo_stat_send, ability_id, tx, space_mgr).await;
}

/// The ids of one cast.
#[derive(Debug, Clone, Copy)]
struct CastIds {
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    effect_seq: i32,
}

/// The target part of a non-beneficial cast (`routed.target_def`): the
/// support shot, or the damage pipeline and the cone fan-out. Flushes the
/// spent ammo on every path.
async fn fire_at_target(
    ids: CastIds,
    ability_def: &Option<AbilityDef>,
    routed: &RoutedCast,
    needs_ammo_stat_send: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let CastIds {
        entity_id,
        ability_id,
        target_id,
        effect_seq,
    } = ids;

    // ── Combat resolution (if target specified) ──

    if target_id <= 0 || routed.target_has_nothing() {
        // Self-buff or no-target ability — skip damage but still flush any
        // dirty ammo stat (e.g. ground-targeted ability that consumed ammo
        // without picking up a target via auto-aim). A cast that routed
        // every effect off its target has nothing to resolve on it either.
        // Not an error (a Self buff lands through `land_routed`), but the
        // target pipeline did nothing, so say so (AB-T2).
        let who = space_mgr.player_identity(entity_id);
        tracing::debug!(
            target: "abilities",
            event = "fire_target_skipped",
            stage = "fire",
            reason = if target_id <= 0 { "no_target" } else { "all_routed_away" },
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id,
            cast_id = effect_seq,
            ability_id,
            target_id,
            off_target_landings = routed.landings.len(),
            "fire: expected a target to resolve the cast on, none took part (no target, or every effect routed to the caster or allies); no QR roll, no damage, only off-target effects land"
        );
        if needs_ammo_stat_send {
            flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
        }
        return;
    }

    // A support shot (beneficial ammo, AM-11d) runs only its on-hit effect,
    // on an ally or the shooter, and never enters the damage pipeline, the
    // channel cancel or the cone fan-out: no damage, threat or combat state.
    if let Some(shot) =
        super::support_shot::beneficial_shot(space_mgr, entity_id, ability_def.as_ref())
    {
        super::support_shot::fire_support(
            entity_id,
            target_id as u32,
            ability_id,
            shot,
            needs_ammo_stat_send,
            tx,
            space_mgr,
        )
        .await;
        return;
    }

    // Phase J: cancel any channelled effects this attacker started
    // with a DIFFERENT ability. Same-ability re-fire keeps the channel
    // alive (it'll refresh via `register_active_effect`'s same-source
    // rule). Cancellation MUST happen before the new damage applies so
    // the wire ordering reads "old channel cleared, new ability fired".
    crate::cell::effects::cancel_channels_from_attacker(entity_id, Some(ability_id), tx, space_mgr)
        .await;

    // The target takes the cast minus its off-target halves.
    let target_def = &routed.target_def;
    super::super::damage_apply::apply_damage_to_target(
        entity_id,
        target_id as u32,
        ability_id,
        target_def,
        effect_seq as u32,
        needs_ammo_stat_send,
        tx,
        space_mgr,
    )
    .await;

    // Cone AoE fan-out — once the primary takes damage,
    // sweep every effect on this ability for `TCM_AECone` and apply
    // damage to any additional hostiles caught in the cone. Returns
    // alive→dead transitions so the caller's kill-credit wrapper can
    // fire entity_death for each. `target_id > 0` already enforced
    // because we'd have early-returned with no-target above.
    let cone_deaths = super::super::cone_aoe::fan_out_cone_effects(
        entity_id,
        target_id as u32,
        ability_id,
        target_def,
        tx,
        space_mgr,
    )
    .await;
    // Stash the cone deaths on the attacker so
    // `handle_use_ability_with_kill_credit` can pick them up after
    // we return. Persisting via a per-attacker scratchpad keeps the
    // function signature stable for non-kill-credit callers.
    if !cone_deaths.is_empty() {
        if let Some(att) = space_mgr.get_entity_mut(entity_id) {
            att.last_aoe_deaths.extend(cone_deaths);
        }
    }
}

/// Land a non-beneficial cast's off-target effects and log where they went.
/// Runs after [`fire_at_target`], so a user buff never changes the roll of
/// the cast that granted it.
async fn land_routed(
    entity_id: u32,
    ability_id: i32,
    routed: &RoutedCast,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let pulsing = land_effects(entity_id, ability_id, &routed.landings, tx, space_mgr).await;
    let who = space_mgr.player_identity(entity_id);
    let landed: Vec<(i32, u32)> = routed
        .landings
        .iter()
        .map(|l| (l.effect.effect_id, l.recipient))
        .collect();
    tracing::debug!(
        target: "abilities",
        event = "effect_routing_applied",
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id,
        cast_id = space_mgr.current_cast_id(),
        ability_id,
        landed = ?landed,
        landed_player_ids = ?routed
            .landings
            .iter()
            .map(|l| space_mgr.player_identity(l.recipient).player_id)
            .collect::<Vec<_>>(),
        pulsing_registered = pulsing,
        target_effects = ?routed.target_def.as_ref().map(|d| d.effect_ids.as_slice()),
        "off-target effects landed: user halves on the caster, beneficial area halves on allies"
    );
}
