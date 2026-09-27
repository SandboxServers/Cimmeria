//! Mail expiry (SS-M4, D-SS04): the 30-day TTL, the base sweep task, the
//! login-time sweep of one mailbox, and what each expired mail tells its
//! players.
//!
//! **TTL.** Every writer sets `expires_at = sent_time + MAIL_TTL_SECS` at
//! insert (player send, the COD payment mail, server mail, the GM COD
//! mail), a return resets it with the fresh `sent_time`, and archiving
//! clears it (archived mail never expires). 720 hours is the client's own
//! constant: `MessageHeader` has no expiry field, and the header-record
//! constructor computes `ExpiresHours = 0x2d0 - hours` (SS-E1 M-Q3,
//! `SGW.exe@0x00eb5ab0`), so the Expires column counts down 30 days and the
//! server takes the mail when it reaches zero. The constant is HIGH
//! confidence; that the countdown starts at the wire `sentTime` is MEDIUM
//! (the decompile does not show `sentTime` reaching the time call), so a
//! capture could still move the anchor, never the length.
//!
//! **Sweeps.** [`spawn_sweeper`] runs [`sweep_due`] every
//! [`SWEEP_INTERVAL`] over every mailbox, [`SWEEP_BATCH`] mails per query,
//! oldest expiry first. [`sweep_mailbox`] does the same for one player's
//! mailbox at world entry. Both take `now` from the caller (an injected
//! clock), and both expire each mail in a transaction of its own
//! ([`terminal::expire_one`]), so one failure never holds another mail.
//!
//! **After each commit**, never inside the transaction: the mail's old owner,
//! if online, gets `onMailHeaderRemove` (their list must not keep a mail
//! that is gone), and a returned mail's new owner gets the new-mail
//! notification (D-SS11).

mod terminal;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::claim::{unix_now, OpError};
use super::notify::{notify_delivered, Delivery};
use crate::base::feedback::{send_to_current_player, FeedbackCtx};
use crate::base::player_index::OnlinePlayerIndex;
use crate::base::ConnectedClientState;
use crate::cell::mail;
use crate::cell::mail::codes::flags::MAIL_ARCHIVE;
use crate::mercury::method_idx;

pub(super) use terminal::{expire_one, ExpireOutcome, Expired, ExpiryPath};

/// A mail's life before the sweep takes it: 720 hours, the client's own
/// Expires constant (`0x2d0`, SS-E1 M-Q3).
pub const MAIL_TTL_SECS: i32 = 720 * 3600;

/// How often the base sweep runs. The client shows expiry in whole hours
/// ("Soon" under 2), so five minutes late is invisible.
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(300);

/// Mails read per sweep query.
pub const SWEEP_BATCH: i64 = 100;

/// Most mails one sweep run expires. A larger backlog is finished on the
/// next runs, so one run never holds the task for long.
pub const SWEEP_MAX_PER_RUN: usize = 1_000;

/// `expires_at` for a mail written or returned at `sent_time`.
pub(super) fn expires_at(sent_time: i32) -> i32 {
    sent_time.saturating_add(MAIL_TTL_SECS)
}

/// What started a sweep, for the logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepSource {
    /// The periodic base task.
    Tick,
    /// A player's world entry, for their own mailbox.
    Login,
    /// A GM's `.mail_expire`.
    Gm,
}

impl SweepSource {
    fn name(self) -> &'static str {
        match self {
            SweepSource::Tick => "tick",
            SweepSource::Login => "login",
            SweepSource::Gm => "gm",
        }
    }
}

/// One sweep run's counts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SweepSummary {
    pub scanned: usize,
    pub returned: usize,
    pub deleted: usize,
    pub quarantined: usize,
    pub skipped: usize,
    pub failed: usize,
}

/// Where a sweep tells players what happened. `None` in a sweep with no
/// sessions to reach (tests of the SQL alone).
pub type SweepNotify<'a> = Option<&'a FeedbackCtx<'a>>;

