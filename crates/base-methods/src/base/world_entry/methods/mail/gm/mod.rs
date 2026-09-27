//! The base half of the GM mail tools (SS-U1): `.mail`, `.mailbox`
//! ([`mailbox`]) and `.mail_expire` ([`expire`], turned on by SS-M4).
//!
//! The cell's `.`-console gate has already checked the GM's server-side
//! access level and parsed the arguments; `MailGmCellToBase` carries the
//! GM's ids from the cell's own entity. Everything here reads the database
//! for the rest: the GM's stored name, the recipient (D-SS13 name rules,
//! online or not).
//!
//! - `.mail` without COD is a system mail ([`super::system`]) with the GM's
//!   name as its sender: cash and item minted, no postage, no `sender_id`,
//!   so it cannot be returned (D-SS10).
//! - `.mail ... cod <n>` is a mail **from the GM's character**
//!   (`sender_id` = the GM), because the COD payment is delivered to the
//!   sender (D-SS09). Its item is minted into escrow and no postage is
//!   charged; the header is the player COD header (`MAIL_COD`, the price in
//!   `cash`), written by the same [`super::system::write_mail`], so SS-M3's
//!   pay and take paths see an ordinary COD mail.
//!
//! Both skip the mailbox cap, like every server-written mail (D-SS03), and
//! an online recipient is told after the commit (D-SS11).
//!
//! - `.mail_expire <mailId>` makes one mail due now and runs its expiry at
//!   once through the sweep's own path ([`super::expiry`]), then tells the
//!   GM which D-SS04 path it took. Archived and quarantined mail are
//!   refused: archived mail never expires, and a quarantined one already
//!   took its path.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::{PgConnection, PgPool};

mod expire;
mod mailbox;

use expire::gm_expire;
use mailbox::gm_mailbox;

use super::notify::{notify_delivered, Delivery};
use super::send::recipients::{candidate_rows, resolve_names, Resolution};
use super::system::{
    self, send_system_mail_tx, MailHeader, SystemItem, SystemMail, SystemMailError,
};
use super::Caller;
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::base::ConnectedClientState;
use crate::cell::mail::codes::flags::MAIL_COD;
use crate::cell::messages::{MailGmActor, MailGmCellToBase};

/// Route one GM mail command from the cell.
#[tracing::instrument(
    name = "mail.gm",
    level = "info",
    skip_all,
    fields(entity_id = msg.actor().entity_id, player_id = msg.actor().player_id, kind = msg.kind()),
)]
pub async fn handle_mail_gm(
    msg: MailGmCellToBase,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    let actor = msg.actor();
    let caller = Caller {
        entity_id: actor.entity_id,
        player_id: actor.player_id,
        transport,
        connected,
        entity_to_addr,
    };
    let Some(pool) = db_pool.as_deref() else {
        rejected(actor, msg.kind(), "no_db_pool");
        feedback(&caller, "Mail is unavailable: the server has no database.").await;
        return;
    };
    match msg {
        MailGmCellToBase::Send {
            actor,
            to,
            cash,
            item,
            cod,
            subject,
        } => {
            let request = GmSend {
                to,
                cash,
                item,
                cod,
                subject,
            };
            gm_send(&caller, pool, actor, &request).await;
        }
        MailGmCellToBase::Mailbox { actor, name } => {
            gm_mailbox(&caller, pool, actor, name.as_deref()).await;
        }
        MailGmCellToBase::Expire { actor, mail_id } => {
            gm_expire(&caller, pool, actor, mail_id).await;
        }
    }
}

/// `.mail`'s arguments, as the cell parsed them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GmSend {
    pub(super) to: Option<String>,
    pub(super) cash: i64,
    pub(super) item: Option<(i32, i32)>,
    pub(super) cod: Option<i32>,
    pub(super) subject: String,
}

