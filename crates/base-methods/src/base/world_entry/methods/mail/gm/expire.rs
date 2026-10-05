//! `.mail_expire <mailId>` (SS-M4): make one mail due now and expire it at
//! once by the sweep's own path.

use sqlx::PgPool;

use super::super::claim::unix_now;
use super::super::expiry::{expire_and_tell, ExpiryPath, SweepSource, SweepSummary};
use super::super::notify::online_identity;
use super::super::Caller;
use super::feedback;
use crate::base::feedback::FeedbackCtx;
use crate::cell::mail::codes::flags::MAIL_ARCHIVE;
use crate::cell::messages::MailGmActor;

/// The GM's line for a `.mail_expire` that changed nothing.
fn expire_refusal(mail_id: i32, reason: &str) -> String {
    match reason {
        "archived" => {
            format!(".mail_expire: mail {mail_id} is archived, and archived mail never expires.")
        }
        "quarantined" => format!(
            ".mail_expire: mail {mail_id} is already quarantined; it keeps its attachments \
             for recovery."
        ),
        "mail_not_found" => format!(".mail_expire: no mail has id {mail_id}."),
        _ => format!(".mail_expire: mail {mail_id} was not changed (the database failed)."),
    }
}

/// `.mail_expire <mailId>` (SS-M4): make the mail due now, then expire it
/// at once by the sweep's own path, and tell the GM which path it took.
pub(super) async fn gm_expire(
    caller: &Caller<'_>,
    pool: &PgPool,
    actor: MailGmActor,
    mail_id: i32,
) {
    let now = unix_now();
    // Any mailbox (a GM tool), but only mail that can expire. One
    // conditional UPDATE: an archive racing it either commits first (and
    // the mail is refused here) or second (and archiving clears the expiry
    // again; the expiry below then skips it as archived).
    let made_due: Result<Option<i32>, sqlx::Error> = sqlx::query_scalar(
        "UPDATE sgw_gate_mail SET expires_at = $2 \
         WHERE mail_id = $1 AND NOT quarantined AND (flags & $3) = 0 \
         RETURNING character_id",
    )
    .bind(mail_id)
    .bind(now)
    .bind(MAIL_ARCHIVE)
    .fetch_optional(pool)
    .await;
    let owner = match made_due {
        Ok(Some(owner)) => owner,
        Ok(None) => {
            let state: Result<Option<(i32, bool)>, sqlx::Error> =
                sqlx::query_as("SELECT flags, quarantined FROM sgw_gate_mail WHERE mail_id = $1")
                    .bind(mail_id)
                    .fetch_optional(pool)
                    .await;
            let reason = match state {
                Ok(Some((_, true))) => "quarantined",
                Ok(Some((flags, false))) if flags & MAIL_ARCHIVE != 0 => "archived",
                Ok(_) => "mail_not_found",
                Err(_) => "db_error",
            };
            return refuse_expire(caller, actor, mail_id, reason, None).await;
        }
        Err(e) => return refuse_expire(caller, actor, mail_id, "db_error", Some(e)).await,
    };

    let fb = FeedbackCtx {
        transport: caller.transport,
        connected: caller.connected,
    };
    let mut summary = SweepSummary::default();
    let expired = expire_and_tell(
        pool,
        owner,
        mail_id,
        now,
        Some(&fb),
        SweepSource::Gm,
        &mut summary,
    )
    .await;
    let who = caller.identity();
    tracing::info!(
        target: "mail",
        event = "mail.gm_action",
        action = "mail_expire",
        entity_id = actor.entity_id,
        entity_name = who.player_name,
        account_id = actor.account_id,
        account_name = who.account_name,
        player_id = actor.player_id,
        player_name = who.player_name,
        subject_player_id = owner,
        subject_player_name = online_identity(caller.connected, owner).player_name,
        mail_id, // nt:id-only mail row, its subject is player text kept out of logs
        expires_at = now,
        path = expired.as_ref().map(|e| e.path.name()),
        "GM .mail_expire made a mail due",
    );
    let line = match expired.map(|e| e.path) {
        Some(ExpiryPath::Returned { to_player_id }) => format!(
            "Mail {mail_id} expired and was returned to its sender ({to_player_id}) with its \
             attachments."
        ),
        Some(ExpiryPath::Deleted) => {
            format!("Mail {mail_id} expired and was deleted (nothing was attached).")
        }
        Some(ExpiryPath::Quarantined { reason }) => format!(
            "Mail {mail_id} expired and was quarantined ({reason}); it keeps its attachments \
             for recovery."
        ),
        None => format!("Mail {mail_id} is now due; the expiry sweep takes it within 5 minutes."),
    };
    feedback(caller, &line).await;
}

/// Log a refused `.mail_expire` with its mail id (`mail.gm_rejected`, WARN,
/// ERROR for a database failure) and tell the GM why.
async fn refuse_expire(
    caller: &Caller<'_>,
    actor: MailGmActor,
    mail_id: i32,
    reason: &'static str,
    error: Option<sqlx::Error>,
) {
    let error = error.map(|e| e.to_string());
    let who = caller.identity();
    if reason == "db_error" {
        tracing::error!(
            target: "mail",
            event = "mail.gm_rejected",
            command = "mail_expire",
            reason,
            entity_id = actor.entity_id,
            entity_name = who.player_name,
            account_id = actor.account_id,
            account_name = who.account_name,
            player_id = actor.player_id,
            player_name = who.player_name,
            mail_id, // nt:id-only mail row, its subject is player text kept out of logs
            error,
            "GM .mail_expire failed",
        );
    } else {
        tracing::warn!(
            target: "mail",
            event = "mail.gm_rejected",
            command = "mail_expire",
            reason,
            entity_id = actor.entity_id,
            entity_name = who.player_name,
            account_id = actor.account_id,
            account_name = who.account_name,
            player_id = actor.player_id,
            player_name = who.player_name,
            mail_id, // nt:id-only mail row, its subject is player text kept out of logs
            "GM mail command refused",
        );
    }
    feedback(caller, &expire_refusal(mail_id, reason)).await;
}
