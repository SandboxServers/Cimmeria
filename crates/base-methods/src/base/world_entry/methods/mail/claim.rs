//! What the four attachment ops (SS-M3: take cash, take item, pay COD,
//! return) share: the lock order, the locked mail row, the balance moves,
//! and how a refusal or a failure is answered.
//!
//! **Lock order.** Every op takes, in this order:
//!
//! 1. the caller's inventory advisory locks (`take_inventory_locks(caller,
//!    [INV_MAIN, INV_CRAFTING])`: key 0, then bag 1, then bag 15, the same
//!    keys and order as the SS-M2 send, crafting, the move path and vendor
//!    purchase). Both carried bags, because a take places an item by its
//!    `container_sets` and a crafting component goes to bag 15, and the
//!    destination is only known once the escrow row is read;
//! 2. the mail row, `FOR UPDATE`, found by `mail_id` **and** the caller's
//!    `character_id`;
//! 3. the escrow row and any inventory rows;
//! 4. `sgw_player` rows, `FOR UPDATE`, in ascending `player_id` order.
//!
//! That is the shared inventory order (advisory locks, rows, player rows)
//! with the mail row among the rows. The send never locks an existing mail
//! row, only inserts one, so no cycle runs through it. Pay COD and return
//! lock both players, ascending, like the send: without that, B paying A's
//! COD (holding B's row, then touching A's through the payment mail's
//! foreign key) could deadlock with A sending to B (holding A's row, then
//! locking B's).
//!
//! Two ops on one mail therefore serialise on its row lock (CAT-G-04), and
//! each re-reads the row after the other commits. The expiry sweep (SS-M4)
//! takes the same locks in the same order, so a sweep racing a take is one
//! more op on the row. Every write is also
//! conditional on the state it was decided on, with `rows_affected`
//! checked, so a missing lock degrades to a refusal, never a double payout.

use cimmeria_entity::inventory::{INV_CRAFTING, INV_MAIN};
use sqlx::PgConnection;

use super::MailCtx;
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::base::inventory_locks::take_inventory_locks;
use crate::cell::mail;
use crate::cell::mail::codes::flags::{MAIL_ARCHIVE, MAIL_COD};
use crate::mercury::method_idx;

/// The caller's mail row, locked.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct LockedMail {
    pub(super) cash: i64,
    pub(super) flags: i32,
    pub(super) sender_id: Option<i32>,
    pub(super) returned: bool,
    pub(super) cod_paid: bool,
    pub(super) subject: String,
    pub(super) sender_name: String,
    /// When the expiry sweep may take it (SS-M4); `None` never expires.
    pub(super) expires_at: Option<i32>,
}

impl LockedMail {
    pub(super) fn cod(&self) -> bool {
        self.flags & MAIL_COD != 0
    }

    pub(super) fn archived(&self) -> bool {
        self.flags & MAIL_ARCHIVE != 0
    }
}

/// Take the caller's inventory advisory locks, then lock `mail_id` if the
/// caller owns it. `None` for someone else's mail, a deleted one, or junk:
/// the three are indistinguishable to the caller on purpose.
///
/// A quarantined mail (SS-M4, D-SS04 path 3) is `None` too: it is out of
/// its owner's reach, so it can be neither taken, paid nor returned; only
/// a GM recovers it.
pub(super) async fn lock_mail(
    conn: &mut PgConnection,
    player_id: i32,
    mail_id: i32,
) -> Result<Option<LockedMail>, sqlx::Error> {
    take_inventory_locks(&mut *conn, player_id, &[INV_MAIN, INV_CRAFTING]).await?;
    sqlx::query_as::<_, LockedMail>(
        "SELECT cash, flags, sender_id, returned, cod_paid, subject, sender_name, expires_at \
         FROM sgw_gate_mail \
         WHERE mail_id = $1 AND character_id = $2 AND NOT quarantined FOR UPDATE",
    )
    .bind(mail_id)
    .bind(player_id)
    .fetch_optional(conn)
    .await
}

/// The escrowed item of a locked mail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub(super) struct EscrowItem {
    pub(super) item_id: i32,
    pub(super) type_id: i32,
    pub(super) stack_size: i32,
}

/// The mail's escrow row, locked, if it holds one.
pub(super) async fn lock_escrow(
    conn: &mut PgConnection,
    mail_id: i32,
) -> Result<Option<EscrowItem>, sqlx::Error> {
    sqlx::query_as::<_, EscrowItem>(
        "SELECT item_id, type_id, stack_size FROM sgw_gate_mail_item \
         WHERE mail_id = $1 FOR UPDATE",
    )
    .bind(mail_id)
    .fetch_optional(conn)
    .await
}

/// One locked `sgw_player` row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct LockedPlayer {
    pub(super) player_id: i32,
    pub(super) player_name: String,
}

/// Lock `ids`' `sgw_player` rows `FOR UPDATE`, ascending. Missing players
/// are simply absent from the result.
pub(super) async fn lock_players(
    conn: &mut PgConnection,
    ids: &[i32],
) -> Result<Vec<LockedPlayer>, sqlx::Error> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    sqlx::query_as::<_, LockedPlayer>(
        "SELECT player_id, player_name FROM sgw_player \
         WHERE player_id = ANY($1) ORDER BY player_id FOR UPDATE",
    )
    .bind(&ids)
    .fetch_all(conn)
    .await
}

/// A balance before and after a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Balance {
    pub(super) before: i32,
    pub(super) after: i32,
}

