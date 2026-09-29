//! Beneficial ammo on an ally (ammo campaign AM-11d, issue #1026).
//!
//! A support dart (Stim, Antidote, Coagulant, Adrenaline, AM-11c) is an
//! `ammo_modifiers` row with `beneficial = true`. With one loaded in the
//! active slot, a player's weapon shot is a support shot, and it turns the
//! #444 target rule around:
//!
//! - **An ally or the shooter** ([`SupportTarget::Ally`]): another player
//!   the shooter may not attack (`combat::player_may_attack` is false), in
//!   the same space, or the shooter's own entity. The launch admits it with
//!   the usual range, line-of-sight, ammo and cooldown checks, and the fire
//!   ([`fire_support`]) runs only the ammo's on-hit effect. There is no QR
//!   roll, no damage, no `onEffectResults`, no threat, no in-combat flag and
//!   no duel or PvP state on either player, and no NPC hears about it.
//! - **A hostile target** ([`SupportTarget::Hostile`]): anything
//!   `player_may_attack` admits, a hostile NPC or an engaged duel opponent.
//!   A support shot is refused there: no damage, no effect, and a
//!   `CHAN_FEEDBACK` line on the first press. The launch refuses it before
//!   the cooldown or the ammo is charged; the fire refuses it again when the
//!   target turned hostile, or the ammo changed, during a warmup.
//! - **Anything else** ([`SupportTarget::Other`], a vendor or a friendly
//!   NPC): the #444 gate refuses it exactly as before.
//!
//! Ammo that is not beneficial never reaches this module, so its targeting
//! is unchanged. `damage_apply` also drops a beneficial row's on-hit effect,
//! so no other path (an AoE secondary, a cone) can land a heal on a hostile.

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::duel::DuelResources;
use cimmeria_cell_world::cell::effects::ammo_damage::{self, ShotAmmo};
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::super::super::combat;
use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::messaging::{flush_attacker_ammo_stat, send_entity_method_to_self_and_witnesses};

/// `event` of a support shot that landed on an ally (DEBUG, target `ammo`).
pub(crate) const EVENT_APPLIED: &str = "ammo_support_applied";
/// `event` of a support shot refused (target `ammo`).
pub(crate) const EVENT_REFUSED: &str = "ammo_support_refused";
/// `reason`: the target is hostile (a hostile NPC or a duel opponent).
pub(crate) const REASON_HOSTILE_TARGET: &str = "hostile_target";
/// `reason`: the target went away between the launch and the fire.
pub(crate) const REASON_TARGET_GONE: &str = "target_gone";
/// `reason`: the target is neither an ally nor hostile (a vendor, a friendly
/// NPC) at fire time. The launch leaves such a target to the #444 gate.
pub(crate) const REASON_NOT_AN_ALLY: &str = "not_an_ally";

/// The feedback line for a support shot at a hostile target.
pub(crate) const HOSTILE_FEEDBACK: &str = "Support rounds only affect allies.";

/// Where a support shot is aimed, from [`classify`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SupportTarget {
    /// Another player the shooter may not attack, or the shooter.
    Ally,
    /// A target `combat::player_may_attack` admits.
    Hostile,
    /// Neither: left to the #444 gate.
    Other,
}

/// The beneficial shot `entity_id` fires with `ability`, or `None` when the
/// shot is not a support shot (the flag is off, the caster is not a player,
/// the ability is not a weapon shot, or the loaded ammo is not beneficial).
pub(crate) fn beneficial_shot(
    space_mgr: &SpaceManager,
    entity_id: u32,
    ability: Option<&AbilityDef>,
) -> Option<ShotAmmo> {
    ammo_damage::shot_ammo(
        space_mgr,
        entity_id,
        ability,
        cimmeria_entity::ammo_feature::finite_special(),
    )
    .filter(|s| s.modifier.beneficial)
}

/// Classify `target` for a support shot by `caster`.
pub(crate) fn classify(
    caster: &CellEntity,
    target: &CellEntity,
    duels: &cimmeria_cell_world::cell::duel::DuelRegistry,
) -> SupportTarget {
    if caster.entity_id == target.entity_id {
        return SupportTarget::Ally;
    }
    if combat::player_may_attack(caster, target, duels) {
        return SupportTarget::Hostile;
    }
    if target.is_player && target.space_id == caster.space_id {
        return SupportTarget::Ally;
    }
    SupportTarget::Other
}

/// Whether `caster`'s cast of `ability` at `target` is a support shot at an
/// ally: the warmup re-check admits it where the #444 rule would not.
pub(crate) fn is_support_ally(
    space_mgr: &SpaceManager,
    caster: &CellEntity,
    target: &CellEntity,
    ability: Option<&AbilityDef>,
) -> bool {
    caster.is_player
        && beneficial_shot(space_mgr, caster.entity_id.0 as u32, ability).is_some()
        && classify(caster, target, space_mgr.resources.duels()) == SupportTarget::Ally
}

