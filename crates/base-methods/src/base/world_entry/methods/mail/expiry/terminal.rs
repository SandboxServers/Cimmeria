//! One expiring mail, in one transaction: the three D-SS04 terminal paths
//! (return, delete, quarantine), chosen under the mail's row lock.

use sqlx::PgPool;

use super::super::claim::{lock_escrow, lock_mail, OpError};
use super::super::return_::{return_locked, SYSTEM_MAIL};
use crate::cell::mail::codes::flags::MAIL_COD;

/// Which terminal path an expired mail took.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum ExpiryPath {
    /// D-SS04 path 1: back to its sender, once, by SS-M3's return.
    Returned { to_player_id: i32 },
    /// Path 2: nothing attached, so the row is gone.
    Deleted,
    /// Path 3: it still holds an item or gift cash and there is nobody it
    /// may go back to. Kept, with its escrow row, out of every list.
    Quarantined { reason: &'static str },
}

impl ExpiryPath {
    /// Stable `path` log value.
    pub(in super::super) fn name(self) -> &'static str {
        match self {
            ExpiryPath::Returned { .. } => "returned",
            ExpiryPath::Deleted => "deleted",
            ExpiryPath::Quarantined { .. } => "quarantined",
        }
    }
}

/// A committed expiry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct Expired {
    pub(in super::super) mail_id: i32,
    /// The mailbox it expired from.
    pub(in super::super) owner: i32,
    /// Its `sender_id` when it expired (`None` for server mail, or a sender
    /// whose character is gone).
    pub(in super::super) sender_id: Option<i32>,
    pub(in super::super) path: ExpiryPath,
    pub(in super::super) expires_at: i32,
    /// Gift cash on the mail: what went back with a return, what a
    /// quarantined mail keeps; 0 for a delete.
    pub(in super::super) cash: i64,
    /// An unpaid COD price cancelled by the expiry (0 when it was no COD).
    pub(in super::super) cod_cancelled: i64,
    /// The escrowed instance it still carries, if any.
    pub(in super::super) item_id: Option<i32>,
    /// For the owner's line when it is quarantined.
    pub(in super::super) subject: String,
}

/// What [`expire_one`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) enum ExpireOutcome {
    Expired(Expired),
    /// Nothing changed. `reason`: `not_found_for_owner` (gone, moved to
    /// another mailbox, or quarantined since the scan), `archived`, or
    /// `not_due` (no expiry, or not yet).
    Skipped(&'static str),
}

/// Expire `mail_id` from `owner`'s mailbox at `now`, in one transaction.
///
/// Locks exactly as the attachment ops do (`claim::lock_mail`: the owner's
/// inventory advisory locks, then the mail row `FOR UPDATE`, then the escrow
/// row), so a take, pay or return racing the sweep serialises on the row
/// and each side re-reads what the other committed. The due check is made
/// under that lock, so a mail archived since the scan is skipped. Then:
///
/// 1. **Nothing attached** (no escrow row, no gift cash): deleted, unless it
///    is an unpaid COD that can still go back to its sender (path 1, per
///    D-SS04). A COD price is not value: a COD with no item and nobody to
///    return it to is deleted, not quarantined empty.
/// 2. **Returnable** (never returned, not a paid COD, a `sender_id`):
///    [`return_locked`], SS-M3's return, in this transaction. An unpaid COD
///    is cancelled there with its price zeroed, so the price never becomes
///    gift cash (D-SS04). A sender whose character has gone is treated as
///    not returnable.
/// 3. **Otherwise** (already returned, a paid COD whose item was never
///    taken, server mail, a sender gone): quarantined, never deleted with
///    value on it. An unpaid COD price on such a mail is zeroed in the same
///    `UPDATE`.
pub(in super::super) async fn expire_one(
    pool: &PgPool,
    owner: i32,
    mail_id: i32,
    now: i32,
) -> Result<ExpireOutcome, OpError> {
    let mut tx = pool.begin().await?;
    let Some(mail) = lock_mail(&mut tx, owner, mail_id).await? else {
        return Ok(ExpireOutcome::Skipped("not_found_for_owner"));
    };
    if mail.archived() {
        return Ok(ExpireOutcome::Skipped("archived"));
    }
    let Some(expires_at) = mail.expires_at.filter(|at| *at <= now) else {
        return Ok(ExpireOutcome::Skipped("not_due"));
    };
    let item = lock_escrow(&mut tx, mail_id).await?;
    let cod = mail.cod();
    let gift_cash = if cod { 0 } else { mail.cash };
    let returnable = !mail.returned && !mail.cod_paid && mail.sender_id.is_some();
    let mut expired = Expired {
        mail_id,
        owner,
        sender_id: mail.sender_id,
        path: ExpiryPath::Deleted,
        expires_at,
        cash: gift_cash,
        cod_cancelled: if cod { mail.cash } else { 0 },
        item_id: item.map(|i| i.item_id),
        subject: mail.subject.clone(),
    };

    if item.is_none() && gift_cash == 0 && !(cod && returnable) {
        // `cash` here is 0, or the price of an unpaid COD nobody can pay.
        let deleted = sqlx::query(
            "DELETE FROM sgw_gate_mail m \
             WHERE m.mail_id = $1 AND m.character_id = $2 \
               AND (m.cash = 0 OR (m.flags & $3) <> 0) AND NOT m.quarantined \
               AND NOT EXISTS (SELECT 1 FROM sgw_gate_mail_item i WHERE i.mail_id = m.mail_id)",
        )
        .bind(mail_id)
        .bind(owner)
        .bind(MAIL_COD)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if deleted != 1 {
            return Err(OpError::Invariant("expire_delete_row_count"));
        }
        tx.commit().await?;
        return Ok(ExpireOutcome::Expired(expired));
    }

    let quarantine_reason = if mail.returned {
        Some("already_returned")
    } else if mail.cod_paid {
        // A paid COD's item is the recipient's: returning it would give the
        // seller the item and the price (SS-M3 integration edit).
        Some("cod_paid")
    } else if mail.sender_id.is_none() {
        Some("system_mail")
    } else {
        None
    };
    let quarantine_reason = match quarantine_reason {
        Some(reason) => reason,
        None => match return_locked(&mut tx, owner, mail_id, &mail, now).await {
            Ok(returned) => {
                tx.commit().await?;
                expired.path = ExpiryPath::Returned {
                    to_player_id: returned.to_player_id,
                };
                expired.cash = returned.cash;
                expired.cod_cancelled = returned.cod_cancelled;
                return Ok(ExpireOutcome::Expired(expired));
            }
            // `sender_id` is set but the row is gone (the foreign key nulls
            // it on delete, so only a race with a character delete).
            Err(OpError::Refused(r)) if r == SYSTEM_MAIL => "sender_gone",
            Err(e) => return Err(e),
        },
    };

    let quarantined = sqlx::query(
        "UPDATE sgw_gate_mail SET quarantined = true, expires_at = NULL, \
                cash = CASE WHEN (flags & $3) <> 0 THEN 0 ELSE cash END, flags = flags & ~$3 \
         WHERE mail_id = $1 AND character_id = $2 AND NOT quarantined",
    )
    .bind(mail_id)
    .bind(owner)
    .bind(MAIL_COD)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if quarantined != 1 {
        return Err(OpError::Invariant("expire_quarantine_row_count"));
    }
    tx.commit().await?;
    expired.path = ExpiryPath::Quarantined {
        reason: quarantine_reason,
    };
    Ok(ExpireOutcome::Expired(expired))
}
