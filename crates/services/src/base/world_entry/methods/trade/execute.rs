//! Atomic player-to-player trade execution.
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

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::{INV_BUYBACK, INV_MAIN};
use cimmeria_entity::trade::{
    serialize_on_trade_results, ETRADERESULTS_CANCELLED, ETRADERESULTS_COMPLETED,
};
use cimmeria_mercury::transport::Transport;
use sqlx::{PgPool, Postgres, Transaction};

use super::super::super::super::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::super::vendor::helpers::send_cash_changed_to_client;
use super::super::vendor::serializers::reserve_free_inventory_slots;
use crate::base::helpers;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// One side of a trade — the data the atomic commit needs to swap items
/// + cash from `from_player` to `to_player`.
struct TradeSide {
    /// Entity id of the player on this side (used for the wire packets).
    entity_id: u32,
    /// `sgw_player.player_id` for this side.
    player_id: i32,
    /// Inventory item instance ids this side is offering.
    item_instance_ids: Vec<i32>,
    /// Cash this side is offering.
    cash: i32,
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
        Ok(()) => {
            // After-commit notifications: cash + inventory + final
            // onTradeResults(Completed) to both clients.
            send_cash_changed_to_client(
                p1.entity_id,
                read_cash(&pool, p1.player_id).await,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            send_cash_changed_to_client(
                p2.entity_id,
                read_cash(&pool, p2.player_id).await,
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
            // The Python source asymmetrically reports NoLocal*/NoRemote*
            // by which side failed. Without breaking the tx return up
            // into per-side error variants, we send the same result code
            // to both. Cancelled is the safest catch-all — the client UI
            // treats it as a clean teardown notification.
            //
            // The result is still per-side so a future enhancement can
            // surface asymmetric codes without changing the wire path.
            tracing::warn!(
                p1_player = p1.player_id,
                p2_player = p2.player_id,
                reason = %reason,
                "ExecuteTrade: atomic swap failed — sending Cancelled"
            );
            send_results_to_both(
                transport,
                connected,
                entity_to_addr,
                p1.entity_id,
                p2.entity_id,
                ETRADERESULTS_CANCELLED,
                ETRADERESULTS_CANCELLED,
            )
            .await;
        }
    }
}

/// Reason the atomic swap aborted. The wire-side code (Cancelled vs
/// NoLocalCash / NoLocalSpace) is hardcoded to Cancelled for now;
/// future work can map these variants to asymmetric ETradeResults
/// codes per the deep dive Python comparison.
#[derive(Debug)]
enum TradeAbort {
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
    BuybackOffered {
        player_id: i32,
        item_id: i32,
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
            TradeAbort::BuybackOffered { player_id, item_id } => write!(
                f,
                "player {player_id} offered item {item_id} from the buyback bag"
            ),
        }
    }
}

impl From<sqlx::Error> for TradeAbort {
    fn from(e: sqlx::Error) -> Self {
        TradeAbort::DbError(e)
    }
}

#[derive(sqlx::FromRow)]
struct TradeItemRow {
    item_id: i32,
    container_id: i32,
    bound: bool,
}

