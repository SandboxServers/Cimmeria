//! The attachment half of the send transaction (SS-M2, D-SS06 and D-SS08):
//! lock and check the sender's item, debit cash plus postage, and move the
//! item (or the split-off part of its stack) out of `sgw_inventory` into
//! `sgw_gate_mail_item`.
//!
//! Lock order is the shared inventory order
//! (`crate::base::crafting::inventory_locks`): the sender's advisory locks,
//! then the item row, then the `sgw_player` rows. [`lock_source_item`] runs
//! before `deliver` locks the player rows; the debit and the move run after.
//! This path only takes from a bag, never fills a slot; it takes the main
//! bag's key so it queues behind a move, a purchase or a craft in the
//! same order they do.
//!
//! The inventory handlers (`methods/inventory/`) are shared with the Bank
//! and Crafting campaigns and each open their own transaction, so this is a
//! second removal path, inside the send's transaction. It enqueues no
//! `CellOutboxPayload::InventoryItemRemoved`, unlike the inventory remove
//! paths: the cell's handler for it is one debug line
//! (`cell/service/base_messages/inventory_events.rs`), and a main-bag item
//! is no cell state. If that handler ever grows behaviour, this path needs
//! the payload too.

use cimmeria_entity::inventory::{INV_BANK, INV_BUYBACK, INV_COMMAND_BANK, INV_MAIN};
use sqlx::PgConnection;

use super::attachment::{
    AttachmentRefusal, ItemRequest, ITEM_BOUND, ITEM_IN_BUYBACK, ITEM_IN_VAULT, ITEM_NOT_FOUND,
    ITEM_NOT_IN_MAIN_BAG, ITEM_QUANTITY_EXCEEDS_STACK, NOT_ENOUGH_CASH,
};
use crate::base::crafting::inventory_locks::take_inventory_locks;

/// The sender's item row, locked `FOR UPDATE`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct SourceItem {
    pub(super) item_id: i32,
    pub(super) type_id: i32,
    pub(super) stack_size: i32,
    pub(super) container_id: i32,
    pub(super) bound: bool,
}

/// Take the sender's inventory locks and lock the item row. `None` when the
/// sender holds no such instance: someone else's id, a stale one, or junk.
pub(super) async fn lock_source_item(
    conn: &mut PgConnection,
    sender_id: i32,
    item: ItemRequest,
) -> Result<Option<SourceItem>, sqlx::Error> {
    take_inventory_locks(&mut *conn, sender_id, &[INV_MAIN]).await?;
    sqlx::query_as::<_, SourceItem>(
        "SELECT item_id, type_id, stack_size, container_id, bound \
         FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 FOR UPDATE",
    )
    .bind(sender_id)
    .bind(item.item_id)
    .fetch_optional(conn)
    .await
}

/// CAT-G-01 / D-SS08: the item is the sender's, in the main bag (the trade
/// allowlist, `trade/execute/swap.rs` `TRADEABLE_CONTAINERS`, so equipped,
/// bandolier, mission, crafting, vault and buyback items are all refused),
/// not bound, and holds at least the quantity asked for. Vault (17-20) and
/// buyback (16) items get their own reason and line: the owner's rule
/// (2026-09-27) is that mail, like vendors, trade and crafting, sees only
/// the backpack.
pub(super) fn check_source(
    source: Option<&SourceItem>,
    quantity: i32,
) -> Result<&SourceItem, AttachmentRefusal> {
    let source = source.ok_or(ITEM_NOT_FOUND)?;
    match source.container_id {
        INV_MAIN => {}
        INV_BANK..=INV_COMMAND_BANK => return Err(ITEM_IN_VAULT),
        INV_BUYBACK => return Err(ITEM_IN_BUYBACK),
        _ => return Err(ITEM_NOT_IN_MAIN_BAG),
    }
    if source.bound {
        return Err(ITEM_BOUND);
    }
    if quantity > source.stack_size {
        return Err(ITEM_QUANTITY_EXCEEDS_STACK);
    }
    Ok(source)
}

/// The sender's balance before and after a debit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct Debit {
    pub(in super::super) before: i32,
    pub(in super::super) after: i32,
}

/// Debit `cost` from the sender, whose row is already locked. Refused with
/// `NotEnoughCash` when the balance cannot cover it; nothing is written.
pub(super) async fn debit(
    conn: &mut PgConnection,
    sender_id: i32,
    cost: i64,
) -> Result<Result<Debit, AttachmentRefusal>, sqlx::Error> {
    let Ok(cost) = i32::try_from(cost) else {
        // More than any balance can hold.
        return Ok(Err(NOT_ENOUGH_CASH));
    };
    let after: Option<i32> = sqlx::query_scalar(
        "UPDATE sgw_player SET naquadah = naquadah - $1 \
         WHERE player_id = $2 AND naquadah >= $1 RETURNING naquadah",
    )
    .bind(cost)
    .bind(sender_id)
    .fetch_optional(conn)
    .await?;
    Ok(match after {
        Some(after) => Ok(Debit {
            before: after + cost,
            after,
        }),
        None => Err(NOT_ENOUGH_CASH),
    })
}

