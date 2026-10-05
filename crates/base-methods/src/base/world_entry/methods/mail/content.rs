//! The base half of the content engine's `send_system_mail` action (SS-U3):
//! the Gate Mail Clerk in the stasis-room debug hub, and any later chain
//! that gives a player a mail.
//!
//! One transaction, in the system writer's lock order:
//!
//! 1. the recipient's `sgw_player` row, `FOR UPDATE` (a missing character
//!    is `RecipientNotFound`, as the writer would say);
//! 2. the cooldown claim, if the action has one: one conditional upsert on
//!    `sgw_player_content_cooldown` that succeeds only when the last claim
//!    is at least `secs` old. A second firing inside the window matches no
//!    row, writes nothing, and the player is told how long to wait;
//! 3. the mail, through [`send_system_mail_tx`].
//!
//! The claim and the mail commit together, so a refused mail keeps the
//! previous claim, and a claim never outlives a rolled-back mail. The
//! cooldown lives in its own table, not in `sgw_gate_mail`: a player may
//! delete the mail after taking its attachments, and the window must hold
//! anyway.
//!
//! Every firing ends with one feedback line to the player: the mail id and
//! what it carries, the time left on the cooldown, or why nothing was sent.
//! A sent mail is then announced like every other delivery (D-SS11, SS-M4):
//! `SystemMailSent::notify` pushes the header, so an open mailbox shows it
//! at once, with the new-mail line, after the commit.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::system::{
    now_secs, send_system_mail_tx, SystemItem, SystemMail, SystemMailError, SystemMailSent,
};
use super::Caller;
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::base::ConnectedClientState;
use crate::cell::messages::{ContentMailCooldown, ContentSystemMail};

/// Claim `key` for `player_id` at `now` unless the last claim is younger
/// than `secs`. `rows_affected` is 1 on a claim (insert or update) and 0
/// inside the window.
const CLAIM_SQL: &str = "INSERT INTO sgw_player_content_cooldown \
         (player_id, cooldown_key, last_used_at) VALUES ($1, $2, $3) \
     ON CONFLICT (player_id, cooldown_key) DO UPDATE SET last_used_at = EXCLUDED.last_used_at \
     WHERE sgw_player_content_cooldown.last_used_at <= EXCLUDED.last_used_at - $4";

/// Why a content mail was not sent.
#[derive(Debug)]
pub(super) enum ContentRefusal {
    /// The server runs without a database.
    NoPool,
    /// The cooldown was claimed `secs - remaining_secs` seconds ago.
    Cooldown {
        last_used_at: i32,
        remaining_secs: i64,
    },
    Mail(SystemMailError),
}

impl From<sqlx::Error> for ContentRefusal {
    fn from(e: sqlx::Error) -> Self {
        ContentRefusal::Mail(SystemMailError::Db(e))
    }
}

impl From<SystemMailError> for ContentRefusal {
    fn from(e: SystemMailError) -> Self {
        ContentRefusal::Mail(e)
    }
}

impl ContentRefusal {
    pub(super) fn reason(&self) -> &'static str {
        match self {
            ContentRefusal::NoPool => "no_db_pool",
            ContentRefusal::Cooldown { .. } => "cooldown",
            ContentRefusal::Mail(e) => e.reason(),
        }
    }
}

/// What a committed content mail wrote, plus the item's display name for
/// the feedback line.
#[derive(Debug)]
pub(super) struct ContentSent {
    pub(super) sent: SystemMailSent,
    pub(super) item_name: Option<String>,
}

