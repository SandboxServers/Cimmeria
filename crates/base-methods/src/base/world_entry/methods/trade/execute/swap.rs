//! Atomic-swap transaction internals.
//!
//! Owns the actual `BEGIN`-to-`COMMIT` pipeline: advisory locks,
//! `FOR UPDATE` reads of the offered items, the recipients' destination
//! bags and naquadah, the validation gauntlet, the two-phase parked-row
//! item move, and the cash debit/credit. Public entry is [`atomic_swap`];
//! where each item lands and which slots it takes live in
//! [`super::placement`].
//!
//! ## Lock order
//!
//! The shared inventory order
//! (`crate::base::inventory_locks`): every advisory lock first,
//! then inventory rows, then `sgw_player` rows. For two players:
//!
//! 1. the lower `player_id`'s keys `(p, 0)`, `(p, 1)`, `(p, 15)`, then the
//!    higher's, via `take_inventory_locks`. Both tradeable bags are locked
//!    whichever holds the items: the destinations are only known once the
//!    rows are read, and the advisory locks must come first;
//! 2. the offered item rows, then every row in each recipient's
//!    destination bags;
//! 3. both `sgw_player` rows, ascending `player_id`.
//!
//! Crafting completions, vendor purchase, the move path, item use and the
//! gate-mail ops all take `(p, 0)` before any row, so each of them queues
//! behind a trade on the same player (or the trade behind it) instead of
//! holding a row the other needs.

use std::sync::Arc;

use cimmeria_entity::inventory::INV_MAIN;
use sqlx::{PgPool, Postgres, Transaction};

use super::placement::{plan_destinations, reserve_slots, ItemMove, TRADEABLE_CONTAINERS};
use super::{TradeAbort, TradeCommitted, TradeFinalBalances, TradeSide};
use crate::base::inventory_locks::take_inventory_locks;

#[derive(Debug, sqlx::FromRow)]
pub(super) struct TradeItemRow {
    pub(super) item_id: i32,
    pub(super) type_id: i32,
    pub(super) container_id: i32,
    pub(super) slot_id: i32,
    pub(super) bound: bool,
    /// The type's `resources.items.container_sets`; `None` when the type
    /// has no `resources.items` row (see `known_type`) or the column is
    /// NULL.
    pub(super) container_sets: Option<Vec<i32>>,
    /// Whether `resources.items` has a row for `type_id`.
    pub(super) known_type: bool,
}

