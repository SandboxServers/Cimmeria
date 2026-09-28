//! Item escrow: a listed item is its own `sgw_inventory` row, moved into the
//! seller's auction container (18, `INV_AUCTION`) while the auction is open.
//!
//! The row keeps its instance id and every column (durability, charges,
//! ammo) and stays owned by the seller. That is the shape the social-systems
//! mail writer takes for a settlement (`SystemItem::ExistingInstance`, which
//! accepts only a container-18 row owned by `owner_player_id`), so packet
//! BM-02b can mail it without a snapshot. Container 18 is server-held: the
//! login and resync item sends skip it, and the move, use, trade, mail and
//! vendor paths refuse it.
//!
//! **Lock order.** A create takes the seller's inventory advisory locks
//! (`take_inventory_locks`: player-wide key, then per-bag keys), then the
//! item row `FOR UPDATE`, then the seller's `sgw_player` row: the shared
//! inventory order. A bid, cancel or settlement locks the `sgw_auction` row
//! first, then the advisory locks (seller's escrow, recipient's bags, in
//! ascending `player_id`), then `sgw_player` rows and the escrowed item row.
//! Every writer of a container-18 row holds the seller's escrow advisory
//! lock before touching the row, so the order between the item row and the
//! player rows cannot deadlock. No writer takes an inventory advisory lock
//! and then an `sgw_auction` row lock; keep it that way. Advisory locks are
//! re-entrant within a transaction, so a caller may take them early and a
//! helper here take them again.

use cimmeria_entity::inventory::{bag_max_slots, INV_AUCTION, INV_MAIN};
use sqlx::PgConnection;

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

/// Why an escrowed item could not be delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryRefused {
    /// Every carried bag of the recipient is full.
    BagFull,
    /// A player's listing has no container-18 row. Nothing removes one
    /// today; if something ever does, the listing must not mint a copy.
    EscrowMissing,
}

/// Is `auction` a boot-seed listing, which never had an instance?
pub fn is_seed_listing(auction: &AuctionRow) -> bool {
    auction.item_id == 0 || auction.seller_id == SYSTEM_SELLER_ID
}

/// Where a returned or delivered item landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    pub item_id: i32,
    pub container_id: i32,
    pub slot_id: i32,
    /// No escrow row existed (a boot-seed listing), so a new instance was
    /// made from the auction's snapshot.
    pub minted: bool,
    /// Every carried bag was full; the row went past the last slot of the
    /// main bag (sweep only).
    pub overflow: bool,
}