async fn atomic_swap(pool: &Arc<PgPool>, p1: &TradeSide, p2: &TradeSide) -> Result<(), TradeAbort> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

    // Lock both players' rows in a deterministic order to avoid
    // deadlocks against any other paths that lock multiple players at
    // once. Ascending player_id is the convention.
    let (lo, hi) = if p1.player_id <= p2.player_id {
        (p1, p2)
    } else {
        (p2, p1)
    };
    take_advisory_lock(&mut tx, lo.player_id).await?;
    take_advisory_lock(&mut tx, hi.player_id).await?;

    // Read + lock both naquadah balances. SELECT FOR UPDATE serializes
    // against vendor purchase / sell / loot / mission grant paths.
    let p1_balance = read_naquadah_for_update(&mut tx, p1.player_id, "p1").await?;
    let p2_balance = read_naquadah_for_update(&mut tx, p2.player_id, "p2").await?;

    if p1_balance < p1.cash {
        let _ = tx.rollback().await;
        return Err(TradeAbort::InsufficientCash {
            which: "p1",
            player_id: p1.player_id,
            has: p1_balance,
            wants: p1.cash,
        });
    }
    if p2_balance < p2.cash {
        let _ = tx.rollback().await;
        return Err(TradeAbort::InsufficientCash {
            which: "p2",
            player_id: p2.player_id,
            has: p2_balance,
            wants: p2.cash,
        });
    }

    // Validate + lock items from each side. We pull the full row so we
    // can also check `bound` (bound items must not change hands) and
    // detect items the player claimed but doesn't actually own (or that
    // are sitting in the buyback bag, which would re-introduce items
    // the player previously sold to a vendor — forbidden).
    let p1_items = lock_items(&mut tx, p1.player_id, &p1.item_instance_ids, "p1").await?;
    let p2_items = lock_items(&mut tx, p2.player_id, &p2.item_instance_ids, "p2").await?;

    // Slot reservation: recipient must have room in INV_MAIN for the
    // items they're about to receive. Slots are taken from MIN→MAX so a
    // recipient with a full main bag fails fast.
    //
    // The recipient of p1's items is p2, and vice versa.
    let p2_new_slots = reserve_main_slots_for(&mut tx, p2.player_id, p1_items.len()).await?;
    let p1_new_slots = reserve_main_slots_for(&mut tx, p1.player_id, p2_items.len()).await?;

    // Apply the item moves: re-key each row to the recipient, drop into
    // INV_MAIN at the reserved slot.
    for (row, &new_slot) in p1_items.iter().zip(p2_new_slots.iter()) {
        move_item_to_recipient(&mut tx, row.item_id, p2.player_id, new_slot).await?;
    }
    for (row, &new_slot) in p2_items.iter().zip(p1_new_slots.iter()) {
        move_item_to_recipient(&mut tx, row.item_id, p1.player_id, new_slot).await?;
    }

    // Cash debits & credits. Net delta per side avoids a redundant
    // SQL roundtrip when both sides offered the same amount (no-op).
    if p1.cash != p2.cash || p1.cash != 0 {
        let p1_delta = p2.cash - p1.cash; // p1 receives p2.cash, owes p1.cash
        let p2_delta = p1.cash - p2.cash;
        if p1_delta != 0 {
            sqlx::query("UPDATE sgw_player SET naquadah = naquadah + $1 WHERE player_id = $2")
                .bind(p1_delta)
                .bind(p1.player_id)
                .execute(&mut *tx)
                .await?;
        }
        if p2_delta != 0 {
            sqlx::query("UPDATE sgw_player SET naquadah = naquadah + $1 WHERE player_id = $2")
                .bind(p2_delta)
                .bind(p2.player_id)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

async fn take_advisory_lock(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1, 0)")
        .bind(player_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn read_naquadah_for_update(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    which: &'static str,
) -> Result<i32, TradeAbort> {
    let row: Option<i32> =
        sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1 FOR UPDATE")
            .bind(player_id)
            .fetch_optional(&mut **tx)
            .await?;
    row.ok_or(TradeAbort::PlayerMissing { which, player_id })
}

async fn read_cash(pool: &Arc<PgPool>, player_id: i32) -> i32 {
    sqlx::query_scalar::<_, i32>("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_optional(pool.as_ref())
        .await
        .ok()
        .flatten()
        .unwrap_or(0)
}

/// SELECT FOR UPDATE every item instance from the player's inventory,
/// returning rows in input order. Fails with `ItemMissing` /
/// `BoundItemOffered` / `DuplicateInstance` / `BuybackOffered` if the
/// validation gauntlet rejects any entry.
async fn lock_items(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    instance_ids: &[i32],
    which: &'static str,
) -> Result<Vec<TradeItemRow>, TradeAbort> {
    // De-duplicate check first — same instance listed twice is
    // structurally invalid and would corrupt the ownership transfer.
    {
        let mut seen = std::collections::HashSet::with_capacity(instance_ids.len());
        for &id in instance_ids {
            if !seen.insert(id) {
                return Err(TradeAbort::DuplicateInstance { item_id: id });
            }
        }
    }

    let mut rows = Vec::with_capacity(instance_ids.len());
    for &item_id in instance_ids {
        let row: Option<TradeItemRow> = sqlx::query_as::<_, TradeItemRow>(
            "SELECT item_id, container_id, bound FROM sgw_inventory \
             WHERE character_id = $1 AND item_id = $2 FOR UPDATE",
        )
        .bind(player_id)
        .bind(item_id)
        .fetch_optional(&mut **tx)
        .await?;
        let row = row.ok_or(TradeAbort::ItemMissing {
            which,
            player_id,
            item_id,
        })?;
        if row.bound {
            return Err(TradeAbort::BoundItemOffered {
                which,
                player_id,
                item_id,
            });
        }
        if row.container_id == INV_BUYBACK {
            return Err(TradeAbort::BuybackOffered { player_id, item_id });
        }
        rows.push(row);
    }
    Ok(rows)
}

async fn reserve_main_slots_for(
    tx: &mut Transaction<'_, Postgres>,
    recipient_player_id: i32,
    needed: usize,
) -> Result<Vec<i32>, TradeAbort> {
    if needed == 0 {
        return Ok(Vec::new());
    }
    match reserve_free_inventory_slots(tx, recipient_player_id, INV_MAIN, needed).await? {
        Some(slots) => Ok(slots),
        None => Err(TradeAbort::NotEnoughSlots {
            recipient_player_id,
            needed,
        }),
    }
}

async fn move_item_to_recipient(
    tx: &mut Transaction<'_, Postgres>,
    item_id: i32,
    recipient_player_id: i32,
    new_slot_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sgw_inventory \
         SET character_id = $1, container_id = $2, slot_id = $3 \
         WHERE item_id = $4",
    )
    .bind(recipient_player_id)
    .bind(INV_MAIN)
    .bind(new_slot_id)
    .bind(item_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
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