/// Credit `amount` to a locked player. `None` when the balance would pass
/// `i32::MAX` (`naquadah` is `integer`, a mail's `cash` is `bigint`):
/// nothing is written, and the cash stays in the mail.
pub(super) async fn credit(
    conn: &mut PgConnection,
    player_id: i32,
    amount: i32,
) -> Result<Option<Balance>, sqlx::Error> {
    let after: Option<i32> = sqlx::query_scalar(
        "UPDATE sgw_player SET naquadah = naquadah + $1 \
         WHERE player_id = $2 AND naquadah <= 2147483647 - $1 RETURNING naquadah",
    )
    .bind(amount)
    .bind(player_id)
    .fetch_optional(conn)
    .await?;
    Ok(after.map(|after| Balance {
        before: after - amount,
        after,
    }))
}

/// Debit `amount` from a locked player. `None` when the balance cannot
/// cover it; nothing is written.
pub(super) async fn debit(
    conn: &mut PgConnection,
    player_id: i32,
    amount: i32,
) -> Result<Option<Balance>, sqlx::Error> {
    let after: Option<i32> = sqlx::query_scalar(
        "UPDATE sgw_player SET naquadah = naquadah - $1 \
         WHERE player_id = $2 AND naquadah >= $1 RETURNING naquadah",
    )
    .bind(amount)
    .bind(player_id)
    .fetch_optional(conn)
    .await?;
    Ok(after.map(|after| Balance {
        before: after + amount,
        after,
    }))
}

/// Which op, for the `op` log field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Op {
    TakeCash,
    TakeItem,
    PayCod,
    Return,
}

impl Op {
    pub(super) fn name(self) -> &'static str {
        match self {
            Op::TakeCash => "take_cash",
            Op::TakeItem => "take_item",
            Op::PayCod => "pay_cod",
            Op::Return => "return",
        }
    }
}

/// Why an op was refused: a stable `reason` and the player's feedback line.
/// Nothing was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Refusal {
    pub(super) reason: &'static str,
    pub(super) text: &'static str,
}

/// Not the caller's mail, or no longer there. The client's header is stale,
/// so the answer also removes it.
pub(super) const NOT_FOUND: Refusal = Refusal {
    reason: "not_found_for_owner",
    text: "That gate-mail message is no longer in your mailbox.",
};

/// How an op failed.
#[derive(Debug)]
pub(super) enum OpError {
    Refused(Refusal),
    /// A conditional write changed the wrong number of rows although the
    /// row was locked. Rolled back; logged at ERROR.
    Invariant(&'static str),
    Db(sqlx::Error),
}

impl From<sqlx::Error> for OpError {
    fn from(e: sqlx::Error) -> Self {
        OpError::Db(e)
    }
}

impl From<Refusal> for OpError {
    fn from(r: Refusal) -> Self {
        OpError::Refused(r)
    }
}

/// Answer a failed op: log it with its reason, and tell the player on the
/// first press. Nothing was written (the transaction rolled back).
pub(super) async fn answer_failure(ctx: &MailCtx<'_>, op: Op, mail_id: i32, err: OpError) {
    let (entity_id, player_id, account_id) = (ctx.entity_id, ctx.player_id, ctx.account_id());
    // The mail's sender for `target_player_id`. The transaction rolled back,
    // so this is a plain owner-scoped read of the row as it stands; absent
    // for someone else's mail and for server mail.
    let target_player_id = match &err {
        OpError::Refused(r) if *r != NOT_FOUND => sqlx::query_scalar::<_, Option<i32>>(
            "SELECT sender_id FROM sgw_gate_mail WHERE mail_id = $1 AND character_id = $2",
        )
        .bind(mail_id)
        .bind(player_id)
        .fetch_optional(ctx.pool)
        .await
        .ok()
        .flatten()
        .flatten(),
        _ => None,
    };
    let text = match &err {
        OpError::Refused(refusal) => {
            tracing::warn!(
                target: "mail",
                event = "mail.op_refused",
                entity_id,
                player_id,
                account_id,
                target_player_id,
                op = op.name(),
                mail_id,
                reason = refusal.reason,
                "Mail: attachment op refused"
            );
            refusal.text
        }
        OpError::Invariant(reason) => {
            tracing::error!(
                target: "mail",
                event = "mail.op_failed",
                entity_id,
                player_id,
                account_id,
                op = op.name(),
                mail_id,
                reason = *reason,
                "Mail: a locked conditional write changed the wrong row count; rolled back"
            );
            FAILED_TEXT
        }
        OpError::Db(e) => {
            tracing::error!(
                target: "mail",
                event = "mail.op_failed",
                entity_id,
                player_id,
                account_id,
                op = op.name(),
                mail_id,
                reason = "db_error",
                error = %e,
                "Mail: attachment op failed; rolled back"
            );
            FAILED_TEXT
        }
    };
    if let Some(addr) = ctx.addr() {
        let fb = FeedbackCtx {
            transport: ctx.transport,
            connected: ctx.connected,
        };
        send_feedback_line(&fb, addr, text).await;
    }
    if matches!(err, OpError::Refused(r) if r == NOT_FOUND) {
        ctx.send_to_caller(
            method_idx::ON_MAIL_HEADER_REMOVE,
            &mail::serialize_on_mail_header_remove(mail_id),
        )
        .await;
    }
}

const FAILED_TEXT: &str = "The gate-mail request could not be completed. Nothing was changed.";

/// Seconds since the epoch, like `sgw_gate_mail.sent_time`.
pub(super) fn unix_now() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32
}
