//! Item escrow: a listed item is its own `sgw_inventory` row, moved into the
//! seller's auction container (18, `INV_AUCTION`) while the auction is open.
//!
//! The row keeps its instance id and every column (durability, charges,
//! ammo) and stays owned by the seller. That is the shape the social-systems
//! mail writer takes for a settlement (`SystemItem::ExistingInstance`, which
//! accepts only a container-18 row owned by `owner_player_id`), so every
//! settlement mails the row itself (BM-02b). Container 18 is server-held: the
//! login and resync item sends skip it, and the move, use, trade, mail and
//! vendor paths refuse it.
//!
//! **Lock order.** A create takes the seller's inventory advisory locks
//! (`take_inventory_locks`: player-wide key, then per-bag keys), then the
//! item row `FOR UPDATE`, then the seller's `sgw_player` row: the shared
//! inventory order. A bid, cancel or settlement locks the `sgw_auction` row
//! first, then the seller's escrow advisory locks ([`lock_escrow`]), then
//! the escrowed item row ([`escrowed_item`]), then every `sgw_player` row it
//! touches in ascending `player_id` (each mail recipient included), and
//! last the mail writer, whose own locks are re-locks of what is held.
//! Every writer of a container-18 row holds the seller's escrow advisory
//! lock before touching the row, so the order between the item row and the
//! player rows cannot deadlock. No writer takes an inventory advisory lock
//! and then an `sgw_auction` row lock; keep it that way. Advisory locks are
//! re-entrant within a transaction, so a caller may take them early and a
//! helper here take them again.

use cimmeria_entity::inventory::INV_AUCTION;
use sqlx::PgConnection;

use super::super::mail::SystemItem;
use super::seed::SYSTEM_SELLER_ID;
use super::types::{AuctionRow, LISTABLE_BAGS};
use super::wire::BMError;
use crate::base::crafting::inventory_locks::take_inventory_locks;

/// The listed row's state as the auction records it.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct EscrowedItem {
    pub item_id: i32,
    pub item_def_id: i32,
    pub stack_size: i32,
    pub durability: i32,
    pub charges: i32,
    /// The bag it was listed from.
    pub container_id: i32,
    pub bound: bool,
}

/// Is `auction` a boot-seed listing, which never had an instance?
pub fn is_seed_listing(auction: &AuctionRow) -> bool {
    auction.item_id == 0 || auction.seller_id == SYSTEM_SELLER_ID
}

/// Move `item_id` out of `seller_id`'s carried bags into their auction
/// container. `Ok(Err(_))` is a refusal: not the seller's row
/// (`InvalidItem`), not in a carried bag (`InvalidItem`), or bound
/// (`ItemBound`). Nothing is written on a refusal.
pub async fn list_into_escrow(
    conn: &mut PgConnection,
    seller_id: i32,
    item_id: i32,
) -> Result<Result<EscrowedItem, BMError>, sqlx::Error> {
    // Read the bag unlocked to know which bag lock to take, then re-read
    // under the locks: a row that moved in between is refused.
    let bag: Option<i32> = sqlx::query_scalar(
        "SELECT container_id FROM sgw_inventory WHERE character_id = $1 AND item_id = $2",
    )
    .bind(seller_id)
    .bind(item_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some(bag) = bag else {
        return Ok(Err(BMError::InvalidItem));
    };
    if !LISTABLE_BAGS.contains(&bag) {
        return Ok(Err(BMError::InvalidItem));
    }
    take_inventory_locks(conn, seller_id, &[bag, INV_AUCTION]).await?;

    let row: Option<EscrowedItem> = sqlx::query_as(
        "SELECT item_id, type_id AS item_def_id, stack_size, durability, charges, \
                container_id, bound \
         FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 FOR UPDATE",
    )
    .bind(seller_id)
    .bind(item_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some(item) = row.filter(|r| r.container_id == bag) else {
        return Ok(Err(BMError::InvalidItem));
    };
    if let Err(e) = super::validate::validate_listed_item(item.container_id, item.bound) {
        return Ok(Err(e));
    }

    let moved = sqlx::query(
        "UPDATE sgw_inventory SET container_id = $3, \
             slot_id = (SELECT COALESCE(MAX(slot_id), -1) + 1 FROM sgw_inventory \
                         WHERE character_id = $1 AND container_id = $3) \
         WHERE character_id = $1 AND item_id = $2 AND container_id = $4",
    )
    .bind(seller_id)
    .bind(item_id)
    .bind(INV_AUCTION)
    .bind(bag)
    .execute(&mut *conn)
    .await?;
    if moved.rows_affected() != 1 {
        // The row is locked and was just read in `bag`, so this cannot
        // happen short of a schema change; refuse rather than list nothing.
        return Ok(Err(BMError::Internal));
    }
    Ok(Ok(item))
}

/// Take the seller's escrow advisory locks (the player-wide key and
/// container 18). Every writer of a container-18 row holds them first.
pub async fn lock_escrow(conn: &mut PgConnection, seller_id: i32) -> Result<(), sqlx::Error> {
    take_inventory_locks(conn, seller_id, &[INV_AUCTION]).await
}

/// The item a settlement mails for `auction`, with the seller's escrow
/// locks held: the escrowed row itself (`ExistingInstance`), or for a
/// boot-seed listing, which never had a row, a new instance of the listed
/// type (`Minted`). `None` when a player's listing has no container-18 row:
/// that is `bm.escrow_missing`, and the listing is never paid out with a
/// minted copy, or anything that ever removed an escrowed row would become
/// an item dupe.
pub async fn escrowed_item(
    conn: &mut PgConnection,
    auction: &AuctionRow,
) -> Result<Option<SystemItem>, sqlx::Error> {
    if is_seed_listing(auction) {
        return Ok(Some(SystemItem::Minted {
            type_id: auction.item_def_id,
            qty: auction.stack_size,
        }));
    }
    let escrowed: Option<i32> = sqlx::query_scalar(
        "SELECT item_id FROM sgw_inventory          WHERE item_id = $1 AND character_id = $2 AND container_id = $3 FOR UPDATE",
    )
    .bind(auction.item_id)
    .bind(auction.seller_id)
    .bind(INV_AUCTION)
    .fetch_optional(&mut *conn)
    .await?;
    if escrowed.is_none() {
        tracing::warn!(
            event = "bm.escrow_missing",
            auction_id = auction.sequence_id,
            seller_id = auction.seller_id,
            item_id = auction.item_id,
            reason = "escrow_missing",
            "Black Market listing has no container-18 row; refusing to pay out a copy"
        );
        return Ok(None);
    }
    Ok(Some(SystemItem::ExistingInstance {
        item_id: auction.item_id,
        owner_player_id: auction.seller_id,
    }))
}