/// An item now in escrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct EscrowedItem {
    /// The escrow row's instance id: the source id on a whole move, a new
    /// id on a split.
    pub(in super::super) escrow_item_id: i32,
    /// The sender's inventory row it came from.
    pub(in super::super) source_item_id: i32,
    pub(in super::super) type_id: i32,
    pub(in super::super) quantity: i32,
    pub(in super::super) stack_before: i32,
    /// What is left in the sender's row; 0 when the whole row moved.
    pub(in super::super) stack_after: i32,
}

impl EscrowedItem {
    /// True when the sender's row is gone, so the client must be told to
    /// drop it (`onUpdateItem` only adds and updates).
    pub(in super::super) fn whole_row(&self) -> bool {
        self.stack_after == 0
    }
}

/// Whole-row move: copy every instance column, keeping the id. `$1` mail id,
/// `$2` sender, `$3` item, `$4` escrow time.
const ESCROW_WHOLE_ROW_SQL: &str = "INSERT INTO sgw_gate_mail_item \
     (mail_id, item_id, type_id, stack_size, charges, durability, flags, bound, \
      ammo, cur_ammo_type, ammo_type, ammo_types, source_character_id, escrowed_at) \
     SELECT $1, item_id, type_id, stack_size, charges, durability, flags, bound, \
            ammo, cur_ammo_type, ammo_type, ammo_types, character_id, $4 \
     FROM sgw_inventory WHERE character_id = $2 AND item_id = $3 \
     RETURNING item_id";

/// Split: the same copy with `$5` as the stack size and a fresh id from the
/// inventory sequence, so an id is never in both tables.
const ESCROW_SPLIT_SQL: &str = "INSERT INTO sgw_gate_mail_item \
     (mail_id, item_id, type_id, stack_size, charges, durability, flags, bound, \
      ammo, cur_ammo_type, ammo_type, ammo_types, source_character_id, escrowed_at) \
     SELECT $1, nextval('sgw_inventory_item_id_seq'), type_id, $5, charges, durability, \
            flags, bound, ammo, cur_ammo_type, ammo_type, ammo_types, character_id, $4 \
     FROM sgw_inventory WHERE character_id = $2 AND item_id = $3 \
     RETURNING item_id";

/// Move `quantity` of `source` into escrow for `mail_id`: the whole row when
/// it is the whole stack, otherwise a decrement of the sender's row plus a
/// new escrow row.
pub(super) async fn escrow_item(
    conn: &mut PgConnection,
    mail_id: i32,
    sender_id: i32,
    source: &SourceItem,
    quantity: i32,
    now: i32,
) -> Result<EscrowedItem, sqlx::Error> {
    let whole = quantity == source.stack_size;
    let escrow_item_id: i32 = if whole {
        let id: i32 = sqlx::query_scalar(ESCROW_WHOLE_ROW_SQL)
            .bind(mail_id)
            .bind(sender_id)
            .bind(source.item_id)
            .bind(now)
            .fetch_one(&mut *conn)
            .await?;
        let deleted =
            sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1 AND item_id = $2")
                .bind(sender_id)
                .bind(source.item_id)
                .execute(&mut *conn)
                .await?
                .rows_affected();
        if deleted != 1 {
            // The row is locked, so this cannot happen; roll back rather
            // than commit a copy.
            return Err(sqlx::Error::RowNotFound);
        }
        id
    } else {
        let shrunk = sqlx::query(
            "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
             WHERE character_id = $2 AND item_id = $3 AND stack_size > $1",
        )
        .bind(quantity)
        .bind(sender_id)
        .bind(source.item_id)
        .execute(&mut *conn)
        .await?
        .rows_affected();
        if shrunk != 1 {
            return Err(sqlx::Error::RowNotFound);
        }
        sqlx::query_scalar(ESCROW_SPLIT_SQL)
            .bind(mail_id)
            .bind(sender_id)
            .bind(source.item_id)
            .bind(now)
            .bind(quantity)
            .fetch_one(&mut *conn)
            .await?
    };
    Ok(EscrowedItem {
        escrow_item_id,
        source_item_id: source.item_id,
        type_id: source.type_id,
        quantity,
        stack_before: source.stack_size,
        stack_after: source.stack_size - quantity,
    })
}
