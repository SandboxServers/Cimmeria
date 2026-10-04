//! Reload draws from the special-ammo reserve (ammo campaign AM-02, issue
//! #1026; D-AM05).
//!
//! With `ammo.finite_special` on, a reload whose active slot holds a special
//! `cur_ammo_type` (`cimmeria_entity::ammo_type::is_special`) does not refill
//! for free. The cell asks the base to draw `clip_size - current_ammo`
//! rounds (`AmmoReserveRequest::ReloadDraw`); the base removes them from the
//! bag stacks and writes them into the weapon row in one transaction, then
//! answers. On the answer the rounds go into the clip and the ordinary
//! reload warmup starts; the completion tick then leaves the clip alone
//! ([`completion_target`]) instead of refilling to `clip_size`.
//!
//! Loading the rounds when the draw commits, not when the warmup ends, is
//! what keeps D-AM05's "never punish": a slot swap, death or a second reload
//! that cancels the warmup cannot lose rounds that already left the bags.
//!
//! * A short stack loads what is there (`drawn < requested`).
//! * An empty stack refuses the reload: `onErrorCode`
//!   (`CONDITION_FEEDBACK_AmmoCountLessThan`, the legacy "insufficient ammo"
//!   code) plus a feedback line; the clip is untouched and no warmup starts.
//! * Default ammo, and a player with infinite ammo on
//!   (`cimmeria_entity::ammo_infinite`, D-AM09), reload exactly as before:
//!   no draw, but the clip still empties and still needs this reload.
//!
//! While a reserve request is in flight the player cannot start another
//! (`ReloadGate::Blocked`), so two presses cannot both draw for one clip.
//! The base serializes draws on the inventory advisory lock as well, so a
//! stack is never spent twice even across two requests.
//!
//! Shared pieces the switch return (`bandolier::switch_return`) also uses
//! live here: the per-entity [`ReserveState`], the feedback line, the stat
//! flush and the ammo display name.

use std::time::{Duration, Instant};

use cimmeria_entity::ammo_type::is_special;
/// "Hollow Point" for `Bullet_Hollow_Point`; lives in the name book crate
/// with the other closed name tables (NT-01).
pub use cimmeria_names::ammo_name;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use tokio::sync::mpsc;

use super::reload::{start_reload_warmup, ABILITY_RELOAD_WEAPON};
use crate::cell::messages::{AmmoReserveAnswer, AmmoReserveRequest, CellToBaseMsg, ReserveRefusal};
use crate::cell::space_manager::SpaceManager;

/// `CONDITION_FEEDBACK_AmmoCountLessThan` (`EConditionHandlerFeedback`,
/// `error_texts` id 61): the legacy server's "insufficient ammo" refusal
/// (`deprecated/python/cell/AbilityManager.py:548-552`).
pub const CONDITION_FEEDBACK_AMMO_COUNT_LESS_THAN: u16 = 61;

/// How long an unanswered reserve request blocks the next one. The round
/// trip is in-process and normally takes milliseconds; this only stops a
/// lost answer from wedging reload for the rest of the session.
pub const PENDING_TIMEOUT: Duration = Duration::from_secs(5);

/// A reserve request the base has not answered yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingRequest {
    pub slot_id: i32,
    pub instance_id: i32,
    pub sent_at: Instant,
}

impl PendingRequest {
    fn live(&self, now: Instant) -> bool {
        now.duration_since(self.sent_at) < PENDING_TIMEOUT
    }
}

/// Per-player reserve bookkeeping, kept in `CellEntity::extensions` (so it
/// dies with the cell entity on logout or space change, like the clip).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReserveState {
    /// A `ReloadDraw` in flight.
    pub pending_draw: Option<PendingRequest>,
    /// A `SwitchReturn` in flight.
    pub pending_switch: Option<PendingRequest>,
    /// The reload whose rounds were loaded when the draw committed:
    /// `(slot, warmup deadline)`. The completion tick matches the deadline,
    /// so a later free reload of the same slot is never mistaken for it.
    pub preloaded: Option<(i32, Instant)>,
}

