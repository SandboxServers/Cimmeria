//! `takeCashFromMailMessage` (CM 49) and `takeItemFromMailMessage` (CM 50):
//! move a mail's gift cash or escrowed item to its owner (CAT-G-02,
//! CAT-G-03, CAT-G-04). Lock order and failure handling: [`super::claim`].

use std::sync::Arc;

use cimmeria_cell_catalog::item_placement::first_player_container;
use cimmeria_entity::inventory::INV_CRAFTING;
use sqlx::{PgConnection, PgPool};

use super::super::inventory::core::send_full_inventory_update;
use super::super::vendor::serializers::reserve_free_inventory_slots;
use super::claim::{
    answer_failure, credit, lock_escrow, lock_mail, lock_players, Balance, EscrowItem, Op, OpError,
    Refusal, NOT_FOUND,
};
use super::headers::refresh_one;
use super::MailCtx;
use crate::cell::mail::codes::flags::MAIL_COD;
use crate::mercury::method_idx;

const COD_UNPAID_CASH: Refusal = Refusal {
    reason: "cod_unpaid",
    text: "The naquadah on a COD message is its price, not a gift. Pay the COD to \
           receive the item, or return the message.",
};
const COD_UNPAID_ITEM: Refusal = Refusal {
    reason: "cod_unpaid",
    text: "Pay the COD before taking the item, or return the message.",
};
const NO_CASH: Refusal = Refusal {
    reason: "no_cash",
    text: "That gate-mail message holds no naquadah.",
};
const NO_ITEM: Refusal = Refusal {
    reason: "no_item",
    text: "That gate-mail message holds no item.",
};
const BALANCE_OVERFLOW: Refusal = Refusal {
    reason: "balance_overflow",
    text: "You cannot carry that much naquadah. The naquadah stays in the message.",
};
const BAGS_FULL: Refusal = Refusal {
    reason: "bags_full",
    text: "Your backpack is full. Make room and take the item again; it stays in \
           the message until then.",
};
const CRAFTING_BAG_FULL: Refusal = Refusal {
    reason: "crafting_bag_full",
    text: "Your crafting bag is full. Make room and take the item again; it stays in \
           the message until then.",
};
const NO_CARRIED_BAG: Refusal = Refusal {
    reason: "no_carried_bag",
    text: "That item cannot be carried in your backpack or crafting bag, so it stays \
           in the message.",
};

/// A committed cash take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CashTaken {
    pub(super) amount: i32,
    pub(super) balance: Balance,
    pub(super) sender_id: Option<i32>,
    /// The sender's stored name, for the log line only; `None` for server mail.
    pub(super) sender_name: Option<String>,
}

/// CAT-G-02: move a mail's gift cash to its owner, once.
///
/// The mail row is locked and read, refused while it is an unpaid COD (the
/// amount is its price, D-SS09) or holds nothing; then `cash` is zeroed by
/// one conditional `UPDATE` (`cash > 0 AND` not COD) that must change
/// exactly one row, and only then is the owner credited, with the `i32`
/// overflow check. One transaction.
pub(super) async fn take_cash_tx(
    pool: &PgPool,
    player_id: i32,
    mail_id: i32,
) -> Result<CashTaken, OpError> {
    let mut tx = pool.begin().await?;
    let mail = lock_mail(&mut tx, player_id, mail_id)
        .await?
        .ok_or(NOT_FOUND)?;
    if mail.cod() {
        return Err(COD_UNPAID_CASH.into());
    }
    if mail.cash <= 0 {
        return Err(NO_CASH.into());
    }
    // `cash` is bigint; the balance it lands in is integer.
    let amount = i32::try_from(mail.cash).map_err(|_| BALANCE_OVERFLOW)?;
    let zeroed = sqlx::query(
        "UPDATE sgw_gate_mail SET cash = 0 \
         WHERE mail_id = $1 AND character_id = $2 AND cash > 0 AND (flags & $3) = 0",
    )
    .bind(mail_id)
    .bind(player_id)
    .bind(MAIL_COD)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if zeroed != 1 {
        // Only reachable without the row lock: the other take won.
        return Err(NO_CASH.into());
    }
    lock_players(&mut tx, &[player_id]).await?;
    let balance = credit(&mut tx, player_id, amount)
        .await?
        .ok_or(BALANCE_OVERFLOW)?;
    tx.commit().await?;
    Ok(CashTaken {
        amount,
        balance,
        sender_id: mail.sender_id,
        sender_name: mail.sender_id.map(|_| mail.sender_name),
    })
}

