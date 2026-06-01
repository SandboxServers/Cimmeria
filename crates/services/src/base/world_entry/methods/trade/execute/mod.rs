//! Atomic player-to-player trade execution — entry point + types.
//!
//! Both participants have already reached `ETRADELOCKSTATE_LockedAndConfirmed`
//! on the cell side; the cell has cleared its in-memory state and handed
//! the snapshot off as `CellToBaseMsg::ExecuteTrade`. This module owns
//! the final commit:
//!
//! 1. `BEGIN` a single sqlx transaction.
//! 2. `FOR UPDATE` lock both player rows (`sgw_player`) — naquadah is here.
//! 3. `FOR UPDATE` lock every item row each player is offering (read
//!    type_id + stack_size + container_id + slot_id + bound/durability/etc.).
//! 4. Re-validate cash balances and item ownership (TOCTOU window
//!    between cell snapshot and base commit).
//! 5. Reserve `needed` free slots in INV_MAIN for each recipient.
//! 6. UPDATE `character_id` on each item row to the recipient + bump
//!    container/slot to the reserved destination.
//! 7. Debit / credit `sgw_player.naquadah`.
//! 8. COMMIT.
//! 9. Push `onCashChanged` + `onUpdateItem` (full inventory) + final
//!    `onTradeResults` to both clients.
//!
//! On any failure between (1) and (8): rollback and send
//! `onTradeResults(Cancelled)` to both clients. **No items are lost** —
//! the rollback undoes every UPDATE and the cash debit. The cell-side
//! state was already cleared by the time we got here, so the players'
//! UIs just see a "trade cancelled" notification and they can start over.
//!
//! ## Module layout
//!
//! - [`swap`] — the atomic-swap transaction internals (advisory lock,
//!   item locking, slot reservation, two-phase parked-row item move,
//!   cash debit/credit). Keeps `atomic_swap` and its helpers in one
//!   place so the FOR-UPDATE → mutate → commit pipeline can be read
//!   top to bottom.
//! - [`tests`] (cfg-only) — the 4 unit test modules:
//!   advisory-lock alignment, slot-exclusion accounting, parking
//!   sentinel distinctness, and the asymmetric ETradeResults code
//!   mapping. Live-DB integration tests live in `super::tests`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::trade::{
    serialize_on_trade_results, ETRADERESULTS_CANCELLED, ETRADERESULTS_COMPLETED,
    ETRADERESULTS_NO_LOCAL_CASH, ETRADERESULTS_NO_LOCAL_SPACE, ETRADERESULTS_NO_REMOTE_CASH,
    ETRADERESULTS_NO_REMOTE_SPACE,
};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::super::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::super::vendor::helpers::send_cash_changed_to_client;
use crate::base::helpers;
use crate::mercury::{build_player_entity_method_packet, method_idx};

mod swap;

