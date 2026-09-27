//! The post-warmup half of a cast: ammo, `Ability_End`, target resolution.
//!
//! Python's `AbilityInstance.afterWarmup` (`deprecated/python/cell/
//! AbilityManager.py:663-684`): consume ammo, play `Ability_End`, collect
//! the targets, apply the effects. [`super::handle::handle_use_ability`]
//! calls [`fire_cast`] in the same pass when the warmup is zero, and the
//! warmup tick ([`super::warmup::warmup_tick`]) calls it when a warmup
//! expires (AT-10). The launch half (validation, cooldown, `Ability_Begin`)
//! never runs here, so each of these steps runs exactly once per cast.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::AbilityDef;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
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
/// a delayed fire.
pub(in crate::cell::abilities) async fn fire_cast(
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    effect_seq: i32,
    ability_def: &Option<AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
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
            tracing::debug!(
                entity_id,
                ability_id,
                ammo_remaining = entity.active_ammo(),
                "useAbility: consumed ammo"
            );
        }
    }

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
                    target_id,
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

    // ── Combat resolution (if target specified) ──

    if target_id <= 0 {
        // Self-buff or no-target ability — skip damage but still flush any
        // dirty ammo stat (e.g. ground-targeted ability that consumed ammo
        // without picking up a target via auto-aim).
        if needs_ammo_stat_send {
            flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
        }
        maybe_trigger_auto_reload(entity_id, needs_ammo_stat_send, ability_id, tx, space_mgr).await;
        return;
    }

    // Phase J: cancel any channelled effects this attacker started
    // with a DIFFERENT ability. Same-ability re-fire keeps the channel
    // alive (it'll refresh via `register_active_effect`'s same-source
    // rule). Cancellation MUST happen before the new damage applies so
    // the wire ordering reads "old channel cleared, new ability fired".
    crate::cell::effects::cancel_channels_from_attacker(entity_id, Some(ability_id), tx, space_mgr)
        .await;

    super::super::damage_apply::apply_damage_to_target(
        entity_id,
        target_id as u32,
        ability_id,
        ability_def,
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
        ability_def,
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

    maybe_trigger_auto_reload(entity_id, needs_ammo_stat_send, ability_id, tx, space_mgr).await;
}