/// `takeCashFromMailMessage(MailId)`.
pub(super) async fn take_cash(ctx: &MailCtx<'_>, mail_id: i32) {
    match take_cash_tx(ctx.pool, ctx.player_id, mail_id).await {
        Ok(taken) => {
            let who = ctx.identity();
            tracing::info!(
                target: "mail",
                event = "mail.cash_taken",
                entity_id = ctx.entity_id,
                entity_name = who.player_name,
                player_id = ctx.player_id,
                player_name = who.player_name,
                account_id = ctx.account_id(),
                account_name = who.account_name,
                target_player_id = taken.sender_id,
                target_player_name = taken.sender_name.as_deref(),
                mail_id, // nt:id-only mail row, its subject is player text kept out of logs
                cash = taken.amount,
                naquadah_before = taken.balance.before,
                naquadah_after = taken.balance.after,
                "gate-mail naquadah moved from the mail to its owner",
            );
            ctx.send_to_caller(
                method_idx::ON_CASH_CHANGED,
                &taken.balance.after.to_le_bytes(),
            )
            .await;
            refresh_one(ctx, mail_id).await;
        }
        Err(err) => answer_failure(ctx, Op::TakeCash, mail_id, err).await,
    }
}

/// A committed item take.
///
/// Like the send's escrow move (`send/escrow.rs`), the take enqueues no
/// `CellOutboxPayload::InventoryItemGranted`: the cell's handler for it is
/// one debug line and a main-bag item is no cell state. If that handler
/// ever grows behaviour, both mail paths need the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ItemTaken {
    pub(super) item: EscrowItem,
    /// The bag it landed in: 1, or 15 for a crafting component.
    pub(super) container_id: i32,
    pub(super) slot_id: i32,
    pub(super) sender_id: Option<i32>,
    /// The sender's stored name, for the log line only; `None` for server mail.
    pub(super) sender_name: Option<String>,
}

/// The carried bag a take would place an item of `type_id` in: the first
/// main (1) or crafting (15) bag its `container_sets` lists, the main bag
/// for an empty list, and `None` for a type that may sit in no carried bag
/// (801 mission-only `{2}` types) or is not in `resources.items`.
///
/// The one placement rule for mail: the take places by it, and the send
/// and the COD payment refuse an item it has no bag for (ss-fix1), so an
/// item never enters escrow, or is paid for, when no take can deliver it.
pub(super) async fn carried_bag(
    conn: &mut PgConnection,
    type_id: i32,
) -> Result<Option<i32>, sqlx::Error> {
    let container_sets: Option<Vec<i32>> = sqlx::query_scalar(
        "SELECT COALESCE(container_sets, '{}') FROM resources.items WHERE item_id = $1",
    )
    .bind(type_id)
    .fetch_optional(conn)
    .await?;
    Ok(container_sets.as_deref().and_then(first_player_container))
}

/// Escrow back into `sgw_inventory`, every instance column restored, the
/// instance id kept. `$1` mail, `$2` owner, `$3` container, `$4` slot.
const RESTORE_SQL: &str = "INSERT INTO sgw_inventory \
     (item_id, character_id, type_id, stack_size, container_id, slot_id, charges, \
      durability, flags, bound, ammo, cur_ammo_type, ammo_type, ammo_types) \
     SELECT item_id, $2, type_id, stack_size, $3, $4, charges, durability, flags, bound, \
            ammo, cur_ammo_type, ammo_type, ammo_types \
     FROM sgw_gate_mail_item WHERE mail_id = $1";