#[cfg(test)]
mod tests;

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
#[allow(clippy::too_many_arguments)]
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

    match atomic_swap(&pool, &p1, &p2).await {
        Ok(final_balances) => {
            // After-commit notifications: cash + inventory + final
            // onTradeResults(Completed) to both clients.
            //
            // Cash totals come from inside the tx (read AFTER the debit/
            // credit UPDATE, before commit). Reading post-commit with a
            // separate query would open a small race window: an
            // unrelated transaction modifying naquadah between our
            // commit and the read would broadcast a wrong total to the
            // client, making the UI desync from `sgw_player.naquadah`.
            send_cash_changed_to_client(
                p1.entity_id,
                final_balances.p1,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            send_cash_changed_to_client(
                p2.entity_id,
                final_balances.p2,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
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
            tracing::info!(
                p1_player = p1.player_id,
                p2_player = p2.player_id,
                "trade executed atomically"
            );
        }
        Err(reason) => {
            // Map the abort variant to the Python-parity per-side
            // ETradeResults codes. The wire shape is unchanged — each
            // client always received an INT32 result on `onTradeResults` —
            // but now the result is per-side asymmetric: the failing
            // player sees `NoLocal*`, the other sees `NoRemote*`. The
            // canonical client uses these to surface a more specific
            // "you don't have enough cash" / "they don't have enough
            // space" string in the trade-results dialog.
            //
            // Catch-all variants (DbError, PlayerMissing, DuplicateInstance,
            // BoundItemOffered, IneligibleContainer) map to Cancelled on
            // both sides — these are either internal faults or
            // server-authority validations the client UI has no
            // dedicated string for.
            let (p1_code, p2_code) =
                trade_abort_to_results_codes(&reason, p1.player_id, p2.player_id);
            tracing::warn!(
                p1_player = p1.player_id,
                p2_player = p2.player_id,
                reason = %reason,
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
        }
    }
}

/// Final post-commit balances for both sides of a successful trade —
/// read inside the same transaction as the cash UPDATEs so the
/// `onCashChanged` packet can't race against a concurrent vendor /
/// loot / mission grant on either player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TradeFinalBalances {
    pub(super) p1: i32,
    pub(super) p2: i32,
}

/// Per-side ETradeResults code mapping for [`TradeAbort`].
///
/// Returns `(p1_code, p2_code)` in p1-then-p2 order. The asymmetric
/// `NoLocal*` / `NoRemote*` codes mirror Python `Trade.py:237-263`:
/// the failing player sees `NoLocal*`, the other sees `NoRemote*`. The
/// canonical client uses the asymmetry to surface a more specific
/// trade-results dialog string ("you don't have enough cash" vs.
/// "they don't have enough space"); both sides seeing Cancelled would
/// render the generic teardown string for both, hiding the cause.
///
/// `InsufficientCash` carries a `which: "p1"|"p2"` discriminant that
/// directly identifies the failing side. `NotEnoughSlots` carries
/// `recipient_player_id` — the side without room — which we resolve
/// against the caller-provided `p1_player_id`/`p2_player_id`.
///
/// Catch-all variants (`DbError`, `PlayerMissing`, `ItemMissing`,
/// `DuplicateInstance`, `BoundItemOffered`, `IneligibleContainer`) are
/// internal faults or server-authority validations the client UI has
/// no dedicated string for — both sides see Cancelled, matching the
/// pre-asymmetric behavior.
fn trade_abort_to_results_codes(
    reason: &TradeAbort,
    p1_player_id: i32,
    p2_player_id: i32,
) -> (i32, i32) {
    match reason {
        TradeAbort::InsufficientCash { which: "p1", .. } => {
            (ETRADERESULTS_NO_LOCAL_CASH, ETRADERESULTS_NO_REMOTE_CASH)
        }
        TradeAbort::InsufficientCash { which: "p2", .. } => {
            (ETRADERESULTS_NO_REMOTE_CASH, ETRADERESULTS_NO_LOCAL_CASH)
        }
        TradeAbort::NotEnoughSlots {
            recipient_player_id,
            ..
        } => {
            if *recipient_player_id == p1_player_id {
                (ETRADERESULTS_NO_LOCAL_SPACE, ETRADERESULTS_NO_REMOTE_SPACE)
            } else if *recipient_player_id == p2_player_id {
                (ETRADERESULTS_NO_REMOTE_SPACE, ETRADERESULTS_NO_LOCAL_SPACE)
            } else {
                // recipient is neither side — shouldn't happen unless
                // the abort variant was constructed with a stale id;
                // fail safe with Cancelled rather than guessing.
                (ETRADERESULTS_CANCELLED, ETRADERESULTS_CANCELLED)
            }
        }
        _ => (ETRADERESULTS_CANCELLED, ETRADERESULTS_CANCELLED),
    }
}

/// Reason the atomic swap aborted. Mapped to per-side asymmetric
/// `ETradeResults` codes via [`trade_abort_to_results_codes`]:
/// `InsufficientCash {p1|p2}` → `NoLocalCash`/`NoRemoteCash`,
/// `NotEnoughSlots {recipient_player_id}` →
/// `NoLocalSpace`/`NoRemoteSpace`, with the remaining catch-all
/// variants (DbError, PlayerMissing, ItemMissing, DuplicateInstance,
/// BoundItemOffered, IneligibleContainer) staying on the generic
/// Cancelled code — those are internal faults or server-authority
/// rejections the client UI has no dedicated string for.
#[derive(Debug)]
pub(super) enum TradeAbort {
    DbError(sqlx::Error),
    PlayerMissing {
        which: &'static str,
        player_id: i32,
    },
    InsufficientCash {
        which: &'static str,
        player_id: i32,
        has: i32,
        wants: i32,
    },
    ItemMissing {
        which: &'static str,
        player_id: i32,
        item_id: i32,
    },
    NotEnoughSlots {
        recipient_player_id: i32,
        needed: usize,
    },
    BoundItemOffered {
        which: &'static str,
        player_id: i32,
        item_id: i32,
    },
    DuplicateInstance {
        item_id: i32,
    },
    /// Item lives in a container that's not on the tradeable-container
    /// whitelist (anything other than `INV_MAIN`). Covers the
    /// dupe-strip-equipped-gear, mission-item-share, banker-gate-bypass,
    /// and bandolier-ammo-sync exploits — all the same shape: the
    /// server must independently verify *which* containers can leak
    /// items, not just whether the row is bound or in the buyback bag.
    IneligibleContainer {
        which: &'static str,
        player_id: i32,
        item_id: i32,
        container_id: i32,
    },
}

impl std::fmt::Display for TradeAbort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeAbort::DbError(e) => write!(f, "db error: {e}"),
            TradeAbort::PlayerMissing { which, player_id } => {
                write!(f, "{which} player {player_id} missing")
            }
            TradeAbort::InsufficientCash {
                which,
                player_id,
                has,
                wants,
            } => write!(
                f,
                "{which} player {player_id} has {has} naquadah, offering {wants}"
            ),
            TradeAbort::ItemMissing {
                which,
                player_id,
                item_id,
            } => write!(
                f,
                "{which} player {player_id} doesn't own item instance {item_id}"
            ),
            TradeAbort::NotEnoughSlots {
                recipient_player_id,
                needed,
            } => write!(
                f,
                "recipient {recipient_player_id} doesn't have {needed} free main-bag slots"
            ),
            TradeAbort::BoundItemOffered {
                which,
                player_id,
                item_id,
            } => write!(f, "{which} player {player_id} offered bound item {item_id}"),
            TradeAbort::DuplicateInstance { item_id } => {
                write!(f, "item instance {item_id} listed twice in proposal")
            }
            TradeAbort::IneligibleContainer {
                which,
                player_id,
                item_id,
                container_id,
            } => write!(
                f,
                "{which} player {player_id} offered item {item_id} from \
                 non-tradeable container {container_id} \
                 (whitelist: only INV_MAIN)"
            ),
        }
    }
}

impl From<sqlx::Error> for TradeAbort {
    fn from(e: sqlx::Error) -> Self {
        TradeAbort::DbError(e)
    }
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
        |key, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_TRADE_RESULTS,
                &args,
            )
        },
    )
    .await;
}
