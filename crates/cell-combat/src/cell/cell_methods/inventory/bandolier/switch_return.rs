//! Ammo-type switch returns unfired special rounds to the bags (ammo
//! campaign AM-02, issue #1026; D-AM05).
//!
//! `requestAmmoChange` (AM-03's `ammo_change.rs`) calls
//! [`begin_switch_return`] once the new type is validated and before it
//! touches the slot. With `ammo.finite_special` on:
//!
//! * **Special clip, rounds left** → [`SwitchReturn::Deferred`]. The rounds
//!   leave the clip now (so they cannot be fired while in transit) and the
//!   base returns them to the bags in one transaction with the weapon row
//!   (`AmmoReserveRequest::SwitchReturn`). Its answer finishes the switch in
//!   [`handle_switch_returned`]: everything fit, so the slot takes the new
//!   type with an empty clip; or the bags were full, so the remainder stays
//!   loaded **as the old type**, the switch does not happen, and a feedback
//!   line says so. Those rounds go back on the next switch attempt, once
//!   there is room. No round is deleted or relabelled.
//! * **Default clip to a special type** → the default rounds are dropped
//!   (they are free, D-AM02) and the switch proceeds with an empty clip;
//!   otherwise a full default clip would fire as special ammo.
//! * Anything else, or the flag off → [`SwitchReturn::Proceed`]: the switch
//!   runs exactly as before the campaign.

use std::time::Instant;

use cimmeria_entity::ammo_type::is_special;
use tokio::sync::mpsc;

use super::super::constants::{build_entity_property_args, GENERICPROPERTY_AMMO_TYPE_ID};
use crate::cell::cell_methods::player::world::reload_reserve::{
    ammo_name, flush_slot, flush_stats, request_in_flight, reserve_state_mut, send_feedback_line,
    PendingRequest,
};
use crate::cell::messages::{AmmoReserveAnswer, AmmoReserveRequest, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;

/// What `requestAmmoChange` does after [`begin_switch_return`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchReturn {
    /// Carry on with the switch as usual.
    Proceed,
    /// Stop: the switch finishes (or is refused) when the base answers, or
    /// was dropped because a reserve request is already in flight.
    Deferred,
}

/// [`begin_switch_return_with`] with the process-wide `ammo.finite_special`.
pub async fn begin_switch_return(
    entity_id: u32,
    slot_id: i32,
    new_ammo_type: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> SwitchReturn {
    let finite = cimmeria_entity::ammo_feature::finite_special();
    begin_switch_return_with(entity_id, slot_id, new_ammo_type, finite, tx, space_mgr).await
}

/// Start the switch-return for `slot_id` moving to `new_ammo_type` (already
/// validated by the caller). See the module docs.
#[tracing::instrument(
    name = "ammo.switch_return",
    level = "info",
    skip_all,
    fields(entity_id, slot_id)
)]
pub(crate) async fn begin_switch_return_with(
    entity_id: u32,
    slot_id: i32,
    new_ammo_type: i32,
    finite_special: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> SwitchReturn {
    if !finite_special {
        return SwitchReturn::Proceed;
    }
    let Some(e) = space_mgr.get_entity(entity_id) else {
        return SwitchReturn::Proceed;
    };
    let (Some(player_id), account_id) = (e.player_id, e.account_id) else {
        return SwitchReturn::Proceed;
    };
    let who = e.identity();
    let Some(item) = e.bandolier_items.get(&slot_id) else {
        return SwitchReturn::Proceed;
    };
    let (instance_id, old_type, clip) = (item.instance_id, item.cur_ammo_type, item.current_ammo);
    if old_type == new_ammo_type {
        return SwitchReturn::Proceed;
    }
    if request_in_flight(space_mgr, entity_id, Instant::now()) {
        tracing::debug!(
            target: "ammo",
            event = "ammo_switch_dropped",
            reason = "request_in_flight",
            account_id,
            account_name = who.account_name,
            player_id,
            player_name = who.player_name,
            entity_id,
            entity_name = who.player_name,
            ammo_type = old_type,
            to_ammo_type = new_ammo_type,
            "ammo switch ignored: a reserve request is still in flight"
        );
        return SwitchReturn::Deferred;
    }

    if is_special(old_type) && clip > 0 {
        if !flush_slot(entity_id, slot_id, tx, space_mgr).await {
            return SwitchReturn::Deferred;
        }
        let req = AmmoReserveRequest::SwitchReturn {
            entity_id,
            player_id,
            slot_id,
            instance_id,
            from_ammo_type: old_type,
            to_ammo_type: new_ammo_type,
            rounds: clip,
        };
        if tx.send(CellToBaseMsg::AmmoReserve(req)).await.is_err() {
            tracing::warn!(
                target: "ammo",
                event = "reserve_request_send_failed",
                reason = "base_channel_closed",
                account_id,
                account_name = who.account_name,
                player_id,
                player_name = who.player_name,
                entity_id,
                entity_name = who.player_name,
                ammo_type = old_type,
                "switch return could not be queued; the switch did not happen"
            );
            return SwitchReturn::Deferred;
        }
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            // A reload in flight on this slot is over: its rounds (if the
            // reserve loaded them) are among the ones going back.
            if e.reload_slot_id == Some(slot_id) {
                e.reload_complete_at = None;
                e.reload_slot_id = None;
            }
            // The rounds are in transit. The base writes the weapon row in
            // the return's transaction, so no clip flush may overtake it.
            e.set_slot_ammo(slot_id, 0);
            e.bandolier_ammo_dirty.remove(&slot_id);
        }
        if let Some(s) = reserve_state_mut(space_mgr, entity_id) {
            s.preloaded = None;
            s.pending_switch = Some(PendingRequest {
                slot_id,
                instance_id,
                sent_at: Instant::now(),
            });
        }
        tracing::debug!(
            target: "ammo",
            event = "ammo_switch_return_requested",
            account_id,
            account_name = who.account_name,
            player_id,
            player_name = who.player_name,
            entity_id,
            entity_name = who.player_name,
            ammo_type = old_type,
            to_ammo_type = new_ammo_type,
            slot_id, // nt:id-only bandolier slot index, not a named object
            rounds = clip,
            "ammo switch: returning unfired special rounds to the bags"
        );
        flush_stats(entity_id, tx, space_mgr).await;
        return SwitchReturn::Deferred;
    }

    if !is_special(old_type) && is_special(new_ammo_type) && clip > 0 {
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            e.set_slot_ammo(slot_id, 0);
        }
        tracing::debug!(
            target: "ammo",
            event = "ammo_switch_default_emptied",
            account_id,
            account_name = who.account_name,
            player_id,
            player_name = who.player_name,
            entity_id,
            entity_name = who.player_name,
            ammo_type = old_type,
            to_ammo_type = new_ammo_type,
            slot_id, // nt:id-only bandolier slot index, not a named object
            clip_before = clip,
            clip_after = 0,
            "ammo switch to a special type: the free default rounds leave the clip"
        );
        flush_stats(entity_id, tx, space_mgr).await;
    }
    SwitchReturn::Proceed
}

