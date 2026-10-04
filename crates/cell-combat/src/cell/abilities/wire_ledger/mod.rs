//! The wire-send ledger (ability-mechanics AB-T4): one `abilities.wire`
//! DEBUG row for every client-bound send the ability subsystem makes,
//! logged after the send and only when it reached at least one client.
//!
//! **What the row says.** The fields come from the bytes that went out
//! ([`decode`]), so a row is the client's view of the message, not the
//! caller's intent. Every row carries `event = "wire_sent"`, `stage =
//! "wire"`, `method`, `method_index`, `origin` (the sending system),
//! `entity_id` (the entity the method is about) with that entity's
//! `account_id` / `player_id`, `route`, `self_sent`, `witness_count`,
//! `witness_player_ids`, `failed_count` and `cast_id` (the caller's, else
//! the resolving cast scope's, AB-T1). Then, per method:
//!
//! | Method | Fields |
//! |---|---|
//! | `onEffectResults` | `source_id`, `ability_id`, `effect_id` (the cast's `effect_seq`, so equal to `cast_id`), `target_id`, `target_player_id`, `result_code`, `results_count`, `results` (`stat_id:delta,…`) |
//! | `onStatUpdate` | `stat_count`, `stats` (`stat_id:cur/max,…`) |
//! | `onTimerUpdate` | `timer_type` (`warmup` \| `cooldown` \| `duration` \| `category`), `timer_type_code`, `timer_id`, `source_id`, `secondary_id`, `total_secs`, `complete_at`, `action` (`start` \| `clear`); `ability_id` for an ability timer, `effect_id` for a duration |
//! | `onErrorCode` | `system_id`, `instance_id`, `error_code`, `reason`; `ability_id` when the system is the ability system |
//! | `onStateFieldUpdate` | `state_field`, `prev_state_field`, `bits_set`, `bits_cleared` (when the caller knew the old value), `refcounts` (`bit:count,…` after the change), `reason` |
//! | `onSequence` | `sequence_id`, `source_id`, `target_id`, `instance_id`, `ability_id`, `reason` (`ability_begin` \| `ability_end` \| `ability_interrupt` \| `death`) |
//!
//! **Volume.** A fan-out is one row with its witness count, never a row per
//! witness. The decode runs only when the row is enabled, so a production
//! filter without `abilities=debug` pays one interest check per send.
//!
//! **Failures** are the router's WARN (`messaging::deliver`, `event =
//! "wire_send_failed"`); a send that reached nobody writes no row.

mod decode;
mod row;

#[cfg(test)]
mod tests;

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::messaging::{deliver, Delivery, WireRoute};
use cimmeria_entity::cell_entity::PlayerIdentity;
pub(crate) use decode::method_name;
use decode::Decoded;

/// What the caller knows about a send that the bytes do not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireCtx {
    /// The sending system (`damage_apply`, `stat_buffs`, `pulse`, …).
    pub origin: &'static str,
    /// The cast that caused the send, when the caller holds it outside a
    /// cast scope (a pulse, an expiry). Defaults to the scope's.
    pub cast_id: Option<i32>,
    /// The ability, when the payload does not carry it (`onSequence`).
    pub ability_id: Option<i32>,
    /// Why: a refusal reason for `onErrorCode`, the phase for `onSequence`,
    /// the transition for `onStateFieldUpdate`.
    pub reason: Option<&'static str>,
    /// `state_field` before the change, for `bits_set` / `bits_cleared`.
    pub prev_state_field: Option<u32>,
}

impl WireCtx {
    pub const fn new(origin: &'static str) -> Self {
        Self {
            origin,
            cast_id: None,
            ability_id: None,
            reason: None,
            prev_state_field: None,
        }
    }

    pub const fn cast(mut self, cast_id: Option<i32>) -> Self {
        self.cast_id = cast_id;
        self
    }

    pub const fn ability(mut self, ability_id: i32) -> Self {
        self.ability_id = Some(ability_id);
        self
    }

    pub const fn reason(mut self, reason: &'static str) -> Self {
        self.reason = Some(reason);
        self
    }

    pub const fn prev_state(mut self, prev: Option<u32>) -> Self {
        self.prev_state_field = prev;
        self
    }
}

fn row_enabled() -> bool {
    tracing::enabled!(target: "abilities.wire", tracing::Level::DEBUG)
}

/// Route one ability send (see [`WireRoute`]) and write its
/// `abilities.wire` row. Returns what it reached.
pub(crate) async fn send(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    route: WireRoute,
    ctx: WireCtx,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> Delivery {
    let decoded = row_enabled().then(|| Decoded::parse(method_index, &args));
    let delivery = deliver(entity_id, method_index, args, route, tx, space_mgr).await;
    if let Some(decoded) = decoded {
        row::emit(
            Some(space_mgr),
            space_mgr.player_identity(entity_id),
            entity_id,
            method_index,
            &decoded,
            route,
            &delivery,
            &ctx,
        );
    }
    delivery
}

/// The decoded payload of a send the caller makes itself (a direct
/// `EntityMethodCall` to its own player, with its own failure WARN). Take
/// it before the send moves the bytes; call [`PreparedRow::sent_to_owner`]
/// once the send succeeded. Empty when the row is filtered out.
#[derive(Debug)]
pub(crate) struct PreparedRow {
    method_index: u16,
    decoded: Option<Decoded>,
}

pub(crate) fn prepare(method_index: u16, args: &[u8]) -> PreparedRow {
    PreparedRow {
        method_index,
        decoded: row_enabled().then(|| Decoded::parse(method_index, args)),
    }
}

impl PreparedRow {
    /// The send reached `entity_id`'s own client: write the row.
    pub(crate) fn sent_to_owner(self, space_mgr: &SpaceManager, entity_id: u32, ctx: WireCtx) {
        if let Some(decoded) = &self.decoded {
            row::emit(
                Some(space_mgr),
                space_mgr.player_identity(entity_id),
                entity_id,
                self.method_index,
                decoded,
                WireRoute::SelfOnly,
                &Delivery::self_only(),
                &ctx,
            );
        }
    }

    /// [`Self::sent_to_owner`] for a caller holding only the player's
    /// identity, not the `SpaceManager`.
    pub(crate) fn sent_to_owner_as(self, who: PlayerIdentity, entity_id: u32, ctx: WireCtx) {
        if let Some(decoded) = &self.decoded {
            row::emit(
                None,
                who,
                entity_id,
                self.method_index,
                decoded,
                WireRoute::SelfOnly,
                &Delivery::self_only(),
                &ctx,
            );
        }
    }
}