#[tracing::instrument(
    name = "trade.atomic_swap",
    level = "info",
    skip_all,
    fields(
        p1_player = p1.player_id,
        p2_player = p2.player_id,
        p1_items = p1.item_instance_ids.len(),
        p2_items = p2.item_instance_ids.len(),
        p1_cash = p1.cash,
        p2_cash = p2.cash,
    ),
)]
pub(super) async fn atomic_swap(
    pool: &Arc<PgPool>,
    p1: &TradeSide,
    p2: &TradeSide,
) -> Result<TradeCommitted, TradeAbort> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

    // Per-phase debug! checkpoints fire BEFORE each `await?` so a
    // parent error log (`ExecuteTrade: atomic swap failed`) can be
    // narrowed to which phase by looking at the last-seen phase in
    // the SigNoz timeline for the same trace_id. debug level (not
    // info) per docs/architecture/instrumentation-discipline.md
    // rule 2 — these are sub-state breadcrumbs inside the parent
    // `trade.atomic_swap` span.

    // Ascending player_id is the convention for every path that locks
    // two players, so two trades sharing a player cannot deadlock.
    let (lo, hi) = if p1.player_id <= p2.player_id {
        (p1, p2)
    } else {
        (p2, p1)
    };
    tracing::debug!(
        target: "trade.atomic_swap",
        phase = "take_advisory_lock",
        lo_player = lo.player_id,
        hi_player = hi.player_id,
        "trade.atomic_swap: acquiring per-player advisory locks"
    );
    take_inventory_locks(&mut tx, lo.player_id, TRADEABLE_CONTAINERS).await?;
    take_inventory_locks(&mut tx, hi.player_id, TRADEABLE_CONTAINERS).await?;

    // Validate + lock items from each side. We pull the full row so we
    // can check `bound` (soul-bound items never change hands), detect
    // items the player claimed but doesn't actually own, and gate on
    // `container_id` against `TRADEABLE_CONTAINERS`.
    tracing::debug!(
        target: "trade.atomic_swap",
        phase = "lock_items",
        p1_count = p1.item_instance_ids.len(),
        p2_count = p2.item_instance_ids.len(),
        "trade.atomic_swap: SELECT FOR UPDATE on offered item rows"
    );
    let p1_items = lock_items(&mut tx, p1.player_id, &p1.item_instance_ids, "p1").await?;
    let p2_items = lock_items(&mut tx, p2.player_id, &p2.item_instance_ids, "p2").await?;

    // Where each item lands in its recipient's bags: decided by the
    // item's own `container_sets`, never by the client.
    let p1_dest = plan_destinations(&p1_items, p1.player_id, "p1")?;
    let p2_dest = plan_destinations(&p2_items, p2.player_id, "p2")?;

    tracing::debug!(
        target: "trade.atomic_swap",
        phase = "reserve_slots",
        p1_needed = p2_items.len(),
        p2_needed = p1_items.len(),
        "trade.atomic_swap: picking free destination slots for each recipient"
    );
    let p2_new_slots = reserve_slots(&mut tx, p2.player_id, &p1_dest, &p2_items).await?;
    let p1_new_slots = reserve_slots(&mut tx, p1.player_id, &p2_dest, &p1_items).await?;

    // Player rows last, ascending. SELECT FOR UPDATE serializes against
    // vendor purchase / sell / loot / mission grant paths. Ascending (not
    // p1 then p2) also matches gate mail without an item, which takes no
    // advisory lock (#913, pinned by `trade::tests::lock_order_live_db`).
    let lo_balance = read_naquadah_for_update(&mut tx, lo.player_id, which_of(lo, p1)).await?;
    let hi_balance = read_naquadah_for_update(&mut tx, hi.player_id, which_of(hi, p1)).await?;
    let (p1_balance, p2_balance) = if std::ptr::eq(lo, p1) {
        (lo_balance, hi_balance)
    } else {
        (hi_balance, lo_balance)
    };

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
    //            slot in INV_MAIN. character_id is left on the sender so
    //            the parked rows still belong to someone (FK +
    //            observability), only container_id/slot_id change.
    //   Phase 2: re-key each parked row to the recipient and into the
    //            reserved destination bag and slot. By this point every
    //            original slot is vacant on both sides, so no UNIQUE
    //            collision.
    //
    // Each parked item gets its OWN distinct negative slot so the parked
    // set itself can't collide. (The single-sentinel approach in
    // `inventory/move_` works there because that path swaps at most two
    // items; trade can move up to 40 per side.)
    let total_items = p1_items.len() + p2_items.len();
    for (parked_index, row) in (0_i32..).zip(p1_items.iter().chain(p2_items.iter())) {
        let sentinel = park_sentinel_slot(parked_index, total_items);
        park_item_at_sentinel(&mut tx, row.item_id, sentinel).await?;
    }
    let mut moves = Vec::with_capacity(total_items);
    for (row, &(container_id, slot_id)) in p1_items.iter().zip(p2_new_slots.iter()) {
        move_item_to_recipient(&mut tx, row.item_id, p2.player_id, container_id, slot_id).await?;
        moves.push(ItemMove::new(row, p1, p2, container_id, slot_id));
    }
    for (row, &(container_id, slot_id)) in p2_items.iter().zip(p1_new_slots.iter()) {
        move_item_to_recipient(&mut tx, row.item_id, p1.player_id, container_id, slot_id).await?;
        moves.push(ItemMove::new(row, p2, p1, container_id, slot_id));
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
    // pre-UPDATE balance under `FOR UPDATE` locks, and the delta is the
    // only mutation to naquadah in this transaction. Sourcing the totals
    // from inside the tx is what closes the race window a post-commit
    // `read_cash` would open — see the design note in
    // `super::handle_execute_trade`.
    let balances = TradeFinalBalances {
        p1: p1_balance + p1_delta,
        p2: p2_balance + p2_delta,
    };

    tx.commit().await?;
    Ok(TradeCommitted {
        balances,
        p1_before: p1_balance,
        p2_before: p2_balance,
        moves,
    })
}

/// `"p1"` or `"p2"` for `side`, by identity against `p1`.
fn which_of(side: &TradeSide, p1: &TradeSide) -> &'static str {
    if std::ptr::eq(side, p1) {
        "p1"
    } else {
        "p2"
    }
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
        // `FOR UPDATE OF i`: only the inventory row is locked; the
        // `resources.items` side of the outer join is read-only content.
        let row: Option<TradeItemRow> = sqlx::query_as::<_, TradeItemRow>(
            "SELECT i.item_id, i.type_id, i.container_id, i.slot_id, i.bound, \
                    ri.container_sets, (ri.item_id IS NOT NULL) AS known_type \
             FROM sgw_inventory i \
             LEFT JOIN resources.items ri ON ri.item_id = i.type_id \
             WHERE i.character_id = $1 AND i.item_id = $2 \
             FOR UPDATE OF i",
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
        // Whitelist gate, not a blacklist: equipped gear, mission items,
        // the bandolier, buyback and the vaults are all refused, and a
        // container added later is refused until someone decides
        // otherwise. Trading equipped gear would strip it while the cell
        // keeps its stats; vault items would bypass the banker gate.
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

async fn move_item_to_recipient(
    tx: &mut Transaction<'_, Postgres>,
    item_id: i32,
    recipient_player_id: i32,
    container_id: i32,
    slot_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sgw_inventory \
         SET character_id = $1, container_id = $2, slot_id = $3 \
         WHERE item_id = $4",
    )
    .bind(recipient_player_id)
    .bind(container_id)
    .bind(slot_id)
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
pub(super) fn park_sentinel_slot(nth: i32, _total: usize) -> i32 {
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
