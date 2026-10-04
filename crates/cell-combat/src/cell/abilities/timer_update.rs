//! `onTimerUpdate` (client method 12) routing: the owning player's own
//! client, and nobody else.
//!
//! **Why only the owner.** The client binds `Event_NetIn_TimerUpdate` to one
//! entity class: `SGWPlayer` (clientIndex 2). The only caller of the bind
//! helper `FUN_00c6f1e0` is the startup sweep at `0x00db3390`, and its 162
//! bindings contain exactly one `onTimerUpdate`, for `"SGWPlayer"`. The
//! inbound dispatcher `Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`)
//! looks a method up under `(clientIndex, methodIndex)` while walking the
//! receiving entity's class chain. An `SGWMob` walks `SGWMob (4)`, then
//! `SGWBeing (1)`, then `SGWSpawnableEntity (0)`, never reaches `SGWPlayer`,
//! and discards the method silently at `0x00c6fa8a` (the drop oracle
//! `0x01590f30`). `SGWPet` (5) walks the same chain through `SGWMob`.
//!
//! Before this module, every NPC shot fanned a cooldown timer out to every
//! witness: colo 2026-09-29 recorded 191 `client.dispatch.method_dropped`
//! rows (method 12, `type_id` 4) in one 45-minute player session. The python
//! reference sent ability timers to `ent.client` only
//! (`deprecated/python/cell/AbilityManager.py:611-652`); its
//! `updateEffectTimer` also sent effect timers to `ent.witnesses`
//! (`:825`), which reaches a handler only when the effect's target is a
//! player, since the dispatch keys on the target entity's class.
//!
//! A dropped method is consumed cleanly (its length is known from the
//! `MethodDescription`) and the rest of the bundle dispatches, so this was
//! wasted bandwidth and oracle noise, not the cause of a lost `onSequence`.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::wire_ledger::{self, WireCtx};
use crate::cell::client_methods::being::ON_TIMER_UPDATE;

/// `true` when the client has no handler for `method_index` on a non-player
/// entity, so sending it to that entity's witnesses is always a silent drop.
///
/// Only `onTimerUpdate` is listed today. Add a method here only with the
/// binding evidence from the `0x00db3390` sweep.
pub(crate) fn unbound_on_non_player(method_index: u16) -> bool {
    method_index == ON_TIMER_UPDATE
}

/// What [`send_timer_update`] did with one timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerRoute {
    /// Sent to the entity's own client.
    Owner,
    /// Not sent: the entity is not a player, so no client can use it.
    NotPlayer,
}

/// Send one serialized `onTimerUpdate` about `entity_id` to that entity's
/// own client.
///
/// A player gets it on their own client. Any other entity (NPC, pet) has no
/// client that can dispatch it, so nothing is sent and the skip is logged
/// at DEBUG with `reason = "not_player"`. That
/// skip is the normal path for every NPC ability, so it is not a warning.
///
/// Every `onTimerUpdate` in the cell goes through here. The witness-fanout
/// helpers in `messaging` refuse method 12 for a non-player entity at WARN,
/// so a caller that bypasses this function is visible in SigNoz.
///
/// A sent timer writes its `abilities.wire` row (AB-T4): cooldown and warmup
/// starts and clears, effect duration starts and clears, category timers.
/// [`send_timer_update_ctx`] names the sending system and the cast.
pub async fn send_timer_update(
    entity_id: u32,
    args: Vec<u8>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> TimerRoute {
    send_timer_update_ctx(
        entity_id,
        args,
        WireCtx::new("ability_timer"),
        tx,
        space_mgr,
    )
    .await
}

/// [`send_timer_update`] with the caller's ledger context: its `origin`,
/// and the `cast_id` of a timer sent outside the cast's scope (an expiry).
pub async fn send_timer_update_ctx(
    entity_id: u32,
    args: Vec<u8>,
    ctx: WireCtx,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> TimerRoute {
    let is_player = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);
    if !is_player {
        tracing::debug!(
            target: "abilities.wire",
            entity_id,
            reason = "not_player",
            "onTimerUpdate not sent: the client binds it on SGWPlayer only, so an NPC timer would be dropped"
        );
        return TimerRoute::NotPlayer;
    }
    // Decoded before the send moves the bytes; logged after it.
    let row = wire_ledger::prepare(ON_TIMER_UPDATE, &args);
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_TIMER_UPDATE,
            args,
        })
        .await
        .is_err()
    {
        let who = space_mgr.player_identity(entity_id);
        crate::cell::abilities::metrics::wire_send_failed_in(
            space_mgr,
            entity_id,
            crate::cell::abilities::metrics::WireMessage::OnTimerUpdate,
        );
        tracing::warn!(
            target: "abilities.wire",
            event = "wire_send_failed",
            method = "onTimerUpdate",
            entity_id,
            account_id = who.account_id,
            player_id = who.player_id,
            origin = ctx.origin,
            reason = "cell_to_base_closed",
            "onTimerUpdate send failed: the cooldown/effect bar will not show on the client"
        );
    } else {
        row.sent_to_owner(space_mgr, entity_id, ctx);
    }
    TimerRoute::Owner
}

#[cfg(test)]
#[path = "timer_update_tests.rs"]
mod tests;