/// The player's reserve state, created on first use.
pub fn reserve_state_mut(
    space_mgr: &mut SpaceManager,
    entity_id: u32,
) -> Option<&mut ReserveState> {
    let e = space_mgr.get_entity_mut(entity_id)?;
    if !e.extensions.contains::<ReserveState>() {
        e.extensions.insert(ReserveState::default());
    }
    e.extensions.get_mut::<ReserveState>()
}

/// Whether a reserve request (draw or switch) is still in flight.
pub fn request_in_flight(space_mgr: &SpaceManager, entity_id: u32, now: Instant) -> bool {
    space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.extensions.get::<ReserveState>())
        .is_some_and(|s| {
            s.pending_draw.is_some_and(|p| p.live(now))
                || s.pending_switch.is_some_and(|p| p.live(now))
        })
}

/// What a reload of the active slot has to do about the reserve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadGate {
    /// Refill for free (default ammo, the flag off, infinite ammo on).
    Free,
    /// Draw from the bags first.
    Draw,
    /// A reserve request is in flight; ignore this press.
    Blocked,
}

/// Decide [`ReloadGate`] for `entity_id`'s active slot.
pub fn reload_gate(space_mgr: &SpaceManager, entity_id: u32, finite_special: bool) -> ReloadGate {
    if request_in_flight(space_mgr, entity_id, Instant::now()) {
        return ReloadGate::Blocked;
    }
    if !finite_special {
        return ReloadGate::Free;
    }
    let Some(e) = space_mgr.get_entity(entity_id) else {
        return ReloadGate::Free;
    };
    let Some(player_id) = e.player_id else {
        return ReloadGate::Free;
    };
    if !is_special(e.active_ammo_type()) {
        return ReloadGate::Free;
    }
    if cimmeria_entity::ammo_infinite::is_on(player_id) {
        return ReloadGate::Free;
    }
    ReloadGate::Draw
}

/// Ask the base to draw rounds for the active slot and mark the request in
/// flight. Nothing on the entity changes until the answer.
#[tracing::instrument(name = "ammo.reload_draw", level = "info", skip_all, fields(entity_id))]
pub async fn request_draw(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(e) = space_mgr.get_entity(entity_id) else {
        return;
    };
    let (Some(player_id), slot_id) = (e.player_id, e.active_bandolier_slot) else {
        return;
    };
    let Some(item) = e.bandolier_items.get(&slot_id) else {
        return;
    };
    let (instance_id, ammo_type, clip_before) =
        (item.instance_id, item.cur_ammo_type, item.current_ammo);
    let account_id = e.account_id;
    let who = e.identity();
    if !flush_slot(entity_id, slot_id, tx, space_mgr).await {
        return;
    }
    let req = AmmoReserveRequest::ReloadDraw {
        entity_id,
        player_id,
        slot_id,
        instance_id,
        ammo_type,
        clip_before,
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
            ammo_type,
            "reload draw could not be queued; the clip is unchanged"
        );
        return;
    }
    if let Some(s) = reserve_state_mut(space_mgr, entity_id) {
        s.pending_draw = Some(PendingRequest {
            slot_id,
            instance_id,
            sent_at: Instant::now(),
        });
    }
    tracing::debug!(
        target: "ammo",
        event = "reload_draw_requested",
        account_id,
        account_name = who.account_name,
        player_id,
        player_name = who.player_name,
        entity_id,
        entity_name = who.player_name,
        ammo_type,
        slot_id, // nt:id-only bandolier slot index, not a named object
        instance_id, // nt:id-only weapon inventory instance, no name of its own
        clip_before,
        "special reload: asked the base to draw rounds"
    );
}

/// The refill target for the reload the completion tick is finishing on
/// `slot_id` (deadline `deadline`): the current clip when its rounds were
/// already drawn and loaded, `clip_size` otherwise (a free reload).
pub fn completion_target(
    entity: &mut cimmeria_entity::cell_entity::CellEntity,
    slot_id: i32,
    deadline: Instant,
    clip_size: i32,
) -> i32 {
    let preloaded = entity
        .extensions
        .get_mut::<ReserveState>()
        .and_then(|s| s.preloaded.take_if(|p| *p == (slot_id, deadline)))
        .is_some();
    if preloaded {
        entity
            .bandolier_items
            .get(&slot_id)
            .map_or(clip_size, |i| i.current_ammo)
    } else {
        clip_size
    }
}