/// Take the advisory locks for moving `item` between two players' escrow
/// and bags, in ascending `player_id`: the seller's auction container, and
/// the recipient's carried bags.
pub async fn lock_for_delivery(
    conn: &mut PgConnection,
    seller_id: i32,
    recipient_id: i32,
) -> Result<(), sqlx::Error> {
    let seller_bags = if seller_id == recipient_id {
        vec![INV_AUCTION, LISTABLE_BAGS[0], LISTABLE_BAGS[1]]
    } else {
        vec![INV_AUCTION]
    };
    let mut plan = vec![(seller_id, seller_bags)];
    if seller_id != recipient_id {
        plan.push((recipient_id, LISTABLE_BAGS.to_vec()));
    }
    plan.sort_by_key(|(player, _)| *player);
    for (player, bags) in plan {
        take_inventory_locks(conn, player, &bags).await?;
    }
    Ok(())
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

/// The first free slot in `player_id`'s carried bags, in [`LISTABLE_BAGS`]
/// order. Call with the bags' advisory locks held.
pub async fn free_bag_slot(
    conn: &mut PgConnection,
    player_id: i32,
) -> Result<Option<(i32, i32)>, sqlx::Error> {
    for bag in LISTABLE_BAGS {
        let slot: Option<i32> = sqlx::query_scalar(
            "SELECT s FROM generate_series(0, $3 - 1) AS s \
             WHERE NOT EXISTS (SELECT 1 FROM sgw_inventory \
                               WHERE character_id = $1 AND container_id = $2 AND slot_id = s) \
             ORDER BY s LIMIT 1",
        )
        .bind(player_id)
        .bind(bag)
        .bind(bag_max_slots(bag))
        .fetch_optional(&mut *conn)
        .await?;
        if let Some(slot) = slot {
            return Ok(Some((bag, slot)));
        }
    }
    Ok(None)
}

/// Move `auction`'s escrowed item from the seller's container 18 into a
/// free slot of `recipient_id`'s bags: the seller again on cancel or
/// expiry, the buyer on a sale.
///
/// A boot-seed listing (no instance) is delivered as a new instance made
/// from the auction's snapshot; a player's listing whose row is missing is
/// refused (`EscrowMissing`), never minted. `BagFull` means every carried
/// bag is full; with `allow_overflow` (the sweep, which must settle) the row
/// goes past the main bag's last slot instead, as the branch's
/// `return_item` did, and `Placed::overflow` says so.
pub async fn deliver_from_escrow(
    conn: &mut PgConnection,
    auction: &AuctionRow,
    recipient_id: i32,
    allow_overflow: bool,
) -> Result<Result<Placed, DeliveryRefused>, sqlx::Error> {
    lock_for_delivery(conn, auction.seller_id, recipient_id).await?;
    let escrowed: Option<i32> = sqlx::query_scalar(
        "SELECT item_id FROM sgw_inventory \
         WHERE item_id = $1 AND character_id = $2 AND container_id = $3 FOR UPDATE",
    )
    .bind(auction.item_id)
    .bind(auction.seller_id)
    .bind(INV_AUCTION)
    .fetch_optional(&mut *conn)
    .await?;
    if escrowed.is_none() && !is_seed_listing(auction) {
        tracing::warn!(
            event = "bm.escrow_missing",
            auction_id = auction.sequence_id,
            seller_id = auction.seller_id,
            item_id = auction.item_id,
            reason = "escrow_missing",
            "Black Market listing has no container-18 row; refusing to deliver a copy"
        );
        return Ok(Err(DeliveryRefused::EscrowMissing));
    }

    let (container_id, slot_id, overflow) = match free_bag_slot(conn, recipient_id).await? {
        Some((bag, slot)) => (bag, slot, false),
        None if allow_overflow => {
            let slot: i32 = sqlx::query_scalar(
                "SELECT GREATEST(COALESCE(MAX(slot_id), -1) + 1, $3) FROM sgw_inventory \
                 WHERE character_id = $1 AND container_id = $2",
            )
            .bind(recipient_id)
            .bind(INV_MAIN)
            .bind(bag_max_slots(INV_MAIN))
            .fetch_one(&mut *conn)
            .await?;
            (INV_MAIN, slot, true)
        }
        None => return Ok(Err(DeliveryRefused::BagFull)),
    };

    let (item_id, minted) = match escrowed {
        Some(item_id) => {
            sqlx::query(
                "UPDATE sgw_inventory SET character_id = $2, container_id = $3, slot_id = $4 \
                 WHERE item_id = $1 AND container_id = $5",
            )
            .bind(item_id)
            .bind(recipient_id)
            .bind(container_id)
            .bind(slot_id)
            .bind(INV_AUCTION)
            .execute(&mut *conn)
            .await?;
            (item_id, false)
        }
        None => (
            mint_instance(conn, auction, recipient_id, container_id, slot_id).await?,
            true,
        ),
    };
    Ok(Ok(Placed {
        item_id,
        container_id,
        slot_id,
        minted,
        overflow,
    }))
}

/// A new instance from the auction's snapshot, with the template's ammo
/// defaults, at `(container_id, slot_id)`. Only for a listing that never
/// had an escrow row (the boot seed).
async fn mint_instance(
    conn: &mut PgConnection,
    auction: &AuctionRow,
    recipient_id: i32,
    container_id: i32,
    slot_id: i32,
) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges, ammo_type, ammo_types, ammo, flags) \
         SELECT $1, ri.item_id, $2, $3, $4, false, $5, $6, \
                COALESCE(ri.default_ammo_type, 'AMMO_NONE'::resources.\"EAmmoType\"), \
                ri.ammo_types, ri.charges, 0 \
         FROM resources.items ri WHERE ri.item_id = $7 \
         RETURNING item_id",
    )
    .bind(recipient_id)
    .bind(auction.stack_size)
    .bind(slot_id)
    .bind(container_id)
    .bind(auction.durability)
    .bind(auction.charges)
    .bind(auction.item_def_id)
    .fetch_one(&mut *conn)
    .await
}