/// Expire every due mail, in every mailbox, at `now`: at most
/// [`SWEEP_MAX_PER_RUN`], oldest expiry first.
pub async fn sweep_due(pool: &PgPool, now: i32, notify: SweepNotify<'_>) -> SweepSummary {
    let mut summary = SweepSummary::default();
    // Keyset pagination: a mail skipped or failed in this run is not read
    // again by it.
    let mut after = (i32::MIN, i32::MIN);
    while summary.scanned < SWEEP_MAX_PER_RUN {
        let batch: Vec<(i32, i32, i32)> = match sqlx::query_as(
            "SELECT expires_at, mail_id, character_id FROM sgw_gate_mail \
             WHERE expires_at <= $1 AND NOT quarantined AND (flags & $2) = 0 \
               AND (expires_at, mail_id) > ($3, $4) \
             ORDER BY expires_at, mail_id LIMIT $5",
        )
        .bind(now)
        .bind(MAIL_ARCHIVE)
        .bind(after.0)
        .bind(after.1)
        .bind(SWEEP_BATCH)
        .fetch_all(pool)
        .await
        {
            Ok(batch) => batch,
            Err(e) => {
                scan_failed(SweepSource::Tick, None, &e);
                summary.failed += 1;
                break;
            }
        };
        let full = batch.len() as i64 == SWEEP_BATCH;
        for (at, mail_id, owner) in batch {
            after = (at, mail_id);
            expire_and_tell(
                pool,
                owner,
                mail_id,
                now,
                notify,
                SweepSource::Tick,
                &mut summary,
            )
            .await;
        }
        if !full {
            break;
        }
    }
    log_summary(SweepSource::Tick, None, now, summary);
    summary
}

/// Expire every due mail in `owner`'s mailbox at `now`: the login-time
/// sweep, so a player who was away never opens a mailbox showing mail the
/// sweep has not reached yet.
pub async fn sweep_mailbox(
    pool: &PgPool,
    owner: i32,
    now: i32,
    notify: SweepNotify<'_>,
) -> SweepSummary {
    let mut summary = SweepSummary::default();
    let due: Vec<i32> = match sqlx::query_scalar(
        "SELECT mail_id FROM sgw_gate_mail \
         WHERE character_id = $1 AND expires_at <= $2 AND NOT quarantined AND (flags & $3) = 0 \
         ORDER BY expires_at, mail_id LIMIT $4",
    )
    .bind(owner)
    .bind(now)
    .bind(MAIL_ARCHIVE)
    .bind(SWEEP_MAX_PER_RUN as i64)
    .fetch_all(pool)
    .await
    {
        Ok(due) => due,
        Err(e) => {
            scan_failed(SweepSource::Login, Some(owner), &e);
            summary.failed += 1;
            return summary;
        }
    };
    for mail_id in due {
        expire_and_tell(
            pool,
            owner,
            mail_id,
            now,
            notify,
            SweepSource::Login,
            &mut summary,
        )
        .await;
    }
    log_summary(SweepSource::Login, Some(owner), now, summary);
    summary
}

/// Expire one mail (the GM's `.mail_expire`, or a sweep's row), log it, tell
/// its players after the commit, and count it.
pub(super) async fn expire_and_tell(
    pool: &PgPool,
    owner: i32,
    mail_id: i32,
    now: i32,
    notify: SweepNotify<'_>,
    source: SweepSource,
    summary: &mut SweepSummary,
) -> Option<Expired> {
    summary.scanned += 1;
    match expire_one(pool, owner, mail_id, now).await {
        Ok(ExpireOutcome::Expired(expired)) => {
            match expired.path {
                ExpiryPath::Returned { .. } => summary.returned += 1,
                ExpiryPath::Deleted => summary.deleted += 1,
                ExpiryPath::Quarantined { .. } => summary.quarantined += 1,
            }
            log_expired(&expired, source, now);
            if let Some(ctx) = notify {
                tell(pool, ctx, &expired).await;
            }
            Some(expired)
        }
        Ok(ExpireOutcome::Skipped(reason)) => {
            summary.skipped += 1;
            tracing::debug!(
                target: "mail",
                event = "mail.expire_skipped",
                player_id = owner,
                mail_id,
                reason,
                source = source.name(),
                "expiry sweep left a mail unchanged",
            );
            None
        }
        Err(err) => {
            summary.failed += 1;
            let (reason, error) = match &err {
                OpError::Invariant(reason) => (*reason, None),
                OpError::Db(e) => ("db_error", Some(e.to_string())),
                OpError::Refused(r) => (r.reason, None),
            };
            tracing::error!(
                target: "mail",
                event = "mail.expire_failed",
                player_id = owner,
                mail_id,
                reason,
                error,
                source = source.name(),
                "mail expiry rolled back; the next sweep retries it",
            );
            None
        }
    }
}