/// The base's answer to a reload draw.
#[tracing::instrument(
    name = "ammo.reload_drawn",
    level = "info",
    skip_all,
    fields(entity_id)
)]
pub async fn handle_reload_drawn(
    entity_id: u32,
    player_id: i32,
    slot_id: i32,
    instance_id: i32,
    ammo_type: i32,
    drawn: i32,
    stack_after: i32,
    result: Result<(), ReserveRefusal>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let (who, active_slot) = match space_mgr.get_entity(entity_id) {
        Some(e) if e.player_id == Some(player_id) => (e.identity(), e.active_bandolier_slot),
        _ => {
            // The rounds are in the weapon row already; the next load of
            // the character shows them.
            tracing::info!(
                target: "ammo",
                event = "reload_drawn_stale",
                reason = "entity_gone",
                player_id, // nt:id-only the entity left: no session left to name
                entity_id, // nt:id-only gone or reused: a live label would name another
                ammo_type,
                drawn,
                "reload draw answered for an entity that is no longer this player"
            );
            return;
        }
    };
    let account_id = who.account_id;
    let was_pending = reserve_state_mut(space_mgr, entity_id)
        .and_then(|s| {
            s.pending_draw
                .take_if(|p| p.slot_id == slot_id && p.instance_id == instance_id)
        })
        .is_some();

    match result {
        Err(refusal) => {
            let text = match refusal {
                ReserveRefusal::StackEmpty => {
                    format!("You have no {} rounds left.", ammo_name(ammo_type))
                }
                ReserveRefusal::WeaponChanged | ReserveRefusal::DbError => {
                    "The reload failed. Try again.".to_string()
                }
            };
            tracing::info!(
                target: "ammo",
                event = "reload_refused_feedback",
                reason = refusal.reason(),
                account_id,
                account_name = who.account_name,
                player_id,
                player_name = who.player_name,
                entity_id,
                entity_name = who.player_name,
                ammo_type,
                slot_id, // nt:id-only bandolier slot index, not a named object
                "special reload refused; clip unchanged, feedback sent"
            );
            send_reload_error(entity_id, tx).await;
            send_feedback_line(entity_id, who.player_name, &text, tx).await;
        }
        Ok(()) if drawn <= 0 => {
            // The clip was already full when the base looked: nothing to do.
        }
        Ok(()) => {
            let loaded = space_mgr.get_entity_mut(entity_id).and_then(|e| {
                let current = e
                    .bandolier_items
                    .get(&slot_id)
                    .filter(|i| i.instance_id == instance_id)
                    .map(|i| i.current_ammo)?;
                e.set_slot_ammo(slot_id, current + drawn)
            });
            let Some(clip_after) = loaded else {
                // The weapon left the slot after the base committed; its
                // row carries the rounds.
                tracing::info!(
                    target: "ammo",
                    event = "reload_drawn_stale",
                    reason = "slot_changed",
                    account_id,
                    account_name = who.account_name,
                    player_id,
                    player_name = who.player_name,
                    entity_id,
                    entity_name = who.player_name,
                    ammo_type,
                    slot_id, // nt:id-only bandolier slot index, not a named object
                    instance_id, // nt:id-only weapon inventory instance, no name of its own
                    drawn,
                    "reload draw answered after the weapon left its slot"
                );
                return;
            };
            tracing::debug!(
                target: "ammo",
                event = "reload_drawn_loaded",
                account_id,
                account_name = who.account_name,
                player_id,
                player_name = who.player_name,
                entity_id,
                entity_name = who.player_name,
                ammo_type,
                slot_id, // nt:id-only bandolier slot index, not a named object
                drawn,
                clip_after,
                stack_after,
                "special rounds loaded; starting the reload warmup"
            );
            let reloading = space_mgr
                .get_entity(entity_id)
                .is_some_and(|e| e.reload_complete_at.is_some());
            if was_pending && slot_id == active_slot && !reloading {
                if let Some(deadline) = start_reload_warmup(entity_id, tx, space_mgr).await {
                    if let Some(s) = reserve_state_mut(space_mgr, entity_id) {
                        s.preloaded = Some((slot_id, deadline));
                    }
                }
            } else {
                flush_stats(entity_id, tx, space_mgr).await;
            }
        }
    }
}