/// The base's answer to a switch return: finish or refuse the switch.
#[tracing::instrument(name = "ammo.switch_returned", level = "info", skip_all)]
pub async fn handle_switch_returned(
    answer: AmmoReserveAnswer,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let AmmoReserveAnswer::SwitchReturned {
        entity_id,
        player_id,
        slot_id,
        instance_id,
        from_ammo_type,
        to_ammo_type,
        rounds,
        returned,
        remainder,
        result,
    } = answer
    else {
        return;
    };
    let who = match space_mgr.get_entity(entity_id) {
        Some(e) if e.player_id == Some(player_id) => e.identity(),
        _ => {
            tracing::info!(
                target: "ammo",
                event = "switch_returned_stale",
                reason = "entity_gone",
                player_id, // nt:id-only the entity left: no session left to name
                entity_id, // nt:id-only gone or reused: a live label would name another
                ammo_type = from_ammo_type,
                returned,
                remainder,
                "switch return answered for an entity that is no longer this player; \
                 the weapon row is already settled"
            );
            return;
        }
    };
    let account_id = who.account_id;
    if let Some(s) = reserve_state_mut(space_mgr, entity_id) {
        s.pending_switch
            .take_if(|p| p.slot_id == slot_id && p.instance_id == instance_id);
    }

    // Settle the slot: (clip, type in the slot, did the switch happen).
    let (clip, slot_type, switched) = match result {
        Ok(()) if remainder == 0 => (0, to_ammo_type, true),
        Ok(()) => (remainder, from_ammo_type, false),
        // Nothing moved on the base: the rounds come back to the clip.
        Err(_) => (rounds, from_ammo_type, false),
    };
    let (applied, is_active) = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => {
            let is_active = e.active_bandolier_slot == slot_id;
            let applied = match e.bandolier_items.get_mut(&slot_id) {
                Some(item) if item.instance_id == instance_id => {
                    item.cur_ammo_type = slot_type;
                    true
                }
                _ => false,
            };
            if applied {
                e.set_slot_ammo(slot_id, clip);
                if result.is_ok() {
                    // The base wrote the weapon row in the return's commit.
                    e.bandolier_ammo_dirty.remove(&slot_id);
                }
            }
            (applied, is_active)
        }
        None => (false, false),
    };
    if !applied {
        tracing::info!(
            target: "ammo",
            event = "switch_returned_stale",
            reason = "slot_changed",
            account_id,
            account_name = who.account_name,
            player_id,
            player_name = who.player_name,
            entity_id,
            entity_name = who.player_name,
            ammo_type = from_ammo_type,
            slot_id, // nt:id-only bandolier slot index, not a named object
            returned,
            remainder,
            "switch return answered after the weapon left its slot"
        );
        return;
    }

    if !switched {
        let text = match result {
            Ok(()) => format!(
                "Your bags are full: {remainder} {} rounds stay loaded. \
                 Make room and switch again.",
                ammo_name(from_ammo_type)
            ),
            Err(_) => "The ammo switch failed. Try again.".to_string(),
        };
        tracing::info!(
            target: "ammo",
            event = "ammo_switch_refused",
            reason = match result {
                Ok(()) => "bags_full",
                Err(r) => r.reason(),
            },
            account_id,
            account_name = who.account_name,
            player_id,
            player_name = who.player_name,
            entity_id,
            entity_name = who.player_name,
            ammo_type = from_ammo_type,
            to_ammo_type,
            slot_id, // nt:id-only bandolier slot index, not a named object
            returned,
            remainder,
            "ammo switch did not happen; the unfitted rounds stay loaded as the old type"
        );
        send_feedback_line(entity_id, who.player_name, &text, tx).await;
    }
    // The client's picker already shows the new choice; tell it the type
    // the slot really holds either way.
    if is_active {
        crate::cell::abilities::send_entity_method(
            entity_id,
            crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY,
            build_entity_property_args(GENERICPROPERTY_AMMO_TYPE_ID, slot_type),
            tx,
            space_mgr,
        )
        .await;
    }
    flush_stats(entity_id, tx, space_mgr).await;
}

#[cfg(test)]
#[path = "switch_return_tests.rs"]
mod tests;
