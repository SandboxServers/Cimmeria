//! Interrupting a cast in its warmup.
//!
//! Python `AbilityInstance.interrupt` + `AbilityManager.interruptAbility`
//! (`deprecated/python/cell/AbilityManager.py:642-660, 991-1001`): cancel
//! the warmup timer, send the caster a zeroed `AbilityWarmup` timer, play
//! `Ability_Interrupt`, and drop the ability's cooldown. The cast never
//! fires, so no ammo is spent and no damage is dealt.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    serialize_timer_update, TIMER_ABILITY_COOLDOWN, TIMER_ABILITY_WARMUP,
};

use crate::cell::combat;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::super::timer_update::send_timer_update;
use super::super::sequence::{play_ability_sequence, AbilityPhase, PhaseSequence};

/// Why a warmup was interrupted. The label is the `reason` log field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InterruptReason {
    /// The caster died (python `onDead`).
    CasterDied,
    /// The active bandolier slot changed (python `onBandolierSlotChange`).
    BandolierSlotChange,
    /// The caster moved past the channel interrupt distance. Not in the
    /// python server; see the AT-10 worknote.
    CasterMoved,
    /// At fire time the target was gone, dead, or in another space.
    TargetLost,
    /// At fire time the target was beyond the ability's range.
    TargetOutOfRange,
    /// At fire time a player had no line of sight to the target.
    NoLineOfSight,
    /// At fire time a player's weapon was reloading or short of ammo.
    AmmoUnavailable,
    /// A respec removed the warming ability (AT-08).
    AbilityUnlearned,
    /// Another entity's interrupt effect broke it (ability mechanics
    /// AB-09c, `effects::interrupt`).
    Interrupted,
}

impl InterruptReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::CasterDied => "caster_died",
            Self::BandolierSlotChange => "bandolier_slot_change",
            Self::CasterMoved => "caster_moved",
            Self::TargetLost => "target_lost",
            Self::TargetOutOfRange => "target_out_of_range",
            Self::NoLineOfSight => "no_line_of_sight",
            Self::AmmoUnavailable => "ammo_unavailable",
            Self::AbilityUnlearned => "ability_unlearned",
            Self::Interrupted => "interrupt_effect",
        }
    }
}

/// Interrupt `entity_id`'s cast in its warmup, if it has one.
///
/// Returns false, and sends nothing, when there is no cast warming up, so
/// every trigger can call it unconditionally.
///
/// Wire, in python's order:
/// 1. (player) `onTimerUpdate(ability, AbilityWarmup, caster, 0, 0, 0)`, the
///    python cancel.
/// 2. (player) `onTimerUpdate(ability, AbilityCooldown, caster, 0, 0, 0)`.
///    Python dropped the server cooldown but never told the client; this
///    clear keeps the hotbar in step with the refund. Deviation, AT-10.
/// 3. `onSequence(Ability_Interrupt)` to the caster and witnesses, when the
///    ability's event set has one.
///
/// When the interrupted ability is the caster's auto-cycle ability, the loop
/// is cleared too. Python cleared it only on a slot change; with the
/// cooldown refunded the loop would otherwise relaunch the cast on the next
/// tick and a moving player would restart the warmup every few ticks.
pub(crate) async fn interrupt_pending_cast(
    entity_id: u32,
    reason: InterruptReason,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(caster) = space_mgr.get_entity_mut(entity_id) else {
        space_mgr.pending_casts.remove(&entity_id);
        return false;
    };
    let Some(pc) = caster.pending_cast.take() else {
        space_mgr.pending_casts.remove(&entity_id);
        return false;
    };
    let cooldown_refunded = caster.abilities.clear_ability_cooldown(pc.ability_id);
    let is_player = caster.is_player;
    let loop_was_on_this_ability =
        is_player && caster.abilities.auto_cycle_ability_id == Some(pc.ability_id);
    space_mgr.pending_casts.remove(&entity_id);

    tracing::info!(
        target: "abilities",
        event = "warmup_interrupted",
        entity_id,
        ability_id = pc.ability_id,
        target_id = pc.target_id,
        reason = reason.as_str(),
        cooldown_refunded,
        "ability warmup interrupted; the cast did not fire"
    );
    // A deployable's staged ground point dies with its warmup.
    space_mgr.deployables.clear_staged(entity_id);
    super::super::summon::log_summon_interrupted(
        space_mgr,
        entity_id,
        pc.ability_id,
        reason.as_str(),
    );
    // A pet's owner order dies with its warmup: drop it and tell the owner
    // (pets PT-04). A no-op for any cast without a pending order.
    super::pet_order::on_cast_interrupted(entity_id, pc.ability_id, reason.as_str(), tx, space_mgr)
        .await;

    if is_player {
        for timer_type in [TIMER_ABILITY_WARMUP, TIMER_ABILITY_COOLDOWN] {
            let args =
                serialize_timer_update(pc.ability_id, timer_type, entity_id as i32, 0, 0.0, 0.0);
            send_timer_update(entity_id, args, tx, space_mgr).await;
        }
    }

    let event_set_id = space_mgr
        .ability_defs
        .get(&pc.ability_id)
        .and_then(|d| d.event_set_id);
    play_ability_sequence(
        PhaseSequence {
            phase: AbilityPhase::Interrupt,
            entity_id,
            ability_id: pc.ability_id,
            target_id: pc.target_id,
            instance_id: pc.effect_seq,
            event_set_id,
        },
        tx,
        space_mgr,
    )
    .await;

    if loop_was_on_this_ability {
        if let Some(new_state) = combat::clear_auto_cycle(space_mgr, entity_id) {
            crate::cell::abilities::send_auto_cycle_state(entity_id, new_state, tx, space_mgr)
                .await;
        }
    }
    true
}

/// Interrupt `entity_id`'s warming cast when a respec has just removed its
/// ability (AT-08). A parked cast would otherwise fire an ability the
/// player no longer knows once the warmup expires. Returns whether a cast
/// was interrupted; sends nothing when the warming ability, if any, is not
/// in `unlearned`.
pub async fn interrupt_unlearned_cast(
    entity_id: u32,
    unlearned: &[i32],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let warming = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.pending_cast.as_ref())
        .map(|pc| pc.ability_id);
    match warming {
        Some(ability_id) if unlearned.contains(&ability_id) => {
            interrupt_pending_cast(entity_id, InterruptReason::AbilityUnlearned, tx, space_mgr)
                .await
        }
        _ => false,
    }
}