/// CAT-G-03: move a mail's escrowed item to its owner's carried bags, once.
///
/// The escrow row is found by the locked mail's id, never by type. The
/// destination is chosen here, by the item's `resources.items.container_sets`
/// and the grant rule (`item_placement::first_player_container`, crafting
/// CR-06): the first carried bag it lists, so the backpack (1) for most
/// items and the crafting bag (15) for a crafting component (`{17,15}`);
/// storage, bandolier and equipment entries are passed over, so never a
/// vault. Its first free slot is reserved under the bag's advisory lock
/// (already held from `lock_mail`). Never anything the client names:
/// `takeItemFromMailMessage`'s `ContainerId` and `SlotId` are uninitialised
/// stack in the shipped client (SS-E1 M-Q5), so this function does not take
/// them. An item with no carried bag, or a full destination bag, leaves the
/// item in escrow; a full bag never spills into another bag. The escrow row
/// is deleted with `rows_affected == 1`.
pub(super) async fn take_item_tx(
    pool: &PgPool,
    player_id: i32,
    mail_id: i32,
) -> Result<ItemTaken, OpError> {
    let mut tx = pool.begin().await?;
    let mail = lock_mail(&mut tx, player_id, mail_id)
        .await?
        .ok_or(NOT_FOUND)?;
    if mail.cod() {
        return Err(COD_UNPAID_ITEM.into());
    }
    let item = lock_escrow(&mut tx, mail_id).await?.ok_or(NO_ITEM)?;
    let container_id = carried_bag(&mut tx, item.type_id)
        .await?
        .ok_or(NO_CARRIED_BAG)?;
    let full = if container_id == INV_CRAFTING {
        CRAFTING_BAG_FULL
    } else {
        BAGS_FULL
    };
    let slot_id = reserve_free_inventory_slots(&mut tx, player_id, container_id, 1)
        .await?
        .and_then(|slots| slots.first().copied())
        .ok_or(full)?;
    let restored = sqlx::query(RESTORE_SQL)
        .bind(mail_id)
        .bind(player_id)
        .bind(container_id)
        .bind(slot_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if restored != 1 {
        return Err(OpError::Invariant("restore_row_count"));
    }
    let removed = sqlx::query("DELETE FROM sgw_gate_mail_item WHERE mail_id = $1")
        .bind(mail_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if removed != 1 {
        return Err(OpError::Invariant("escrow_delete_row_count"));
    }
    tx.commit().await?;
    Ok(ItemTaken {
        item,
        container_id,
        slot_id,
        sender_id: mail.sender_id,
        sender_name: mail.sender_id.map(|_| mail.sender_name),
    })
}

/// `takeItemFromMailMessage(MailId, ContainerId, SlotId)`. The last two are
/// only logged (see [`take_item_tx`]).
pub(super) async fn take_item(
    ctx: &MailCtx<'_>,
    mail_id: i32,
    client_container_id: i32,
    client_slot_id: i32,
) {
    let who = ctx.identity();
    tracing::debug!(
        target: "mail",
        entity_id = ctx.entity_id,
        entity_name = who.player_name,
        player_id = ctx.player_id,
        player_name = who.player_name,
        account_id = who.account_id,
        account_name = who.account_name,
        mail_id, // nt:id-only mail row, its subject is player text kept out of logs
        client_container_id, // nt:id-only client-sent bag index, ignored by the server
        client_slot_id, // nt:id-only client-sent slot index, ignored by the server
        "Mail: take item (client container and slot ignored, SS-E1 M-Q5)"
    );
    match take_item_tx(ctx.pool, ctx.player_id, mail_id).await {
        Ok(taken) => {
            let who = ctx.identity();
            let book = cimmeria_names::book();
            tracing::info!(
                target: "mail",
                event = "mail.item_taken",
                entity_id = ctx.entity_id,
                entity_name = who.player_name,
                player_id = ctx.player_id,
                player_name = who.player_name,
                account_id = ctx.account_id(),
                account_name = who.account_name,
                target_player_id = taken.sender_id,
                target_player_name = taken.sender_name.as_deref(),
                mail_id, // nt:id-only mail row, its subject is player text kept out of logs
                item_id = taken.item.item_id,
                item_type_id = taken.item.type_id,
                item_name = book.item(taken.item.type_id),
                stack_size = taken.item.stack_size,
                container_id = taken.container_id, // nt:id-only inventory bag index (1 or 15), not a named row
                slot_id = taken.slot_id, // nt:id-only slot index inside a bag, a counter with no name
                "gate-mail item moved from escrow to its owner's backpack",
            );
            send_full_inventory_update(
                ctx.entity_id,
                ctx.player_id,
                &Arc::new(ctx.pool.clone()),
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await;
            refresh_one(ctx, mail_id).await;
        }
        Err(err) => answer_failure(ctx, Op::TakeItem, mail_id, err).await,
    }
}
