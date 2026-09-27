//! After an attached send commits: the before-and-after telemetry of the
//! cash and the item it moved, and the sender's client sync.

use std::sync::Arc;

use sqlx::PgPool;

use super::super::super::inventory::core::send_full_inventory_update;
use super::super::Caller;
use super::attachment::{self, Attachment};
use super::deliver::AttachedOutcome;
use super::SenderSession;
use crate::mercury::method_idx;

/// Log what an attached send moved, before and after, and bring the
/// sender's client up to date: `onCashChanged` with the balance read inside
/// the transaction, `onRemoveItem` when the whole row left the bag
/// (`onUpdateItem` only adds and updates), then the full inventory list.
pub(super) async fn attached_sent(
    caller: &Caller<'_>,
    session: SenderSession,
    pool: &PgPool,
    attachment: &Attachment,
    outcome: AttachedOutcome,
) {
    tracing::info!(
        target: "mail",
        event = "mail.cash_debited",
        entity_id = caller.entity_id,
        player_id = caller.player_id,
        account_id = session.account_id,
        target_player_id = outcome.recipient_id,
        mail_id = outcome.mail_id,
        naquadah_before = outcome.debit.before,
        naquadah_after = outcome.debit.after,
        postage = attachment::POSTAGE,
        cash = attachment.cash,
        cod = attachment.cod,
        "gate-mail postage and attached naquadah debited",
    );
    if let Some(item) = outcome.item {
        tracing::info!(
            target: "mail",
            event = "mail.item_escrowed",
            entity_id = caller.entity_id,
            player_id = caller.player_id,
            account_id = session.account_id,
            target_player_id = outcome.recipient_id,
            mail_id = outcome.mail_id,
            item_id = item.source_item_id,
            escrow_item_id = item.escrow_item_id,
            type_id = item.type_id,
            quantity = item.quantity,
            stack_before = item.stack_before,
            stack_after = item.stack_after,
            whole_row = item.whole_row(),
            "gate-mail item moved from the sender's bag into escrow",
        );
    }

    caller
        .send_to_caller(
            method_idx::ON_CASH_CHANGED,
            &outcome.debit.after.to_le_bytes(),
        )
        .await;
    if let Some(item) = outcome.item {
        if item.whole_row() {
            let mut args = Vec::with_capacity(8);
            args.extend_from_slice(&1u32.to_le_bytes()); // ARRAY<INT32> count
            args.extend_from_slice(&item.source_item_id.to_le_bytes());
            caller
                .send_to_caller(method_idx::ON_REMOVE_ITEM, &args)
                .await;
        }
        send_full_inventory_update(
            caller.entity_id,
            caller.player_id,
            &Arc::new(pool.clone()),
            caller.transport,
            caller.connected,
            caller.entity_to_addr,
        )
        .await;
    }
}
