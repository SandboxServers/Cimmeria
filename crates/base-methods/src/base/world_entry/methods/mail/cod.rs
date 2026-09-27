//! `payCODForMailMessage` (CM 51): the recipient pays a COD mail's price,
//! and the payment reaches the sender as a new mail carrying the cash
//! (D-SS09, CAT-G-05). The item is then taken with an ordinary take-item.
//! Lock order and failure handling: [`super::claim`].

use sqlx::PgPool;

use super::claim::{
    answer_failure, debit, lock_escrow, lock_mail, lock_players, unix_now, Balance, Op, OpError,
    Refusal, NOT_FOUND,
};
use super::headers::refresh_one;
use super::MailCtx;
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
const SENDER_GONE: Refusal = Refusal {
    reason: "sender_gone",
    text: "The sender of that COD message no longer exists, so it cannot be paid.",
};
const NOT_ENOUGH_CASH: Refusal = Refusal {
    reason: "not_enough_cash",
    text: "You do not have enough naquadah to pay this COD.",
};

/// The payment mail's subject prefix. `subject` is `varchar(128)`, so the
/// original subject is cut to fit.
const PAYMENT_SUBJECT_PREFIX: &str = "COD payment: ";
const SUBJECT_MAX_CHARS: usize = 128;

/// A committed COD payment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CodPaid {
    pub(super) price: i32,
    pub(super) balance: Balance,
    /// The COD's sender, who receives the payment mail.
    pub(super) sender_id: i32,
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
/// unpaid COD with an item and a live sender; lock the payer and the sender
/// ascending; debit the payer the stored price (never a client number);
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
) -> Result<CodPaid, OpError> {
    let mut tx = pool.begin().await?;
    let mail = lock_mail(&mut tx, player_id, mail_id)
        .await?
        .ok_or(NOT_FOUND)?;
    if !mail.cod() || mail.cash <= 0 {
        return Err(NOT_COD.into());
    }
    let sender_id = mail.sender_id.ok_or(SENDER_GONE)?;
    lock_escrow(&mut tx, mail_id)
        .await?
        .ok_or(COD_WITHOUT_ITEM)?;
    let players = lock_players(&mut tx, &[player_id, sender_id]).await?;
    if !players.iter().any(|p| p.player_id == sender_id) {
        return Err(SENDER_GONE.into());
    }
    let Some(payer) = players.iter().find(|p| p.player_id == player_id) else {
        return Err(NOT_FOUND.into());
    };
    let price = i32::try_from(mail.cash).map_err(|_| NOT_ENOUGH_CASH)?;
    let balance = debit(&mut tx, player_id, price)
        .await?
        .ok_or(NOT_ENOUGH_CASH)?;
    let cleared = sqlx::query(
        "UPDATE sgw_gate_mail SET cash = 0, flags = flags & ~$3 \
         WHERE mail_id = $1 AND character_id = $2 AND (flags & $3) <> 0 AND cash = $4",
    )
    .bind(mail_id)
    .bind(player_id)
    .bind(MAIL_COD)
    .bind(mail.cash)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if cleared != 1 {
        // Only reachable without the row lock: the other payment won.
        return Err(NOT_COD.into());
    }
    let body = format!(
        "{} paid {price} naquadah for the item you sent by COD (\"{}\").",
        payer.player_name, mail.subject
    );
    let payment_mail_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_id, sender_name, subject, message, cash, \
             sent_time, read_time, flags, item_id) \
         VALUES ($1, NULL, $2, $3, $4, $5, $6, 0, 0, NULL) \
         RETURNING mail_id",
    )
    .bind(sender_id)
    .bind(&payer.player_name)
    .bind(payment_subject(&mail.subject))
    .bind(body)
    .bind(i64::from(price))
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(CodPaid {
        price,
        balance,
        sender_id,
        payment_mail_id,
    })
}

/// `payCODForMailMessage(MailId)`.
pub(super) async fn pay_cod(ctx: &MailCtx<'_>, mail_id: i32) {
    match pay_cod_tx(ctx.pool, ctx.player_id, mail_id, unix_now()).await {
        Ok(paid) => {
            tracing::info!(
                target: "mail",
                event = "mail.cod_paid",
                entity_id = ctx.entity_id,
                player_id = ctx.player_id,
                account_id = ctx.account_id(),
                target_player_id = paid.sender_id,
                mail_id,
                payment_mail_id = paid.payment_mail_id,
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
        }
        Err(err) => answer_failure(ctx, Op::PayCod, mail_id, err).await,
    }
}