/// Route one `send_system_mail` firing from the cell.
#[tracing::instrument(
    name = "mail.content",
    level = "info",
    skip_all,
    fields(entity_id = msg.entity_id, player_id = msg.player_id, chain_id = msg.chain_id),
)]
pub async fn handle_content_system_mail(
    msg: ContentSystemMail,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    let caller = Caller {
        entity_id: msg.entity_id,
        player_id: msg.player_id,
        transport,
        connected,
        entity_to_addr,
    };
    let account_id = msg.account_id.or_else(|| caller.account_id());
    let who = caller.identity();
    let result = match db_pool.as_deref() {
        Some(pool) => write_content_mail(pool, &msg, i64::from(now_secs())).await,
        None => Err(ContentRefusal::NoPool),
    };
    let cooldown_key = msg.cooldown.as_ref().map(|c| c.key.as_str());
    match result {
        Ok(done) => {
            done.sent.log_sent();
            let item = done.sent.item;
            tracing::info!(
                target: "content",
                event = "content.send_system_mail",
                outcome = "sent",
                entity_id = msg.entity_id,
                entity_name = who.player_name,
                account_id,
                account_name = who.account_name,
                player_id = msg.player_id,
                player_name = who.player_name,
                chain_id = msg.chain_id, // nt:id-only content chain row, the chain has no player-facing name
                mail_id = done.sent.mail_id, // nt:id-only mail row, its subject is player text kept out of logs
                sender_name = %msg.sender_name,
                cash = msg.cash,
                item_id = item.map(|i| i.item_id),
                item_type_id = item.map(|i| i.type_id),
                item_name = done.item_name.as_deref(),
                stack_size = item.map(|i| i.stack_size),
                cooldown_key,
                cooldown_secs = msg.cooldown.as_ref().map(|c| c.secs),
                "content system mail sent",
            );
            feedback(&caller, &sent_line(&msg, &done)).await;
            if let Some(pool) = db_pool.as_deref() {
                let fb = FeedbackCtx {
                    transport,
                    connected,
                };
                done.sent.notify(pool, &fb).await;
            }
        }
        Err(refusal) => {
            let (last_used_at, remaining_secs) = match &refusal {
                ContentRefusal::Cooldown {
                    last_used_at,
                    remaining_secs,
                } => (Some(*last_used_at), Some(*remaining_secs)),
                _ => (None, None),
            };
            let error = match &refusal {
                ContentRefusal::Mail(e) => Some(e.to_string()),
                _ => None,
            };
            let book = cimmeria_names::book();
            tracing::warn!(
                target: "content",
                event = "content.send_system_mail",
                reason = refusal.reason(),
                entity_id = msg.entity_id,
                entity_name = who.player_name,
                account_id,
                account_name = who.account_name,
                player_id = msg.player_id,
                player_name = who.player_name,
                chain_id = msg.chain_id, // nt:id-only content chain row, the chain has no player-facing name
                sender_name = %msg.sender_name,
                cash = msg.cash,
                item_type_id = msg.item.map(|(t, _)| t),
                item_name = msg
                    .item
                    .and_then(|(t, _)| book.item(t)),
                quantity = msg.item.map(|(_, q)| q),
                cooldown_key,
                last_used_at,
                remaining_secs,
                error,
                "content system mail refused",
            );
            feedback(&caller, &refused_line(&msg.sender_name, &refusal)).await;
        }
    }
}

/// The transaction behind one firing, on an explicit clock so the cooldown
/// can be stepped in tests.
pub(super) async fn write_content_mail(
    pool: &PgPool,
    msg: &ContentSystemMail,
    now: i64,
) -> Result<ContentSent, ContentRefusal> {
    let mut tx = pool.begin().await?;
    let exists: Option<i32> =
        sqlx::query_scalar("SELECT player_id FROM sgw_player WHERE player_id = $1 FOR UPDATE")
            .bind(msg.player_id)
            .fetch_optional(&mut *tx)
            .await?;
    if exists.is_none() {
        return Err(SystemMailError::RecipientNotFound.into());
    }
    if let Some(cooldown) = &msg.cooldown {
        claim_cooldown(&mut tx, msg.player_id, cooldown, now).await?;
    }
    let mail = SystemMail {
        sender_name: msg.sender_name.clone(),
        recipient_player_id: msg.player_id,
        subject: msg.subject.clone(),
        body: msg.body.clone(),
        cash: msg.cash,
        item: match msg.item {
            Some((type_id, qty)) => SystemItem::Minted { type_id, qty },
            None => SystemItem::None,
        },
    };
    let sent = send_system_mail_tx(&mut tx, &mail).await?;
    let item_name = match sent.item {
        Some(escrow) => {
            sqlx::query_scalar("SELECT name FROM resources.items WHERE item_id = $1")
                .bind(escrow.type_id)
                .fetch_optional(&mut *tx)
                .await?
        }
        None => None,
    };
    tx.commit().await?;
    Ok(ContentSent { sent, item_name })
}