/// The old owner loses the header; a returned mail's new owner is told.
async fn tell(pool: &PgPool, ctx: &FeedbackCtx<'_>, expired: &Expired) {
    let online = {
        let clients = match ctx.connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        OnlinePlayerIndex::new(&clients).find_player(expired.owner)
    };
    if let Some(owner) = online {
        send_to_current_player(
            ctx,
            owner.addr,
            expired.owner,
            method_idx::ON_MAIL_HEADER_REMOVE,
            &mail::serialize_on_mail_header_remove(expired.mail_id),
        )
        .await;
    }
    if let ExpiryPath::Returned { to_player_id } = expired.path {
        notify_delivered(
            pool,
            ctx,
            to_player_id,
            expired.mail_id,
            Delivery::ExpiredReturn,
        )
        .await;
    }
}

/// `mail.expired`: INFO for a return or a delete, WARN for a quarantine
/// (D-SS04: a quarantined mail needs a GM). `player_id` is the mailbox the
/// mail expired from, `target_player_id` its sender.
fn log_expired(e: &Expired, source: SweepSource, now: i32) {
    let returned_to = match e.path {
        ExpiryPath::Returned { to_player_id } => Some(to_player_id),
        _ => None,
    };
    macro_rules! expired_event {
        ($level:ident, $reason:expr, $msg:literal) => {
            tracing::$level!(
                target: "mail",
                event = "mail.expired",
                path = e.path.name(),
                reason = $reason,
                player_id = e.owner,
                target_player_id = e.sender_id,
                returned_to,
                mail_id = e.mail_id,
                item_id = e.item_id,
                cash = e.cash,
                cod_cancelled = e.cod_cancelled,
                expires_at = e.expires_at,
                now,
                source = source.name(),
                $msg,
            )
        };
    }
    match e.path {
        ExpiryPath::Quarantined { reason } => expired_event!(
            warn,
            reason,
            "expired gate-mail quarantined with its attachments; a GM must recover it"
        ),
        ExpiryPath::Returned { .. } => expired_event!(
            info,
            "expired",
            "expired gate-mail returned to its sender with its attachments"
        ),
        ExpiryPath::Deleted => expired_event!(
            info,
            "expired",
            "expired gate-mail with nothing attached deleted"
        ),
    }
}

fn log_summary(source: SweepSource, owner: Option<i32>, now: i32, s: SweepSummary) {
    tracing::debug!(
        target: "mail",
        event = "mail.expiry_sweep",
        source = source.name(),
        player_id = owner,
        now,
        scanned = s.scanned,
        returned = s.returned,
        deleted = s.deleted,
        quarantined = s.quarantined,
        skipped = s.skipped,
        failed = s.failed,
        "mail expiry sweep finished",
    );
}

fn scan_failed(source: SweepSource, owner: Option<i32>, e: &sqlx::Error) {
    tracing::error!(
        target: "mail",
        event = "mail.expire_failed",
        source = source.name(),
        player_id = owner,
        reason = "scan_db_error",
        error = %e,
        "mail expiry scan failed; the next sweep retries",
    );
}

/// The login-time sweep: [`sweep_mailbox`] for `player_id`'s own mailbox,
/// spawned so nothing on the world-entry path waits for it. Called on every
/// `onClientReady`, so a gate travel runs it too (cheap: an index scan of
/// one mailbox).
pub fn spawn_login_sweep(
    pool: Arc<PgPool>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    player_id: i32,
) {
    tokio::spawn(async move {
        let ctx = FeedbackCtx {
            transport: &transport,
            connected: &connected,
        };
        sweep_mailbox(&pool, player_id, unix_now(), Some(&ctx)).await;
    });
}

/// Start the base's expiry task: [`sweep_due`] every [`SWEEP_INTERVAL`],
/// the first run one interval after startup (the login sweep covers a
/// player who enters before then).
pub fn spawn_sweeper(
    pool: Arc<PgPool>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
        // The first tick of `interval` fires at once; skip it.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let ctx = FeedbackCtx {
                transport: &transport,
                connected: &connected,
            };
            sweep_due(&pool, unix_now(), Some(&ctx)).await;
        }
    });
}