/// Why a GM command stopped before writing anything.
#[derive(Debug)]
enum GmRefusal {
    /// The GM's own `sgw_player` row is gone.
    GmMissing,
    /// The recipient name did not resolve (`unknown_recipient` or
    /// `ambiguous_recipient`).
    Recipient(&'static str, String),
    /// COD without an item, or with gift cash (the cell refuses both first).
    BadCod(&'static str),
    Mail(SystemMailError),
}

impl From<sqlx::Error> for GmRefusal {
    fn from(e: sqlx::Error) -> Self {
        GmRefusal::Mail(SystemMailError::Db(e))
    }
}

impl From<SystemMailError> for GmRefusal {
    fn from(e: SystemMailError) -> Self {
        GmRefusal::Mail(e)
    }
}

impl GmRefusal {
    /// The underlying error, for the log.
    fn detail(&self) -> String {
        match self {
            GmRefusal::Mail(e) => e.to_string(),
            other => other.reason().to_string(),
        }
    }

    fn reason(&self) -> &'static str {
        match self {
            GmRefusal::GmMissing => "gm_missing",
            GmRefusal::Recipient(reason, _) => reason,
            GmRefusal::BadCod(reason) => reason,
            GmRefusal::Mail(e) => e.reason(),
        }
    }

    fn text(&self) -> String {
        match self {
            GmRefusal::GmMissing => ".mail: your character was not found.".to_string(),
            GmRefusal::Recipient("ambiguous_recipient", name) => {
                format!(".mail: more than one character is called {name}; type the exact name.")
            }
            GmRefusal::Recipient(_, name) => format!(".mail: no character is called {name}."),
            GmRefusal::BadCod(_) => ".mail: cod needs an item and a price of 1 or more, \
                                     and cannot be combined with cash."
                .to_string(),
            GmRefusal::Mail(e) => format!(".mail: not sent ({e})."),
        }
    }
}

/// What a committed `.mail` wrote.
struct GmSent {
    recipient_id: i32,
    recipient_name: String,
    mail_id: i32,
    escrow_item_id: Option<i32>,
    system: Option<system::SystemMailSent>,
}

/// `.mail`.
pub(super) async fn gm_send(
    caller: &Caller<'_>,
    pool: &PgPool,
    actor: MailGmActor,
    request: &GmSend,
) {
    match write_gm_send(pool, actor, request).await {
        Ok(sent) => {
            if let Some(system) = &sent.system {
                system.log_sent();
            }
            tracing::info!(
                target: "mail",
                event = "mail.gm_action",
                action = "mail",
                // The cash and the item were created, not moved from anyone:
                // a currency-flow query tells GM minting from player mail.
                minted = true,
                entity_id = actor.entity_id,
                account_id = actor.account_id,
                player_id = actor.player_id,
                subject_player_id = sent.recipient_id,
                mail_id = sent.mail_id,
                cash = request.cash,
                cod = request.cod,
                type_id = request.item.map(|(t, _)| t),
                quantity = request.item.map(|(_, q)| q),
                escrow_item_id = sent.escrow_item_id,
                "GM .mail sent",
            );
            let fb = FeedbackCtx {
                transport: caller.transport,
                connected: caller.connected,
            };
            let delivery = if request.cod.is_some() {
                Delivery::Sent
            } else {
                Delivery::System
            };
            notify_delivered(pool, &fb, sent.recipient_id, sent.mail_id, delivery).await;
            let mut parts = Vec::new();
            if request.cash > 0 {
                parts.push(format!("{} naquadah", request.cash));
            }
            if let Some((type_id, qty)) = request.item {
                parts.push(format!("{qty} x item {type_id}"));
            }
            if let Some(price) = request.cod {
                parts.push(format!("COD {price}"));
            }
            let attached = if parts.is_empty() {
                String::new()
            } else {
                format!(" with {}", parts.join(", "))
            };
            feedback(
                caller,
                &format!(
                    "Mail {} sent to {}{attached}.",
                    sent.mail_id, sent.recipient_name
                ),
            )
            .await;
        }
        Err(refusal) => {
            // Everything the GM asked for, so a refused mint can be read
            // back from SigNoz alone.
            tracing::warn!(
                target: "mail",
                event = "mail.gm_rejected",
                command = "send",
                reason = refusal.reason(),
                entity_id = actor.entity_id,
                account_id = actor.account_id,
                player_id = actor.player_id,
                to = request.to.as_deref(),
                cash = request.cash,
                cod = request.cod,
                type_id = request.item.map(|(t, _)| t),
                quantity = request.item.map(|(_, q)| q),
                error = %refusal.detail(),
                "GM mail command refused",
            );
            feedback(caller, &refusal.text()).await;
        }
    }
}

/// The transaction behind `.mail`.
async fn write_gm_send(
    pool: &PgPool,
    actor: MailGmActor,
    request: &GmSend,
) -> Result<GmSent, GmRefusal> {
    let item = match request.item {
        Some((type_id, qty)) => SystemItem::Minted { type_id, qty },
        None => SystemItem::None,
    };
    // The cell checks these too; the base re-checks, because a COD mail
    // with no item or no price could never be paid or taken.
    if let Some(price) = request.cod {
        let bad = if request.item.is_none() {
            Some("cod_without_item")
        } else if request.cash != 0 {
            Some("cod_with_cash")
        } else if price < 1 {
            Some("cod_price_invalid")
        } else {
            None
        };
        if let Some(reason) = bad {
            return Err(GmRefusal::BadCod(reason));
        }
    }

    let mut tx = pool.begin().await?;
    let gm_name: Option<String> =
        sqlx::query_scalar("SELECT player_name FROM sgw_player WHERE player_id = $1")
            .bind(actor.player_id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some(gm_name) = gm_name else {
        return Err(GmRefusal::GmMissing);
    };
    let (recipient_id, recipient_name) = match &request.to {
        None => (actor.player_id, gm_name.clone()),
        Some(name) => resolve_one(&mut tx, name).await?,
    };
    let mail = SystemMail {
        sender_name: gm_name,
        recipient_player_id: recipient_id,
        subject: request.subject.clone(),
        body: "Sent by a GM with the .mail command.".to_string(),
        cash: request.cash,
        item,
    };

    let sent = match request.cod {
        None => {
            let sent = send_system_mail_tx(&mut tx, &mail).await?;
            GmSent {
                recipient_id,
                recipient_name,
                mail_id: sent.mail_id,
                escrow_item_id: sent.item.map(|i| i.item_id),
                system: Some(sent),
            }
        }
        Some(price) => {
            system::validate(&mail)?;
            let header = MailHeader {
                recipient_player_id: recipient_id,
                sender_id: Some(actor.player_id),
                sender_name: &mail.sender_name,
                subject: &mail.subject,
                body: &mail.body,
                cash: i64::from(price),
                flags: MAIL_COD,
            };
            let written = system::write_mail(&mut tx, &header, item, system::now_secs()).await?;
            GmSent {
                recipient_id,
                recipient_name,
                mail_id: written.mail_id,
                escrow_item_id: written.item.map(|i| i.item_id),
                system: None,
            }
        }
    };
    tx.commit().await?;
    Ok(sent)
}

/// One typed name to `(player_id, stored name)`, by the D-SS13 rules the
/// player send path uses.
async fn resolve_one(conn: &mut PgConnection, name: &str) -> Result<(i32, String), GmRefusal> {
    let typed = vec![name.to_string()];
    let rows = candidate_rows(&mut *conn, &typed).await?;
    match resolve_names(&typed, &rows).pop() {
        Some(Resolution::Found { player_id }) => {
            let stored = rows
                .iter()
                .find(|(id, _)| *id == player_id)
                .map(|(_, n)| n.clone())
                .unwrap_or_else(|| name.to_string());
            Ok((player_id, stored))
        }
        Some(Resolution::Failed(reason)) => Err(GmRefusal::Recipient(reason.reason(), name.into())),
        None => Err(GmRefusal::Recipient("unknown_recipient", name.into())),
    }
}

/// Log a refused GM mail command.
fn rejected(actor: MailGmActor, kind: &'static str, reason: &'static str) {
    tracing::warn!(
        target: "mail",
        event = "mail.gm_rejected",
        command = kind,
        reason,
        entity_id = actor.entity_id,
        account_id = actor.account_id,
        player_id = actor.player_id,
        "GM mail command refused",
    );
}

async fn feedback(caller: &Caller<'_>, text: &str) {
    let Some(addr) = caller.addr() else {
        tracing::debug!(
            target: "mail",
            entity_id = caller.entity_id,
            player_id = caller.player_id,
            reason = "no_session",
            "GM mail feedback dropped: the GM has no client address",
        );
        return;
    };
    let ctx = FeedbackCtx {
        transport: caller.transport,
        connected: caller.connected,
    };
    send_feedback_line(&ctx, addr, text).await;
}