/// The conditional upsert. On a miss, reads the standing claim for the
/// wait time; the row is there, because the upsert found a conflict.
async fn claim_cooldown(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    player_id: i32,
    cooldown: &ContentMailCooldown,
    now: i64,
) -> Result<(), ContentRefusal> {
    let now32 = i32::try_from(now).unwrap_or(i32::MAX);
    let claimed = sqlx::query(CLAIM_SQL)
        .bind(player_id)
        .bind(&cooldown.key)
        .bind(now32)
        .bind(i32::try_from(cooldown.secs).unwrap_or(i32::MAX))
        .execute(&mut **tx)
        .await?
        .rows_affected();
    if claimed == 1 {
        return Ok(());
    }
    let last_used_at: i32 = sqlx::query_scalar(
        "SELECT last_used_at FROM sgw_player_content_cooldown \
         WHERE player_id = $1 AND cooldown_key = $2",
    )
    .bind(player_id)
    .bind(&cooldown.key)
    .fetch_one(&mut **tx)
    .await?;
    let remaining_secs = (i64::from(last_used_at) + i64::from(cooldown.secs) - now).max(1);
    Err(ContentRefusal::Cooldown {
        last_used_at,
        remaining_secs,
    })
}

/// "Gate Mail Clerk sent you mail 12 with 50 naquadah and 5 x Health
/// Slappack TC1. Open your mail to take them."
pub(super) fn sent_line(msg: &ContentSystemMail, done: &ContentSent) -> String {
    let mut parts = Vec::new();
    if msg.cash > 0 {
        parts.push(format!("{} naquadah", msg.cash));
    }
    if let Some(escrow) = done.sent.item {
        let name = done
            .item_name
            .clone()
            .unwrap_or_else(|| format!("item {}", escrow.type_id));
        parts.push(format!("{} x {name}", escrow.stack_size));
    }
    if parts.is_empty() {
        format!(
            "{} sent you mail {}. Open your mail to read it.",
            msg.sender_name, done.sent.mail_id
        )
    } else {
        format!(
            "{} sent you mail {} with {}. Open your mail to take them.",
            msg.sender_name,
            done.sent.mail_id,
            parts.join(" and ")
        )
    }
}

/// The refusal line: the wait for a cooldown, the writer's reason otherwise.
pub(super) fn refused_line(sender_name: &str, refusal: &ContentRefusal) -> String {
    match refusal {
        ContentRefusal::Cooldown { remaining_secs, .. } => format!(
            "{sender_name} has already sent you mail. You can ask again in {}.",
            wait_text(*remaining_secs)
        ),
        ContentRefusal::NoPool | ContentRefusal::Mail(SystemMailError::Db(_)) => {
            format!("{sender_name} could not send your mail: the mail service is unavailable.")
        }
        ContentRefusal::Mail(e) => format!("{sender_name} could not send your mail ({e})."),
    }
}

/// "7 minutes", "1 minute", "45 seconds": minutes rounded up, so the player
/// is never told a time that is too short.
pub(super) fn wait_text(secs: i64) -> String {
    let secs = secs.max(1);
    if secs < 60 {
        return format!("{secs} second{}", if secs == 1 { "" } else { "s" });
    }
    let minutes = (secs + 59) / 60;
    format!("{minutes} minute{}", if minutes == 1 { "" } else { "s" })
}

async fn feedback(caller: &Caller<'_>, text: &str) {
    let Some(addr) = caller.addr() else {
        // `mail`, not `content`: the content target ships at INFO, and a
        // player who logged out before the answer is not worth a WARN.
        let who = caller.identity();
        tracing::debug!(
            target: "mail",
            entity_id = caller.entity_id,
            entity_name = who.player_name,
            player_id = caller.player_id,
            player_name = who.player_name,
            reason = "no_session",
            "content mail feedback dropped: the player has no client address",
        );
        return;
    };
    let ctx = FeedbackCtx {
        transport: caller.transport,
        connected: caller.connected,
    };
    send_feedback_line(&ctx, addr, text).await;
}
