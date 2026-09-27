//! Atomic player-to-player trade execution — entry point + types.
//!
//! Both participants have already reached `ETRADELOCKSTATE_LockedAndConfirmed`
//! on the cell side; the cell has cleared its in-memory state and handed
//! the snapshot off as `CellToBaseMsg::ExecuteTrade`. This module owns
//! the final commit:
//!
//! 1. `BEGIN` a single sqlx transaction.
//! 2. Take both players' inventory advisory locks (the shared order in
//!    `crate::base::crafting::inventory_locks`, lower `player_id` first).
//! 3. `FOR UPDATE` lock every item row each player is offering, and
//!    re-validate ownership, `bound` and the source bag (TOCTOU window
//!    between cell snapshot and base commit).
//! 4. Choose each item's destination bag from its `container_sets` and
//!    reserve free slots there for each recipient.
//! 5. `FOR UPDATE` lock both `sgw_player` rows (naquadah) and re-validate
//!    the cash offers.
//! 6. UPDATE `character_id` on each item row to the recipient + bump
//!    container/slot to the reserved destination.
//! 7. Debit / credit `sgw_player.naquadah`.
//! 8. COMMIT.
//! 9. Push `onCashChanged` + `onUpdateItem` (full inventory) + final
//!    `onTradeResults` to both clients.
//!
//! On any failure between (1) and (8): rollback, send each client its
//! `onTradeResults` code and, where the client shows nothing for that
//! code or its generic line hides the cause, a feedback line saying why.
//! **No items are lost** — the rollback undoes every UPDATE and the cash
//! debit.
//!
//! ## Module layout
//!
//! - [`swap`] — the atomic-swap transaction (locks, item validation,
//!   two-phase parked-row item move, cash debit/credit).
//! - [`placement`] — which bags a trade takes from, where each item
//!   lands, and the slot reservation.
//! - [`abort`] — the abort reasons, their result codes, labels and
//!   feedback lines.
//! - `tests` (cfg-only) — unit guards for the pure pieces. Live-DB
//!   integration tests live in `super::tests`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::trade::{
    serialize_on_trade_results, ETRADERESULTS_CANCELLED, ETRADERESULTS_COMPLETED,
};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::super::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::super::vendor::helpers::send_cash_changed_to_client;
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::base::helpers;
use crate::mercury::{build_player_entity_method_packet, method_idx};

mod abort;
mod placement;
mod swap;

#[cfg(test)]
mod tests;

pub(super) use abort::TradeAbort;
use abort::{
    refusal_container, refusal_lines, trade_abort_outcome_label, trade_abort_to_results_codes,
};
#[cfg(test)]
pub(super) use abort::{LOCAL_CRAFTING_BAG_FULL, LOCAL_UNTRADEABLE_ITEM, REMOTE_CRAFTING_BAG_FULL};
use placement::ItemMove;
use swap::atomic_swap;

/// One side of a trade — the data the atomic commit needs to swap items
/// + cash from `from_player` to `to_player`.
pub(super) struct TradeSide {
    /// Entity id of the player on this side (used for the wire packets).
    pub(super) entity_id: u32,
    /// `sgw_player.player_id` for this side.
    pub(super) player_id: i32,
    /// Inventory item instance ids this side is offering.
    pub(super) item_instance_ids: Vec<i32>,
    /// Cash this side is offering.
    pub(super) cash: i32,
}

