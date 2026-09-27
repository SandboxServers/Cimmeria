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

use super::super::super::messaging::send_entity_method;
use super::super::sequence::ability_sequence_args;

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

    if is_player {
        for timer_type in [TIMER_ABILITY_WARMUP, TIMER_ABILITY_COOLDOWN] {
            let args =
                serialize_timer_update(pc.ability_id, timer_type, entity_id as i32, 0, 0.0, 0.0);
            send_entity_method(entity_id, 12, args, tx, space_mgr).await; // 12 = onTimerUpdate
        }
    }

    let event_set_id = space_mgr
        .ability_defs
        .get(&pc.ability_id)
        .and_then(|d| d.event_set_id);
    if let Some(event_set_id) = event_set_id {
        use crate::cell::spawner::EVENT_ABILITY_INTERRUPT;
        if let Some(&seq_id) = space_mgr
            .sequence_map
            .get(&(event_set_id, EVENT_ABILITY_INTERRUPT))
        {
            let seq_args = ability_sequence_args(seq_id, entity_id, pc.target_id, pc.effect_seq);
            send_entity_method(entity_id, 1, seq_args, tx, space_mgr).await; // 1 = onSequence
            tracing::debug!(
                target: "abilities.sequence",
                event = "ability_interrupt",
                source_id = entity_id,
                target_id = pc.target_id,
                ability_id = pc.ability_id,
                sequence_id = seq_id,
                event_set_id,
                "onSequence broadcast: Ability_Interrupt (warmup cancelled)"
            );
        }
    }

    if loop_was_on_this_ability {
        if let Some(new_state) = combat::clear_auto_cycle(space_mgr, entity_id) {
            super::super::handle::send_state_field(entity_id, new_state, tx, space_mgr).await;
        }
    }
    true
}
