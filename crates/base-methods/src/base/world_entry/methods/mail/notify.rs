//! New-mail notification (D-SS11): tell an online recipient, after the
//! commit, that a mail reached their mailbox.
//!
//! **Mechanism, and the evidence for it.** SS-E1 left M-Q6 open; the
//! client's own code settles it:
//!
//! - There is no client method for "you have mail". `onNewMail` is a cell
//!   method and `notifyPlayersOfNewMail` a base method
//!   (`SGWMailManager.def`, audit A-13).
//! - `onMailHeaderInfo`'s decoder upserts each row by id into the list its
//!   own `MAIL_Archive` bit names, whether or not the mailbox is open
//!   (M-Q7, `SGW.exe@0x00e15450`).
//! - The only Lua reader of the mailbox lists is `GateMail.lua`. Its
//!   refresh, `GateMailMod.onUpdateMailbox` (`:40-69`, on
//!   `Events.MailUpdateMailbox`, `:408`), redraws the inbox window's own
//!   rows and an open read window, and nothing else. No other UI file
//!   listens to a mail event, and there is no new-mail icon or sound (the
//!   minimap mail button is commented out, `MinimapButtons.lua:34`).
//!
//! So an unsolicited one-row `onMailHeaderInfo` (`ResetCategory` 0, which
//! leaves the rest of the list alone) makes an open mailbox show the mail at
//! once and is harmless when the window is closed, but it is invisible
//! then. The visible cue is a feedback line ("You have new gate-mail from
//! X."). Both are sent. A capture with the window closed would confirm the
//! decoder has no side effect beyond the upsert (M-Q6 stays MEDIUM).
//!
//! Only a recipient listed in the [`OnlinePlayerIndex`] (in the world,
//! not logged off) is told. Everyone else sees the mail on the next
//! mailbox open, as before. Every send is addressed to the session's
//! current player entity for that `player_id` (`send_to_current_player`),
//! so a gate travel or a character switch between the lookup and the send
//! cannot deliver to the wrong entity.

use sqlx::PgPool;

use super::headers::read_one;
use crate::base::feedback::{
    send_to_current_player, serialize_on_player_communication, FeedbackCtx, FeedbackOutcome,
    CHAN_FEEDBACK, FEEDBACK_SPEAKER,
};
use crate::base::player_index::OnlinePlayerIndex;
use crate::cell::mail;
use crate::mercury::method_idx;

/// How the mail reached the mailbox; picks the feedback line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Delivery {
    /// A player's `sendMailMessage`.
    Sent,
    /// The recipient's own mail, returned by the player it was sent to.
    Returned,
    /// The payment for a COD the recipient sent.
    CodPayment,
    /// The recipient's own mail, returned by the expiry sweep (D-SS04
    /// path 1).
    ExpiredReturn,
    /// Server mail (SS-U1's writer, the GM `.mail`).
    System,
}

impl Delivery {
    /// Stable `delivery` log value.
    pub(super) fn name(self) -> &'static str {
        match self {
            Delivery::Sent => "sent",
            Delivery::Returned => "returned",
            Delivery::CodPayment => "cod_payment",
            Delivery::ExpiredReturn => "expired_return",
            Delivery::System => "system",
        }
    }

    /// The feedback line. `from` is the header's sender name as stored:
    /// the sender, the returner, the COD payer, or (for an expiry return)
    /// the player the mail was sent to.
    pub(super) fn text(self, from: &str) -> String {
        match self {
            Delivery::Sent | Delivery::System => format!("You have new gate-mail from {from}."),
            Delivery::Returned => {
                format!("{from} returned your gate-mail. It is back in your mailbox.")
            }
            Delivery::CodPayment => {
                format!("{from} paid for your COD delivery. The payment is in your gate-mail.")
            }
            Delivery::ExpiredReturn => format!(
                "Your gate-mail to {from} expired unclaimed and was returned to your mailbox."
            ),
        }
    }
}

/// How one notification ended. Every outcome is logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NotifyOutcome {
    Notified,
    /// Not listed online: nothing sent.
    Offline,
    /// The mail is no longer the recipient's (taken, returned, deleted or
    /// quarantined before the push).
    MailGone,
    /// Listed, but the session had no player entity for that character at
    /// send time, or the header read failed.
    NotSent,
}

/// Tell `recipient` that `mail_id` is in their mailbox, if they are online.
/// Call it after the transaction that delivered the mail has committed:
/// the header is read back from the database, so a mail rolled back is
/// never announced.
pub(super) async fn notify_delivered(
    pool: &PgPool,
    ctx: &FeedbackCtx<'_>,
    recipient: i32,
    mail_id: i32,
    delivery: Delivery,
) -> NotifyOutcome {
    let online = {
        let clients = match ctx.connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        OnlinePlayerIndex::new(&clients)
            .find_player(recipient)
            .map(|p| (p.addr, clients.get(&p.addr).map(|c| c.account_id)))
    };
    let Some((addr, account_id)) = online else {
        skipped(recipient, None, mail_id, delivery, "offline");
        return NotifyOutcome::Offline;
    };

    let (headers, attachments) = match read_one(pool, recipient, mail_id).await {
        Ok(read) => read,
        Err(e) => {
            tracing::warn!(
                target: "mail",
                event = "mail.notify_failed",
                player_id = recipient,
                account_id,
                mail_id,
                delivery = delivery.name(),
                reason = "db_error",
                error = %e,
                "new-mail notification not sent: the header read failed",
            );
            return NotifyOutcome::NotSent;
        }
    };
    let Some(header) = headers.first() else {
        skipped(recipient, account_id, mail_id, delivery, "mail_gone");
        return NotifyOutcome::MailGone;
    };

    let text = delivery.text(&header.from_text);
    let line = serialize_on_player_communication(FEEDBACK_SPEAKER, 0, CHAN_FEEDBACK, &text);
    let (line_outcome, entity_id) = send_to_current_player(
        ctx,
        addr,
        recipient,
        method_idx::ON_PLAYER_COMMUNICATION,
        &line,
    )
    .await;
    if line_outcome != FeedbackOutcome::Sent {
        skipped(recipient, account_id, mail_id, delivery, "not_in_world");
        return NotifyOutcome::NotSent;
    }
    // One row, `ResetCategory` 0: an upsert that leaves the rest of the
    // client's list alone. New mail is never archived, so `bArchive` 0.
    let args = mail::serialize_on_mail_header_info(false, 0, &headers, &attachments);
    let (header_outcome, _) =
        send_to_current_player(ctx, addr, recipient, method_idx::ON_MAIL_HEADER_INFO, &args).await;
    tracing::debug!(
        target: "mail",
        event = "mail.notified",
        player_id = recipient,
        account_id,
        entity_id,
        mail_id,
        source_player_id = header.from_id,
        delivery = delivery.name(),
        header_pushed = header_outcome == FeedbackOutcome::Sent,
        "online recipient told of new gate-mail",
    );
    NotifyOutcome::Notified
}

fn skipped(
    recipient: i32,
    account_id: Option<u32>,
    mail_id: i32,
    delivery: Delivery,
    reason: &'static str,
) {
    tracing::debug!(
        target: "mail",
        event = "mail.notify_skipped",
        player_id = recipient,
        account_id,
        mail_id,
        delivery = delivery.name(),
        reason,
        "new-mail notification not sent",
    );
}