/// Log `ammo_support_refused` and send the shooter the feedback line.
/// `stage` is `launch` or `fire`.
pub(crate) async fn refuse(
    entity_id: u32,
    target_id: u32,
    ability_id: i32,
    shot: &ShotAmmo,
    stage: &'static str,
    reason: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let who = space_mgr.player_identity(entity_id);
    let target_who = space_mgr.player_identity(target_id);
    // A player can aim at a hostile at will, so this is ordinary play.
    tracing::debug!(
        target: "ammo",
        event = EVENT_REFUSED,
        decision_outcome = "refused",
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id,
        item_id = shot.ammo_item_id,
        ammo_type = shot.ammo_type,
        target_entity_id = target_id,
        target_player_id = target_who.player_id,
        ability_id,
        stage,
        reason,
        "support ammo shot refused: beneficial rounds never affect a hostile target"
    );
    if reason != REASON_HOSTILE_TARGET {
        return;
    }
    let chat = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, HOSTILE_FEEDBACK);
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
            args: chat,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            target: "ammo",
            event = "ammo_support_feedback_send_failed",
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id,
            ammo_type = shot.ammo_type,
            reason = "cell_to_base_closed",
            "support ammo refusal feedback could not be queued (base channel closed)"
        );
    }
}

/// Fire a committed support shot at `target_id`: the damage-free resolve.
///
/// The ammo has been consumed and `Ability_End` played by the caller
/// (`fire::fire_cast`). This re-classifies the target, because a warmup or
/// an ammo change can separate the launch from the fire, then runs the
/// ammo's on-hit effect on an ally, or refuses.
pub(super) async fn fire_support(
    entity_id: u32,
    target_id: u32,
    ability_id: i32,
    shot: ShotAmmo,
    needs_ammo_stat_send: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let verdict = match (
        space_mgr.get_entity(entity_id),
        space_mgr.get_entity(target_id),
    ) {
        (Some(c), Some(t)) if !combat::is_dead_state(t.state_field) => {
            classify(c, t, space_mgr.resources.duels())
        }
        _ => {
            refuse(
                entity_id,
                target_id,
                ability_id,
                &shot,
                "fire",
                REASON_TARGET_GONE,
                tx,
                space_mgr,
            )
            .await;
            flush_ammo(entity_id, needs_ammo_stat_send, tx, space_mgr).await;
            return;
        }
    };
    match verdict {
        SupportTarget::Ally => {}
        SupportTarget::Hostile => {
            refuse(
                entity_id,
                target_id,
                ability_id,
                &shot,
                "fire",
                REASON_HOSTILE_TARGET,
                tx,
                space_mgr,
            )
            .await;
            flush_ammo(entity_id, needs_ammo_stat_send, tx, space_mgr).await;
            return;
        }
        SupportTarget::Other => {
            refuse(
                entity_id,
                target_id,
                ability_id,
                &shot,
                "fire",
                REASON_NOT_AN_ALLY,
                tx,
                space_mgr,
            )
            .await;
            flush_ammo(entity_id, needs_ammo_stat_send, tx, space_mgr).await;
            return;
        }
    }

    let pools_before = pools(space_mgr, target_id);
    let on_hit_effect_id = shot.on_hit_effect_id(space_mgr);
    let effect_def = on_hit_effect_id.and_then(|id| space_mgr.effect_defs.get(&id).cloned());
    if let Some(effect_def) = effect_def.as_ref() {
        if let Some(script_name) = effect_def.script_name.clone() {
            let mut ctx = crate::cell::effects::EffectContext {
                source_id: entity_id,
                target_id,
                effect: effect_def,
                space_mgr,
            };
            crate::cell::effects::dispatch_by_name(&script_name, &mut ctx);
        }
    }

    // The heal reaches the ally's own client and everyone who sees them,
    // the shooter included, as the damage path's stat update does.
    let pools_after = pools(space_mgr, target_id);
    let stat_update = match space_mgr.get_entity_mut(target_id) {
        Some(t) => {
            let update = t.stats.serialize_dirty();
            t.stats.clear_dirty();
            update
        }
        None => Vec::new(),
    };
    if !stat_update.is_empty() {
        send_entity_method_to_self_and_witnesses(
            target_id,
            crate::mercury::method_idx::ON_STAT_UPDATE,
            stat_update,
            tx,
            space_mgr,
        )
        .await;
    }

    // A pulsing on-hit effect keeps pulsing on the ally, as on any target.
    if let Some(effect_def) = effect_def.filter(|e| e.is_pulsing()) {
        let _ = crate::cell::effects::register_active_effect(
            space_mgr,
            target_id,
            entity_id,
            &effect_def,
            std::time::Instant::now(),
            tx,
        )
        .await;
    }

    flush_ammo(entity_id, needs_ammo_stat_send, tx, space_mgr).await;

    let who = space_mgr.player_identity(entity_id);
    let target_who = space_mgr.player_identity(target_id);
    tracing::debug!(
        target: "ammo",
        event = EVENT_APPLIED,
        decision_outcome = "applied",
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id,
        item_id = shot.ammo_item_id,
        ammo_type = shot.ammo_type,
        target_entity_id = target_id,
        target_player_id = target_who.player_id,
        self_target = entity_id == target_id,
        ability_id,
        on_hit_effect_id,
        target_health_before = pools_before.0,
        target_health_after = pools_after.0,
        target_focus_before = pools_before.1,
        target_focus_after = pools_after.1,
        "support ammo shot applied its on-hit effect to an ally"
    );
}

/// The target's current `(Health, Focus)`, for the before and after fields.
fn pools(space_mgr: &SpaceManager, target_id: u32) -> (Option<i32>, Option<i32>) {
    space_mgr.get_entity(target_id).map_or((None, None), |t| {
        (
            t.stats.get(cimmeria_entity::stats::HEALTH).map(|s| s.cur),
            t.stats.get(cimmeria_entity::stats::FOCUS).map(|s| s.cur),
        )
    })
}

async fn flush_ammo(
    entity_id: u32,
    needs_ammo_stat_send: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if needs_ammo_stat_send {
        flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
    }
}