/// Route a base answer to its handler.
pub async fn handle_reserve_answer(
    answer: AmmoReserveAnswer,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match answer {
        AmmoReserveAnswer::ReloadDrawn {
            entity_id,
            player_id,
            slot_id,
            instance_id,
            ammo_type,
            drawn,
            stack_after,
            result,
        } => {
            handle_reload_drawn(
                entity_id,
                player_id,
                slot_id,
                instance_id,
                ammo_type,
                drawn,
                stack_after,
                result,
                tx,
                space_mgr,
            )
            .await
        }
        switch @ AmmoReserveAnswer::SwitchReturned { .. } => {
            crate::cell::cell_methods::inventory::bandolier::handle_switch_returned(
                switch, tx, space_mgr,
            )
            .await
        }
    }
}

/// Persist `slot_id`'s clip and type (`BandolierAmmoUpdate`) right before a
/// reserve request. The base handles cell messages in order, so the request
/// then finds the weapon row current and counts rounds from it. Returns
/// `false` (and the caller sends nothing) when the flush cannot be queued.
pub async fn flush_slot(
    entity_id: u32,
    slot_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(e) = space_mgr.get_entity(entity_id) else {
        return false;
    };
    let (Some(player_id), Some(item)) = (e.player_id, e.bandolier_items.get(&slot_id)) else {
        return false;
    };
    let who = e.identity();
    let msg = CellToBaseMsg::BandolierAmmoUpdate {
        player_id,
        slot_id,
        expected_instance_id: item.instance_id,
        current_ammo: item.current_ammo,
        cur_ammo_type: item.cur_ammo_type,
    };
    if tx.send(msg).await.is_err() {
        tracing::warn!(
            target: "ammo",
            event = "reserve_request_send_failed",
            reason = "base_channel_closed",
            player_id,
            player_name = who.player_name,
            entity_id,
            entity_name = who.player_name,
            slot_id, // nt:id-only bandolier slot index, not a named object
            "clip flush before a reserve request could not be queued; nothing sent"
        );
        return false;
    }
    if let Some(e) = space_mgr.get_entity_mut(entity_id) {
        e.bandolier_ammo_dirty.remove(&slot_id);
    }
    true
}

/// `onErrorCode(Ability, 596, AmmoCountLessThan)`: the refused reload.
pub fn reload_error_args() -> Vec<u8> {
    let mut args = Vec::with_capacity(7);
    args.push(0u8); // ERRORCODE_SYSTEM_Ability
    args.extend_from_slice(&ABILITY_RELOAD_WEAPON.to_le_bytes());
    args.extend_from_slice(&CONDITION_FEEDBACK_AMMO_COUNT_LESS_THAN.to_le_bytes());
    args
}

async fn send_reload_error(entity_id: u32, tx: &mpsc::Sender<CellToBaseMsg>) {
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_ERROR_CODE,
            args: reload_error_args(),
        })
        .await;
}

/// One `SYSTEM` feedback line to the player. `entity_name` is the player's,
/// for the send-failure row.
pub async fn send_feedback_line(
    entity_id: u32,
    entity_name: Option<&str>,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
            args,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            target: "ammo",
            event = "feedback_send_failed",
            reason = "base_channel_closed",
            entity_id,
            entity_name,
            "an ammo feedback line could not be queued"
        );
    }
}

/// Send the entity's dirty stats (the AmmoSlot{N} clip counter) now.
pub async fn flush_stats(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let payload = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => {
            let p = e.stats.serialize_dirty();
            e.stats.clear_dirty();
            p
        }
        None => return,
    };
    // `serialize_dirty` always writes a 4-byte count; skip an empty set.
    if payload.len() > 4 {
        crate::cell::abilities::send_entity_method(
            entity_id,
            crate::mercury::method_idx::ON_STAT_UPDATE,
            payload,
            tx,
            space_mgr,
        )
        .await;
    }
}

#[cfg(test)]
#[path = "reload_reserve_tests.rs"]
mod tests;