/// Atomic execution of a confirmed trade. Sends per-side `onTradeResults`
/// (Completed or Cancelled) to both clients.
///
/// `entity_id`/`player_id` is p1; `partner_entity_id`/`partner_player_id`
/// is p2. The semantic distinction matters only for log-correlation —
/// the swap is fully symmetric.
#[tracing::instrument(
    name = "trade.execute",
    level = "info",
    skip_all,
    fields(
        p1_entity = entity_id,
        p1_player = player_id,
        p2_entity = partner_entity_id,
        p2_player = partner_player_id,
        p1_items = p1_item_instance_ids.len(),
        p1_cash,
        p2_items = p2_item_instance_ids.len(),
        p2_cash,
    )
)]
pub async fn handle_execute_trade(
    entity_id: u32,
    player_id: i32,
    partner_entity_id: u32,
    partner_player_id: i32,
    p1_item_instance_ids: Vec<i32>,
    p1_cash: i32,
    p2_item_instance_ids: Vec<i32>,
    p2_cash: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p.clone(),
        None => {
            tracing::warn!(
                entity_id,
                partner_entity_id,
                "ExecuteTrade: no DB pool — sending Cancelled to both"
            );
            cimmeria_observability::counter!(
                "trade_swaps_total",
                "outcome" => "no_db_pool",
            );
            send_results_to_both(
                transport,
                connected,
                entity_to_addr,
                entity_id,
                partner_entity_id,
                ETRADERESULTS_CANCELLED,
                ETRADERESULTS_CANCELLED,
            )
            .await;
            return;
        }
    };

    // Refuse negative cash trivially (the cell already deduped via the
    // version-check + proposal-update path, but the wire is the wire).
    if p1_cash < 0 || p2_cash < 0 {
        tracing::warn!(
            p1_cash,
            p2_cash,
            "ExecuteTrade: negative cash in proposal — rejecting"
        );
        cimmeria_observability::counter!(
            "trade_swaps_total",
            "outcome" => "negative_cash",
        );
        send_results_to_both(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            partner_entity_id,
            ETRADERESULTS_CANCELLED,
            ETRADERESULTS_CANCELLED,
        )
        .await;
        return;
    }

    let p1 = TradeSide {
        entity_id,
        player_id,
        item_instance_ids: p1_item_instance_ids,
        cash: p1_cash,
    };
    let p2 = TradeSide {
        entity_id: partner_entity_id,
        player_id: partner_player_id,
        item_instance_ids: p2_item_instance_ids,
        cash: p2_cash,
    };
    let p1_account = account_of(connected, entity_to_addr, p1.entity_id);
    let p2_account = account_of(connected, entity_to_addr, p2.entity_id);

    match atomic_swap(&pool, &p1, &p2).await {
        Ok(committed) => {
            for m in &committed.moves {
                let (account_id, target_account_id) = if m.from_player == p1.player_id {
                    (p1_account, p2_account)
                } else {
                    (p2_account, p1_account)
                };
                log_item_moved(m, account_id, target_account_id);
            }
            tracing::info!(
                target: "trade.atomic_swap",
                event = "trade.completed",
                entity_id = p1.entity_id,
                player_id = p1.player_id,
                account_id = p1_account,
                target_entity_id = p2.entity_id,
                target_player_id = p2.player_id,
                target_account_id = p2_account,
                p1_items = p1.item_instance_ids.len(),
                p2_items = p2.item_instance_ids.len(),
                p1_cash = p1.cash,
                p2_cash = p2.cash,
                p1_naquadah_before = committed.p1_before,
                p1_naquadah_after = committed.balances.p1,
                p2_naquadah_before = committed.p2_before,
                p2_naquadah_after = committed.balances.p2,
                "trade executed atomically"
            );
            // After-commit notifications: cash + inventory + final
            // onTradeResults(Completed) to both clients.
            //
            // Cash totals come from inside the tx (computed from the
            // balances read under FOR UPDATE, before commit). Reading
            // post-commit with a separate query would open a small race
            // window: an unrelated transaction modifying naquadah
            // between our commit and the read would broadcast a wrong
            // total to the client.
            send_cash_changed_to_client(
                p1.entity_id,
                committed.balances.p1,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            send_cash_changed_to_client(
                p2.entity_id,
                committed.balances.p2,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            // Also refreshes each player's crafting options when a Field
            // Crafting Tool entered or left their crafting bag.
            send_full_inventory_update(
                p1.entity_id,
                p1.player_id,
                &pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            send_full_inventory_update(
                p2.entity_id,
                p2.player_id,
                &pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            send_results_to_both(
                transport,
                connected,
                entity_to_addr,
                p1.entity_id,
                p2.entity_id,
                ETRADERESULTS_COMPLETED,
                ETRADERESULTS_COMPLETED,
            )
            .await;
            cimmeria_observability::counter!(
                "trade_swaps_total",
                "outcome" => "completed",
            );
        }
        Err(reason) => {
            // Per the instrumentation-discipline ADR (rule 4): the
            // metric label vocab is enumerated low-cardinality, NOT
            // player_id / entity_id.
            let label = trade_abort_outcome_label(&reason);
            cimmeria_observability::counter!(
                "trade_swaps_total",
                "outcome" => label,
            );
            // Per-side ETradeResults codes: the failing player sees
            // `NoLocal*`, the other `NoRemote*`; catch-all variants map
            // to Cancelled on both sides.
            let (p1_code, p2_code) = trade_abort_to_results_codes(&reason, p1.player_id);
            tracing::warn!(
                target: "trade.atomic_swap",
                event = "trade.refused",
                entity_id = p1.entity_id,
                player_id = p1.player_id,
                account_id = p1_account,
                target_entity_id = p2.entity_id,
                target_player_id = p2.player_id,
                target_account_id = p2_account,
                reason = label,
                container_id = refusal_container(&reason),
                detail = %reason,
                p1_code,
                p2_code,
                "ExecuteTrade: atomic swap failed — sending asymmetric results"
            );
            send_results_to_both(
                transport,
                connected,
                entity_to_addr,
                p1.entity_id,
                p2.entity_id,
                p1_code,
                p2_code,
            )
            .await;
            let (p1_line, p2_line) = refusal_lines(&reason, p1.player_id);
            let ctx = FeedbackCtx {
                transport,
                connected,
            };
            for (side, line) in [(&p1, p1_line), (&p2, p2_line)] {
                if let Some(text) = line {
                    send_refusal_line(&ctx, entity_to_addr, side, label, text).await;
                }
            }
        }
    }
}

/// Successful swap: both sides' final balances, computed inside the
/// transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TradeFinalBalances {
    pub(super) p1: i32,
    pub(super) p2: i32,
}

/// What a committed swap did: the balances before and after, and every
/// item move.
#[derive(Debug)]
pub(super) struct TradeCommitted {
    pub(super) balances: TradeFinalBalances,
    pub(super) p1_before: i32,
    pub(super) p2_before: i32,
    pub(super) moves: Vec<ItemMove>,
}

/// One INFO per item that changed hands, with the bag and slot on both
/// ends.
fn log_item_moved(m: &ItemMove, account_id: Option<u32>, target_account_id: Option<u32>) {
    tracing::info!(
        target: "trade.atomic_swap",
        event = "trade.item_moved",
        entity_id = m.from_entity,
        player_id = m.from_player,
        account_id,
        target_entity_id = m.to_entity,
        target_player_id = m.to_player,
        target_account_id,
        item_id = m.item_id,
        type_id = m.type_id,
        container_before = m.from_container,
        slot_before = m.from_slot,
        container_after = m.to_container,
        slot_after = m.to_slot,
        "trade: item changed hands"
    );
}

/// The account behind `entity_id`'s session, if it is still connected.
fn account_of(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    entity_id: u32,
) -> Option<u32> {
    let addr = *entity_to_addr.lock().ok()?.get(&entity_id)?;
    let account = connected.lock().ok()?.get(&addr).map(|c| c.account_id);
    account
}

/// Send one side its refusal line. A miss is logged by
/// `send_feedback_line` itself, except an unmapped entity, logged here.
async fn send_refusal_line(
    ctx: &FeedbackCtx<'_>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    side: &TradeSide,
    reason: &'static str,
    text: &str,
) {
    let addr = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&side.entity_id).copied());
    let Some(addr) = addr else {
        tracing::warn!(
            target: "trade.atomic_swap",
            event = "trade.feedback_send_failed",
            entity_id = side.entity_id,
            player_id = side.player_id,
            reason = "entity_to_addr_miss",
            refusal = reason,
            "trade refusal line not sent: no address for the entity"
        );
        return;
    };
    send_feedback_line(ctx, addr, text).await;
}

// ── Outbound onTradeResults ───────────────────────────────────────────────

async fn send_results_to_both(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    p1_entity: u32,
    p2_entity: u32,
    p1_result: i32,
    p2_result: i32,
) {
    send_on_trade_results(
        transport,
        connected,
        entity_to_addr,
        p1_entity,
        p2_entity as i32,
        p1_result,
    )
    .await;
    send_on_trade_results(
        transport,
        connected,
        entity_to_addr,
        p2_entity,
        p1_entity as i32,
        p2_result,
    )
    .await;
}

async fn send_on_trade_results(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    entity_id: u32,
    partner_entity_id: i32,
    result: i32,
) {
    let args = serialize_on_trade_results(partner_entity_id, result);
    helpers::send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_TRADE_RESULTS,
                &args,
                version,
            )
        },
    )
    .await;
}
