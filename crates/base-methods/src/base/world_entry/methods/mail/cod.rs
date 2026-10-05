//! `payCODForMailMessage` (CM 51): the recipient pays a COD mail's price,
//! and the payment reaches the sender as a new mail carrying the cash
//! (D-SS09, CAT-G-05). The item is then taken with an ordinary take-item.
//! Lock order and failure handling: [`super::claim`].

use sqlx::{PgConnection, PgPool};

use super::claim::{
    answer_failure, debit, lock_escrow, lock_mail, lock_players, unix_now, Balance, Op, OpError,
    Refusal, NOT_FOUND,
};
use super::expiry::expires_at;
use super::headers::refresh_one;
use super::notify::{notify_delivered, Delivery};
use super::take::carried_bag;
use super::MailCtx;
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::cell::mail::codes::flags::MAIL_COD;
use crate::mercury::method_idx;

const NOT_COD: Refusal = Refusal {
    reason: "not_cod",
    text: "That gate-mail message has no COD to pay.",
};
const COD_WITHOUT_ITEM: Refusal = Refusal {
    reason: "cod_without_item",
    text: "That COD message holds no item, so there is nothing to pay for. \
           Return it to its sender.",
};
/// Told when a COD is cancelled because its sender's character is gone.
const SENDER_GONE_TEXT: &str = "The sender of that COD message no longer exists. The COD \
     is cancelled and nothing was charged; the item is yours to take.";
/// The COD's item may sit in no carried bag (see [`carried_bag`]), so the
/// take after a payment would refuse it for good. Refused before any debit;
/// the mail stays an unpaid COD, so it can still be returned.
const COD_NO_CARRIED_BAG: Refusal = Refusal {
    reason: "no_carried_bag",
    text: "That item cannot be carried in your backpack or crafting bag, so the COD \
           was not paid and nothing was charged. Return the message to its sender.",
};
const NOT_ENOUGH_CASH: Refusal = Refusal {
    reason: "not_enough_cash",
    text: "You do not have enough naquadah to pay this COD.",
};

/// The payment mail's subject prefix. `subject` is `varchar(128)`, so the
/// original subject is cut to fit.
const PAYMENT_SUBJECT_PREFIX: &str = "COD payment: ";
const SUBJECT_MAX_CHARS: usize = 128;

/// What a committed `payCODForMailMessage` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CodOutcome {
    Paid(CodPaid),
    /// The COD's sender no longer exists (their character was deleted, and
    /// the foreign key set `sender_id` NULL), so there is nobody to pay.
    /// The COD is cancelled instead, its price zeroed and nothing debited,
    /// so the item becomes an ordinary take. Without this the item would be
    /// stranded for good: pay has nobody to pay, return has nobody to
    /// return to, and take and delete refuse an unpaid COD.
    CancelledSenderGone {
        price: i64,
        /// The stored `sender_name`: with `sender_id` gone, the only trail
        /// to the seller.
        sender_name: String,
    },
}

/// A committed COD payment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodPaid {
    pub(super) price: i32,
    pub(super) balance: Balance,
    /// The COD's sender, who receives the payment mail.
    pub(super) sender_id: i32,
    /// The sender's stored name, for the log line only.
    pub(super) sender_name: String,
    pub(super) payment_mail_id: i32,
}

/// The payment mail's subject: the prefix and as much of the original
/// subject as fits in the column.
pub(super) fn payment_subject(subject: &str) -> String {
    PAYMENT_SUBJECT_PREFIX
        .chars()
        .chain(subject.chars())
        .take(SUBJECT_MAX_CHARS)
        .collect()
}

/// CAT-G-05: pay a COD once, at the price stored on the mail.
///
/// One transaction: lock the mail (the caller's), refuse unless it is an
/// unpaid COD with an item; lock the payer and the sender ascending (a
/// sender who no longer exists cancels the COD instead, see
/// [`CodOutcome::CancelledSenderGone`]); refuse an item no carried bag may
/// hold ([`carried_bag`]), since a take could never deliver it; debit the payer the stored price (never a client number);
/// clear `MAIL_COD` **and zero `cash`** with a conditional `UPDATE` that
/// must change one row (the delete guard keys on `cash = 0`, and the price
/// must never become takeable gift cash); insert the payment mail to the
/// sender. The payment mail is server mail: `sender_id` NULL (so it cannot
/// be returned, D-SS10), the payer's stored name as `sender_name`, exempt
/// from the mailbox cap (D-SS03), so it is delivered whether the sender is
/// online or not.
pub(super) async fn pay_cod_tx(
    pool: &PgPool,
    player_id: i32,
    mail_id: i32,
    now: i32,
) -> Result<CodOutcome, OpError> {
    let mut tx = pool.begin().await?;
    let mail = lock_mail(&mut tx, player_id, mail_id)
        .await?
        .ok_or(NOT_FOUND)?;
    if !mail.cod() || mail.cash <= 0 {
        return Err(NOT_COD.into());
    }
    // Escrow before player rows (the lock order in `claim`).
    let item = lock_escrow(&mut tx, mail_id).await?;
    // The sender's row, if it still exists; the payer's stored name.
    let mut live = None;
    if let Some(sender_id) = mail.sender_id {
        let players = lock_players(&mut tx, &[player_id, sender_id]).await?;
        if players.iter().any(|p| p.player_id == sender_id) {
            let payer = players
                .into_iter()
                .find(|p| p.player_id == player_id)
                .ok_or(NOT_FOUND)?;
            live = Some((sender_id, payer.player_name));
        }
    }
    let Some((sender_id, payer_name)) = live else {
        clear_cod(&mut tx, player_id, mail_id, mail.cash, false, now).await?;
        tx.commit().await?;
        return Ok(CodOutcome::CancelledSenderGone {
            price: mail.cash,
            sender_name: mail.sender_name,
        });
    };
    let item = item.ok_or(COD_WITHOUT_ITEM)?;
    // A paid COD can no longer be returned, so paying for an item no take
    // can deliver would leave the payer with neither item nor naquadah.
    if carried_bag(&mut tx, item.type_id).await?.is_none() {
        return Err(COD_NO_CARRIED_BAG.into());
    }
    let price = i32::try_from(mail.cash).map_err(|_| NOT_ENOUGH_CASH)?;
    let balance = debit(&mut tx, player_id, price)
        .await?
        .ok_or(NOT_ENOUGH_CASH)?;
    clear_cod(&mut tx, player_id, mail_id, mail.cash, true, now).await?;
    let body = format!(
        "{payer_name} paid {price} naquadah for the item you sent by COD (\"{}\").",
        mail.subject
    );
    let payment_mail_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_id, sender_name, subject, message, cash, \
             sent_time, read_time, flags, item_id, expires_at) \
         VALUES ($1, NULL, $2, $3, $4, $5, $6, 0, 0, NULL, $7) \
         RETURNING mail_id",
    )
    .bind(sender_id)
    .bind(&payer_name)
    .bind(payment_subject(&mail.subject))
    .bind(body)
    .bind(i64::from(price))
    .bind(now)
    .bind(expires_at(now))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(CodOutcome::Paid(CodPaid {
        price,
        balance,
        sender_id,
        sender_name: mail.sender_name,
        payment_mail_id,
    }))
}

