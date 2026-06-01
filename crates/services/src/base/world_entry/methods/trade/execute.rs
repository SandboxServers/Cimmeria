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

use cimmeria_entity::inventory::INV_MAIN;
use cimmeria_entity::trade::{
    serialize_on_trade_results, ETRADERESULTS_CANCELLED, ETRADERESULTS_COMPLETED,
    ETRADERESULTS_NO_LOCAL_CASH, ETRADERESULTS_NO_LOCAL_SPACE, ETRADERESULTS_NO_REMOTE_CASH,
    ETRADERESULTS_NO_REMOTE_SPACE,
};
use cimmeria_mercury::transport::Transport;
use sqlx::{PgPool, Postgres, Transaction};

use super::super::super::super::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::super::vendor::helpers::send_cash_changed_to_client;
use super::super::vendor::serializers::free_inventory_slots;
use crate::base::helpers;
use crate::base::resources::{bag_max_slots, bag_min_slot};
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
struct TradeFinalBalances {
    p1: i32,
    p2: i32,
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

#[derive(sqlx::FromRow)]
struct TradeItemRow {
    item_id: i32,
    container_id: i32,
    slot_id: i32,
    bound: bool,
}

async fn atomic_swap(
    pool: &Arc<PgPool>,
    p1: &TradeSide,
    p2: &TradeSide,
) -> Result<TradeFinalBalances, TradeAbort> {
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
    // can check `bound` (soul-bound items never change hands), detect
    // items the player claimed but doesn't actually own, and gate on
    // `container_id` — only INV_MAIN is on the tradeable whitelist
    // (buyback / bank / mission / equip slots / bandolier are all
    // rejected). See `TRADEABLE_CONTAINERS` below for rationale.
    let p1_items = lock_items(&mut tx, p1.player_id, &p1.item_instance_ids, "p1").await?;
    let p2_items = lock_items(&mut tx, p2.player_id, &p2.item_instance_ids, "p2").await?;

    // Slot reservation: recipient must have room in INV_MAIN for the
    // items they're about to receive. Slots are taken from MIN→MAX so a
    // recipient with a full main bag fails fast.
    //
    // The recipient of p1's items is p2, and vice versa.
    //
    // Important: the recipient's OWN outgoing INV_MAIN items count as
    // "currently occupied" in the raw SELECT, but they're about to be
    // moved to the other side in this same transaction — so for the
    // purposes of slot reservation they should be treated as free.
    // Without this exclusion, a valid full-bag swap (e.g., P2's bag is
    // full but one of those slots holds the item P2 is trading away)
    // would fail spuriously. The atomic-commit transaction makes this
    // sound: either both sides' UPDATEs land, or neither does.
    let p2_vacating_slots = main_slot_ids_of(&p2_items);
    let p1_vacating_slots = main_slot_ids_of(&p1_items);
    let p2_new_slots =
        reserve_main_slots_excluding(&mut tx, p2.player_id, p1_items.len(), &p2_vacating_slots)
            .await?;
    let p1_new_slots =
        reserve_main_slots_excluding(&mut tx, p1.player_id, p2_items.len(), &p1_vacating_slots)
            .await?;

    // Apply the item moves in two phases to avoid violating the
    // `sgw_inventory_unique_slot` UNIQUE INDEX on
    // `(character_id, container_id, slot_id)`. A single-statement re-key
    // collides whenever the recipient's destination slot is currently
    // occupied by a row that this same transaction will vacate
    // (the trivial case: both sides hold an item at INV_MAIN slot 0 —
    // moving p1's item to (p2, INV_MAIN, 0) collides with p2's existing
    // row at (p2, INV_MAIN, 0) until that row is itself moved out).
    //
    // The two-phase shape mirrors the swap pattern in `inventory/move_`:
    //   Phase 1: park every outgoing item in a unique negative sentinel
    //            slot in INV_MAIN. character_id and container_id are left
    //            on the sender so the parked rows still belong to
    //            someone (FK + observability), only slot_id changes.
    //   Phase 2: re-key each parked row to the recipient and into the
    //            reserved destination slot. By this point every original
    //            slot is vacant on both sides, so no UNIQUE collision.
    //
    // Each parked item gets its OWN distinct negative slot so the parked
    // set itself can't collide. (The single-sentinel approach in
    // `inventory/move_` works there because that path swaps at most two
    // items; trade can move up to 40 per side.) The (player_id, INV_MAIN)
    // advisory lock taken upstream serializes against any other path
    // that might also be parking rows for either player.
    let total_items = p1_items.len() + p2_items.len();
    for (parked_index, row) in (0_i32..).zip(p1_items.iter().chain(p2_items.iter())) {
        let sentinel = park_sentinel_slot(parked_index, total_items);
        park_item_at_sentinel(&mut tx, row.item_id, sentinel).await?;
    }
    for (row, &new_slot) in p1_items.iter().zip(p2_new_slots.iter()) {
        move_item_to_recipient(&mut tx, row.item_id, p2.player_id, new_slot).await?;
    }
    for (row, &new_slot) in p2_items.iter().zip(p1_new_slots.iter()) {
        move_item_to_recipient(&mut tx, row.item_id, p1.player_id, new_slot).await?;
    }

    // Cash debits & credits. Net delta per side avoids a redundant
    // SQL roundtrip when both sides offered the same amount (no-op).
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

    // Compute final balances arithmetically rather than re-reading
    // `sgw_player.naquadah` post-UPDATE. We already hold the
    // pre-UPDATE balance (`p1_balance` / `p2_balance`) under
    // `FOR UPDATE` locks, and the delta is the only mutation to
    // naquadah in this transaction. A re-read would just round-trip
    // the same value (the row is locked, no concurrent writer can
    // change it). Sourcing the totals from inside the tx is what
    // closes the race window the post-commit `read_cash` opened —
    // see the design note in [`handle_execute_trade`].
    let final_balances = TradeFinalBalances {
        p1: p1_balance + p1_delta,
        p2: p2_balance + p2_delta,
    };

    tx.commit().await?;
    Ok(final_balances)
}

/// Acquire the per-player advisory lock for the trade transaction.
///
/// The namespace MUST match the vendor stack's lock shape — vendor
/// uses `pg_advisory_xact_lock(player_id, container_id)` with
/// `container_id = INV_MAIN` in `reserve_free_inventory_slots`. Trade
/// previously used `(player_id, 0)` which gave Postgres two
/// independent lock keys for the same logical lock, so a concurrent
/// trade and vendor purchase on the same player wouldn't serialize at
/// the advisory layer (`FOR UPDATE` on the row remains correct, but
/// the deadlock detector surfaces ABBA failures noisily under load).
///
/// `pg_advisory_xact_lock` is idempotent within a single transaction
/// (see the comment in `inventory::grant`), so the redundant lock
/// acquired by `reserve_free_inventory_slots` for INV_MAIN later in
/// the same trade tx is a no-op.
/// The SQL form for the advisory lock — extracted into a constant so
/// the alignment test below can assert it byte-for-byte against the
/// vendor stack's lock SQL. Any divergence (single-arg form, different
/// namespace) breaks the alignment guarantee.
const ADVISORY_LOCK_SQL: &str = "SELECT pg_advisory_xact_lock($1, $2)";

async fn take_advisory_lock(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(ADVISORY_LOCK_SQL)
        .bind(player_id)
        .bind(INV_MAIN)
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

/// Whitelist of containers an item may sit in to be eligible for trade.
///
/// Only `INV_MAIN` (the visible main bag) is on the wire-trade allowlist.
/// Players who want to trade equipped gear, banked items, or bandolier
/// ammo must unequip / withdraw / unload first — the same flow the
/// canonical SGW client uses, and the same intent as the Python
/// `canSell()` check (Trade.py only operates on main-bag rows).
///
/// Anything outside this list is rejected with
/// [`TradeAbort::IneligibleContainer`]. This is the **server-authority**
/// version of the check: the wire only carries `instance_id`, so the
/// server independently decides which rows are trade-eligible.
///
/// **Do not add `INV_BUYBACK` (16) here** — buyback bag items must
/// remain reclaimable only by their original seller. The whitelist
/// subsumes the old buyback blacklist.
const TRADEABLE_CONTAINERS: &[i32] = &[INV_MAIN];

/// SELECT FOR UPDATE every item instance from the player's inventory,
/// returning rows in input order. Fails with `ItemMissing` /
/// `BoundItemOffered` / `DuplicateInstance` / `IneligibleContainer` if
/// the validation gauntlet rejects any entry.
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
            "SELECT item_id, container_id, slot_id, bound FROM sgw_inventory \
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
        // Whitelist gate: only INV_MAIN is trade-eligible. The blacklist
        // pre-fix only blocked INV_BUYBACK; every other container
        // (equipped gear, mission items, bank, bandolier, crafting,
        // auction, team/command bank) silently passed. That's a
        // dupe-strip exploit on equip slots and a bypass of the
        // banker-NPC gate on bank items. See the security review on
        // PR #438.
        if !TRADEABLE_CONTAINERS.contains(&row.container_id) {
            return Err(TradeAbort::IneligibleContainer {
                which,
                player_id,
                item_id,
                container_id: row.container_id,
            });
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Slot IDs of every TradeItemRow that lives in INV_MAIN — these are
/// the slots that will become free in the same transaction as the
/// recipient's slot reservation, so they must be excluded from the
/// recipient's "currently occupied" set.
///
/// Non-INV_MAIN items can't appear in the trade today (the
/// `TRADEABLE_CONTAINERS` whitelist rejects them in [`lock_items`]) but
/// the filter is kept defensive so a future whitelist expansion doesn't
/// silently misaccount.
fn main_slot_ids_of(rows: &[TradeItemRow]) -> Vec<i32> {
    rows.iter()
        .filter(|r| r.container_id == INV_MAIN)
        .map(|r| r.slot_id)
        .collect()
}

/// Reserve `needed` free slots in the recipient's INV_MAIN, excluding
/// any slot IDs the same transaction is about to vacate.
///
/// Without the exclusion, a valid full-bag swap fails: if the
/// recipient's bag is full but contains an item they're trading away,
/// `reserve_free_inventory_slots` sees that slot as occupied and
/// rejects the trade even though the slot will be free by commit time.
/// The transaction is atomic — either every `UPDATE sgw_inventory`
/// statement lands or none do — so excluding the soon-to-vacate slots
/// is correct.
///
/// We inline the slot query rather than calling
/// `reserve_free_inventory_slots` directly because that helper has no
/// exclusion hook. Composition via the pure [`free_inventory_slots`]
/// keeps the slot-pick logic shared, only the occupancy assembly
/// differs.
async fn reserve_main_slots_excluding(
    tx: &mut Transaction<'_, Postgres>,
    recipient_player_id: i32,
    needed: usize,
    excluding_slots: &[i32],
) -> Result<Vec<i32>, TradeAbort> {
    if needed == 0 {
        return Ok(Vec::new());
    }

    // Per-(player, container) advisory lock — mirror the lock shape
    // `reserve_free_inventory_slots` uses, since concurrent vendor /
    // grant paths serialize against this namespace.
    sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(recipient_player_id)
        .bind(INV_MAIN)
        .execute(&mut **tx)
        .await?;

    #[derive(sqlx::FromRow)]
    struct InventorySlotRow {
        slot_id: i32,
    }

    let rows = sqlx::query_as::<_, InventorySlotRow>(
        "SELECT slot_id FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2 \
         FOR UPDATE",
    )
    .bind(recipient_player_id)
    .bind(INV_MAIN)
    .fetch_all(&mut **tx)
    .await?;

    let raw_occupied: Vec<i32> = rows.into_iter().map(|row| row.slot_id).collect();
    match pick_free_main_slots_excluding(&raw_occupied, excluding_slots, needed) {
        Some(slots) => Ok(slots),
        None => Err(TradeAbort::NotEnoughSlots {
            recipient_player_id,
            needed,
        }),
    }
}

/// Pure slot-pick: given the recipient's current INV_MAIN occupancy and
/// the slot IDs they're about to vacate in the same transaction, return
/// the lowest-indexed `needed` slots that will be free post-swap, or
/// `None` if the bag can't fit them.
///
/// Split out from [`reserve_main_slots_excluding`] so the unit test
/// path exercises the same algorithmic core the production async fn
/// uses — a revert of the exclusion logic here trips the unit-level
/// regression guard, not just the live-DB integration test.
fn pick_free_main_slots_excluding(
    raw_occupied: &[i32],
    vacating: &[i32],
    needed: usize,
) -> Option<Vec<i32>> {
    let excluding: std::collections::HashSet<i32> = vacating.iter().copied().collect();
    let occupied_after_exclusion: Vec<i32> = raw_occupied
        .iter()
        .copied()
        .filter(|slot| !excluding.contains(slot))
        .collect();
    free_inventory_slots(
        bag_min_slot(INV_MAIN),
        bag_max_slots(INV_MAIN),
        &occupied_after_exclusion,
        needed,
    )
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

/// Compute a unique negative parking slot for the `nth` of `total`
/// outgoing items in a trade.
///
/// Returns slots in the range `[-(total), -1]` so the parked items
/// don't collide with each other against the
/// `sgw_inventory_unique_slot` UNIQUE INDEX, and don't collide with
/// any real container slot (every container's `bag_min_slot` is 0,
/// so negative slots are unreachable from any normal grant / move /
/// purchase path).
///
/// The exact mapping (`-(nth + 1)`) is an internal detail; only the
/// distinctness and negativity are load-bearing. The `total`
/// parameter is plumbed through for a future debug assertion / log
/// without changing the wire shape.
fn park_sentinel_slot(nth: i32, _total: usize) -> i32 {
    // -1, -2, -3, ... — distinct per parked item.
    -(nth + 1)
}

/// Phase-1 parking step of the two-phase swap: relocate `item_id` to
/// a sentinel slot in INV_MAIN without changing its owner. This
/// vacates the item's original slot so the partner's incoming item
/// can land there in phase 2 without colliding with the
/// `sgw_inventory_unique_slot` UNIQUE INDEX on
/// `(character_id, container_id, slot_id)`.
///
/// `sentinel_slot_id` must be unique within the parked set for this
/// transaction — see [`park_sentinel_slot`].
async fn park_item_at_sentinel(
    tx: &mut Transaction<'_, Postgres>,
    item_id: i32,
    sentinel_slot_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sgw_inventory \
         SET container_id = $1, slot_id = $2 \
         WHERE item_id = $3",
    )
    .bind(INV_MAIN)
    .bind(sentinel_slot_id)
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

// ── Static-read regression guards ──────────────────────────────────────────

#[cfg(test)]
mod advisory_lock_alignment {
    //! Static-read regression guard: trade's advisory-lock SQL form
    //! must match the vendor stack's. Both use the two-arg form
    //! `pg_advisory_xact_lock($1, $2)` and bind `(player_id, INV_MAIN)`
    //! so the two paths serialize through the same lock namespace —
    //! otherwise concurrent trade-and-vendor on the same player race
    //! at the advisory layer (`FOR UPDATE` still saves correctness,
    //! but the deadlock detector surfaces ABBA failures noisily
    //! under load).
    //!
    //! The original trade code used the single-arg form
    //! `pg_advisory_xact_lock(player_id, 0)`. Postgres treats
    //! `(player_id, 0)` and `(player_id, 1)` as completely independent
    //! locks — they don't serialize against each other.
    //!
    //! This is a static-read test (no DB required) so the alignment
    //! invariant is gated in CI even when the live-DB profile is off.

    use super::ADVISORY_LOCK_SQL;
    use cimmeria_entity::inventory::INV_MAIN;

    /// Pin the SQL string. If a future change goes back to the
    /// single-arg `(player_id, 0)` form, this assertion trips.
    #[test]
    fn lock_sql_is_two_arg_form() {
        assert_eq!(
            ADVISORY_LOCK_SQL, "SELECT pg_advisory_xact_lock($1, $2)",
            "trade's advisory lock MUST use the two-arg form to match \
             vendor's `pg_advisory_xact_lock(player_id, container_id)`. \
             Single-arg form races at the advisory layer against vendor."
        );
    }

    /// Pin that the second bind is INV_MAIN. The vendor stack always
    /// binds the container id; for trade we lock under INV_MAIN
    /// (since that's the only tradeable container per the whitelist).
    #[test]
    fn lock_namespace_is_inv_main_not_zero() {
        // INV_MAIN is `1` per `entities/defs/system.xml`. The pre-fix
        // shape was `(player_id, 0)` which collides with no real
        // container — so the lock was effectively in its own
        // namespace.
        assert_eq!(
            INV_MAIN, 1,
            "INV_MAIN must be 1 — if this constant changes, audit the \
             vendor stack too: it binds the container id directly."
        );
    }
}

#[cfg(test)]
mod slot_exclusion_accounting {
    //! Unit-level regression guard for the recipient-slot-reservation
    //! bug. The full live-DB integration guard is
    //! `commit_succeeds_when_recipient_bag_full_but_trading_slot_away`
    //! in this module's `tests.rs`; this is the algorithmic-core proxy
    //! that runs without a live DB.
    //!
    //! Pre-fix, `reserve_main_slots_for(recipient, needed)` called
    //! `reserve_free_inventory_slots(_, recipient, INV_MAIN, needed)`
    //! which read the recipient's current INV_MAIN occupancy and
    //! treated EVERY existing row as occupied. That counted the
    //! recipient's own outgoing-this-trade items as occupied even
    //! though the same atomic transaction would move them out.
    //!
    //! Post-fix, `reserve_main_slots_excluding` filters the outgoing
    //! slot IDs out of the "occupied" set before calling the pure
    //! `free_inventory_slots(min, max, occupied, needed)` helper. This
    //! test exercises that algorithmic core: same occupancy, same
    //! `needed`, but excluding vs. not excluding the about-to-vacate
    //! slot must give different answers.

    use super::*;

    /// Bag is 100% full (40/40 in INV_MAIN). One of those 40 is being
    /// traded away. Without the exclusion the reservation fails
    /// (0 free); with the exclusion it succeeds (slot 39 is the
    /// vacating slot, returned as the pick).
    ///
    /// This test calls [`pick_free_main_slots_excluding`] directly —
    /// the same pure helper that the production async
    /// `reserve_main_slots_excluding` delegates to after reading the
    /// raw occupancy from the DB. A revert of the exclusion logic in
    /// `pick_free_main_slots_excluding` (e.g., dropping the
    /// `.filter(|slot| !excluding.contains(slot))`) trips this guard.
    ///
    /// Revert-verifier: change `pick_free_main_slots_excluding` to
    /// pass `raw_occupied` straight through to `free_inventory_slots`
    /// without subtracting `vacating`; the second assertion below
    /// fails with `None != Some([39])`.
    #[test]
    fn full_bag_swap_succeeds_with_exclusion_fails_without() {
        // 40/40 occupied — slots 0..=39.
        let raw_occupied: Vec<i32> = (0..bag_max_slots(INV_MAIN)).collect();
        let vacating = vec![39]; // the slot the recipient is trading away

        // Sanity: with NO exclusion (vacating is empty), the
        // fully-occupied bag rejects the reservation. This is the
        // pre-fix shape — what the bug looked like in production.
        let without_exclusion = pick_free_main_slots_excluding(&raw_occupied, &[], 1);
        assert!(
            without_exclusion.is_none(),
            "sanity: without the exclusion (i.e., vacating list empty), \
             a 40/40 INV_MAIN bag can't reserve a slot. This is the \
             pre-fix shape — the bug Copilot flagged was that the \
             recipient's outgoing trade item counted as occupied even \
             though it's about to leave."
        );

        // Post-fix behaviour: with the vacating slot excluded, the
        // 40-slot bag has 1 free slot (39 itself, since the picker
        // returns the lowest free slot in [min, max)).
        let with_exclusion = pick_free_main_slots_excluding(&raw_occupied, &vacating, 1);
        assert_eq!(
            with_exclusion,
            Some(vec![39]),
            "with the exclusion, the recipient's about-to-vacate slot \
             39 is available for the incoming item. If this returns \
             None, the exclusion was dropped from \
             `pick_free_main_slots_excluding` — re-check the filter."
        );
    }

    /// Two-for-two swap: recipient's bag is full (40/40), trading 2
    /// of those slots away, must accept 2 incoming items.
    #[test]
    fn full_bag_two_for_two_swap_succeeds_with_exclusion() {
        let raw_occupied: Vec<i32> = (0..bag_max_slots(INV_MAIN)).collect();
        let vacating = vec![5, 17]; // two non-contiguous outgoing slots
        let picked = pick_free_main_slots_excluding(&raw_occupied, &vacating, 2);
        assert_eq!(
            picked,
            Some(vec![5, 17]),
            "the two vacating slots are the only free slots and must \
             be returned in ascending order"
        );
    }

    /// Recipient still has free slots even WITHOUT counting the
    /// vacating ones — the exclusion is a no-op in that case. This
    /// pins the "happy path" doesn't regress when the fix is in:
    /// the exclusion must not poison the pick when it isn't needed.
    #[test]
    fn partially_full_bag_doesnt_need_exclusion() {
        // 10/40 used — slots 0..=9. Plenty of room.
        let raw_occupied: Vec<i32> = (0..10).collect();
        let picked = pick_free_main_slots_excluding(&raw_occupied, &[], 1);
        assert_eq!(
            picked,
            Some(vec![10]),
            "with 10/40 used and no exclusions, slot 10 is the lowest \
             free slot — must be returned"
        );
    }
}

#[cfg(test)]
mod parking_sentinel {
    //! Unit-level regression guard for the two-phase swap parking step.
    //!
    //! Background: `sgw_inventory` has a UNIQUE INDEX on
    //! `(character_id, container_id, slot_id)` (see
    //! `db/sgw/Inventory/Tables/sgw_inventory.sql`). The pre-fix
    //! single-statement re-key (item_a → recipient's destination slot,
    //! immediately followed by item_b → sender's destination slot)
    //! violated the constraint whenever the destination slot still
    //! contained the partner's outgoing-this-trade item — the trivial
    //! 1-for-1 swap where both players hold an item at INV_MAIN slot 0
    //! tripped it. The transaction rolled back, items stayed with
    //! their original owners, and the test diagnostics surfaced the
    //! recipient-slot accounting fix path even though the underlying
    //! cause was unique-index collision, not slot accounting.
    //!
    //! The fix: phase 1 parks each outgoing item at a distinct
    //! NEGATIVE slot in INV_MAIN, vacating its original slot before
    //! phase 2 re-keys the row to the recipient at its reserved
    //! positive slot. Distinctness within the parked set is critical —
    //! two items parked at the same `(player, INV_MAIN, sentinel)`
    //! would themselves collide on the UNIQUE INDEX.
    //!
    //! These tests pin the distinctness + negativity invariants of
    //! [`park_sentinel_slot`] so a refactor that breaks either
    //! property trips at the unit level. The live-DB tests
    //! `commit_swaps_items_atomically` and `lock_items_accepts_inv_main`
    //! are the integration-level revert-verifiers: removing the
    //! parking loop in `atomic_swap` makes them fail with `left:
    //! Some(player_a) right: Some(player_b)` (item didn't move).

    use super::park_sentinel_slot;
    use std::collections::HashSet;

    /// Every parked item must land on a distinct slot. The smallest
    /// trade that exposed the original bug had 2 items (one per side);
    /// the largest realistic case has 40 per side (full INV_MAIN
    /// each). Sweep the whole range.
    #[test]
    fn parked_slots_are_pairwise_distinct() {
        for total in [2usize, 4, 10, 40, 80] {
            let slots: Vec<i32> = (0..total as i32)
                .map(|n| park_sentinel_slot(n, total))
                .collect();
            let unique: HashSet<i32> = slots.iter().copied().collect();
            assert_eq!(
                unique.len(),
                total,
                "parking slots must be distinct for total={total}; \
                 got {slots:?}. A duplicate sentinel collides on the \
                 sgw_inventory_unique_slot index during phase 1."
            );
        }
    }

    /// Parked slots must be negative, so they never collide with a
    /// real container slot (every container's `bag_min_slot` is 0)
    /// and the grant / purchase / move paths can never accidentally
    /// land there.
    #[test]
    fn parked_slots_are_strictly_negative() {
        for total in [2usize, 40, 80] {
            for n in 0..total as i32 {
                let s = park_sentinel_slot(n, total);
                assert!(
                    s < 0,
                    "park_sentinel_slot({n}, {total}) = {s}; sentinel \
                     slots must be negative so they can't be reached \
                     from any legitimate slot-allocation path"
                );
            }
        }
    }
}

#[cfg(test)]
mod trade_results_code_mapping {
    //! Unit-level regression guard for the per-side asymmetric
    //! `ETradeResults` code mapping introduced for Clara's G7 review
    //! on PR #438.
    //!
    //! The mapping replaces the old "both sides see Cancelled on every
    //! abort" behavior with the Python-parity `NoLocal*`/`NoRemote*`
    //! codes from `Trade.py:237-263`. The canonical client uses these
    //! to surface a specific trade-results dialog string (e.g. "you
    //! don't have enough cash" vs. "they don't have enough space")
    //! rather than the generic teardown notification.
    //!
    //! Pinning the mapping at the unit level means a refactor that
    //! reverts to "Cancelled on both sides" trips here without needing
    //! a live-DB integration run. The live-DB tests already verify the
    //! Err arm fires for the underlying conditions
    //! (insufficient cash, full bag, bound item, etc.); this is the
    //! algorithmic-core proxy for the codes those Errs translate to.
    //!
    //! Revert-verifier: replacing `trade_abort_to_results_codes` with
    //! `|_, _, _| (ETRADERESULTS_CANCELLED, ETRADERESULTS_CANCELLED)`
    //! trips every assertion below.

    use super::*;

    const P1_PID: i32 = 1000;
    const P2_PID: i32 = 2000;

    #[test]
    fn insufficient_cash_p1_maps_to_no_local_cash_and_no_remote_cash() {
        let reason = TradeAbort::InsufficientCash {
            which: "p1",
            player_id: P1_PID,
            has: 5,
            wants: 10,
        };
        assert_eq!(
            trade_abort_to_results_codes(&reason, P1_PID, P2_PID),
            (ETRADERESULTS_NO_LOCAL_CASH, ETRADERESULTS_NO_REMOTE_CASH),
            "p1 short on cash: p1 sees NoLocalCash, p2 sees NoRemoteCash"
        );
    }

    #[test]
    fn insufficient_cash_p2_maps_to_no_remote_cash_and_no_local_cash() {
        let reason = TradeAbort::InsufficientCash {
            which: "p2",
            player_id: P2_PID,
            has: 0,
            wants: 100,
        };
        assert_eq!(
            trade_abort_to_results_codes(&reason, P1_PID, P2_PID),
            (ETRADERESULTS_NO_REMOTE_CASH, ETRADERESULTS_NO_LOCAL_CASH),
            "p2 short on cash: p1 sees NoRemoteCash, p2 sees NoLocalCash"
        );
    }

    #[test]
    fn not_enough_slots_resolves_recipient_against_player_ids() {
        // Recipient = p1 → p1 sees NoLocalSpace (their bag is full),
        // p2 sees NoRemoteSpace (partner is the one without room).
        let reason = TradeAbort::NotEnoughSlots {
            recipient_player_id: P1_PID,
            needed: 3,
        };
        assert_eq!(
            trade_abort_to_results_codes(&reason, P1_PID, P2_PID),
            (ETRADERESULTS_NO_LOCAL_SPACE, ETRADERESULTS_NO_REMOTE_SPACE)
        );

        // Recipient = p2 → mirrored.
        let reason = TradeAbort::NotEnoughSlots {
            recipient_player_id: P2_PID,
            needed: 3,
        };
        assert_eq!(
            trade_abort_to_results_codes(&reason, P1_PID, P2_PID),
            (ETRADERESULTS_NO_REMOTE_SPACE, ETRADERESULTS_NO_LOCAL_SPACE)
        );
    }

    /// Defensive: an abort variant with a stale `recipient_player_id`
    /// that matches neither side must fall back to Cancelled rather
    /// than guess. The atomic-commit-failure shape would still be
    /// surfaced to the player as the generic teardown string — which
    /// is correct: the trade is cancelled.
    #[test]
    fn not_enough_slots_unknown_recipient_falls_back_to_cancelled() {
        let reason = TradeAbort::NotEnoughSlots {
            recipient_player_id: 99999, // matches neither
            needed: 3,
        };
        assert_eq!(
            trade_abort_to_results_codes(&reason, P1_PID, P2_PID),
            (ETRADERESULTS_CANCELLED, ETRADERESULTS_CANCELLED)
        );
    }

    /// Catch-all variants (DbError, PlayerMissing, ItemMissing,
    /// DuplicateInstance, BoundItemOffered, IneligibleContainer) are
    /// either internal faults or server-authority validations the
    /// client UI has no dedicated string for — both sides see
    /// Cancelled.
    #[test]
    fn catch_all_variants_map_to_cancelled_on_both_sides() {
        let catch_alls = [
            TradeAbort::PlayerMissing {
                which: "p1",
                player_id: P1_PID,
            },
            TradeAbort::ItemMissing {
                which: "p2",
                player_id: P2_PID,
                item_id: 7,
            },
            TradeAbort::DuplicateInstance { item_id: 42 },
            TradeAbort::BoundItemOffered {
                which: "p1",
                player_id: P1_PID,
                item_id: 99,
            },
            TradeAbort::IneligibleContainer {
                which: "p1",
                player_id: P1_PID,
                item_id: 11,
                container_id: 8, // INV_EQUIP or similar
            },
        ];
        for reason in &catch_alls {
            assert_eq!(
                trade_abort_to_results_codes(reason, P1_PID, P2_PID),
                (ETRADERESULTS_CANCELLED, ETRADERESULTS_CANCELLED),
                "catch-all {reason:?} must map to Cancelled on both sides"
            );
        }
    }
}
