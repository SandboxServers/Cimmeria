//! Atomic-swap transaction internals.
//!
//! Owns the actual `BEGIN`-to-`COMMIT` pipeline: advisory locks,
//! `FOR UPDATE` reads of naquadah + items, the validation gauntlet,
//! the two-phase parked-row item move, and the cash debit/credit.
//! Public entry is [`atomic_swap`]; helpers (`lock_items`,
//! `reserve_main_slots_excluding`, `park_*`, `move_item_to_recipient`)
//! are crate-private but exposed as `pub(super)` so the unit-test
//! modules in [`super::tests`] can pin their invariants.

use std::sync::Arc;

use cimmeria_entity::inventory::INV_MAIN;
use sqlx::{PgPool, Postgres, Transaction};

use super::super::super::vendor::serializers::free_inventory_slots;
use super::{TradeAbort, TradeFinalBalances, TradeSide};
use crate::base::resources::{bag_max_slots, bag_min_slot};

#[derive(sqlx::FromRow)]
pub(super) struct TradeItemRow {
    pub(super) item_id: i32,
    pub(super) container_id: i32,
    pub(super) slot_id: i32,
    pub(super) bound: bool,
}

pub(super) async fn atomic_swap(
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
    // see the design note in `super::handle_execute_trade`.
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
///
/// The SQL form for the advisory lock — extracted into a constant so
/// the alignment test below can assert it byte-for-byte against the
/// vendor stack's lock SQL. Any divergence (single-arg form, different
/// namespace) breaks the alignment guarantee.
pub(super) const ADVISORY_LOCK_SQL: &str = "SELECT pg_advisory_xact_lock($1, $2)";

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
        // banker-NPC gate on bank items.
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
/// exclusion hook. Composition via the pure [`pick_free_main_slots_excluding`]
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
pub(super) fn pick_free_main_slots_excluding(
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