/// Clear `MAIL_COD` **and zero `cash`** on a locked COD mail, with a
/// conditional `UPDATE` that must change one row. The delete guard keys on
/// `cash = 0`, and a price must never become takeable gift cash. `paid`
/// sets `cod_paid`: the item is now the recipient's, so the mail can no
/// longer be returned (nor, in SS-M4, expire back to the seller). A COD
/// cancelled because its sender is gone is not paid.
///
/// Either way the mail's 30 days restart at `now` (SS-M4): the item now
/// belongs to the recipient, and a COD paid on day 29 (or while the bags
/// are full) must not be quarantined out of reach hours later.
async fn clear_cod(
    conn: &mut PgConnection,
    player_id: i32,
    mail_id: i32,
    price: i64,
    paid: bool,
    now: i32,
) -> Result<(), OpError> {
    let cleared = sqlx::query(
        "UPDATE sgw_gate_mail SET cash = 0, flags = flags & ~$3, cod_paid = $5, expires_at = $6 \
         WHERE mail_id = $1 AND character_id = $2 AND (flags & $3) <> 0 AND cash = $4",
    )
    .bind(mail_id)
    .bind(player_id)
    .bind(MAIL_COD)
    .bind(price)
    .bind(paid)
    .bind(expires_at(now))
    .execute(conn)
    .await?
    .rows_affected();
    if cleared != 1 {
        // Only reachable without the row lock: the other payment won.
        return Err(NOT_COD.into());
    }
    Ok(())
}

/// `payCODForMailMessage(MailId)`. After the commit, the COD's sender, if
/// online, is told the payment mail arrived (D-SS11).
pub(super) async fn pay_cod(ctx: &MailCtx<'_>, mail_id: i32) {
    match pay_cod_tx(ctx.pool, ctx.player_id, mail_id, unix_now()).await {
        Ok(CodOutcome::Paid(paid)) => {
            let who = ctx.identity();
            tracing::info!(
                target: "mail",
                event = "mail.cod_paid",
                entity_id = ctx.entity_id,
                entity_name = who.player_name,
                player_id = ctx.player_id,
                player_name = who.player_name,
                account_id = ctx.account_id(),
                account_name = who.account_name,
                target_player_id = paid.sender_id,
                target_player_name = paid.sender_name.as_str(),
                mail_id, // nt:id-only mail row, its subject is player text kept out of logs
                payment_mail_id = paid.payment_mail_id, // nt:id-only payment mail row, its subject is player text kept out of logs
                price = paid.price,
                naquadah_before = paid.balance.before,
                naquadah_after = paid.balance.after,
                "gate-mail COD paid; the price was mailed to the sender",
            );
            ctx.send_to_caller(
                method_idx::ON_CASH_CHANGED,
                &paid.balance.after.to_le_bytes(),
            )
            .await;
            refresh_one(ctx, mail_id).await;
            let fb = FeedbackCtx {
                transport: ctx.transport,
                connected: ctx.connected,
            };
            notify_delivered(
                ctx.pool,
                &fb,
                paid.sender_id,
                paid.payment_mail_id,
                Delivery::CodPayment,
            )
            .await;
        }
        Ok(CodOutcome::CancelledSenderGone { price, sender_name }) => {
            let who = ctx.identity();
            tracing::info!(
                target: "mail",
                event = "mail.cod_cancelled",
                entity_id = ctx.entity_id,
                entity_name = who.player_name,
                player_id = ctx.player_id,
                player_name = who.player_name,
                account_id = ctx.account_id(),
                account_name = who.account_name,
                mail_id, // nt:id-only mail row, its subject is player text kept out of logs
                reason = "sender_gone",
                price,
                sender_name = %sender_name,
                "gate-mail COD cancelled: its sender no longer exists; nothing charged",
            );
            if let Some(addr) = ctx.addr() {
                let fb = FeedbackCtx {
                    transport: ctx.transport,
                    connected: ctx.connected,
                };
                send_feedback_line(&fb, addr, SENDER_GONE_TEXT).await;
            }
            refresh_one(ctx, mail_id).await;
        }
        Err(err) => answer_failure(ctx, Op::PayCod, mail_id, err).await,
    }
}
