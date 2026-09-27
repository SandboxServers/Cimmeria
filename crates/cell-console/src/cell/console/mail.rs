//! GM mail console commands (SS-U1): `.mail`, `.mailbox` and `.mail_expire`,
//! tools for testing gate mail without a second player.
//!
//! The cell parses and range-checks; the base does the rest
//! (`base-methods` `mail/gm.rs`), because every mail write is a base
//! transaction. None has a legacy counterpart; the wording is plain
//! English (the owner's preference for GM commands).
//!
//! - `.mail [to <name>] [cash <n>] [item <typeId> [qty]] [cod <n>] [<subject>]`
//!   mails the GM (or `<name>`, online or not) cash and an item, both
//!   minted, with no postage. The options come first, in any order; the
//!   first word that is not an option starts the subject, and a number right
//!   after `item <typeId>` is the quantity. `cod <n>` makes it a COD mail
//!   from the GM's character, so the payment comes back to the GM; it needs
//!   an item and no cash.
//! - `.mailbox [name]` reports a mailbox: open and archived counts, what is
//!   in escrow, and the next expiry.
//! - `.mail_expire <mailId>` makes one mail due now; the base expires it at
//!   once by the sweep's own path (SS-M4) and tells the GM which D-SS04
//!   path it took.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::{CellToBaseMsg, MailGmActor, MailGmCellToBase};
use crate::cell::space_manager::SpaceManager;

/// The subject when `.mail` is given none.
pub(crate) const DEFAULT_SUBJECT: &str = "GM test mail";

pub(crate) const MAIL_USAGE: &str =
    "Usage: .mail [to <name>] [cash <n>] [item <typeId> [qty]] [cod <n>] [<subject>]";
pub(crate) const MAIL_EXPIRE_USAGE: &str =
    ".mail_expire: name the mail to expire. Usage: .mail_expire <mailId>";

/// `.mail`'s arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MailArgs {
    pub to: Option<String>,
    pub cash: i64,
    pub item: Option<(i32, i32)>,
    pub cod: Option<i32>,
    pub subject: String,
}

/// Why a GM mail command was refused on the cell: a stable `reason` and
/// the GM's line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Refusal {
    pub reason: &'static str,
    pub line: String,
}

fn refusal(reason: &'static str, what: &str) -> Refusal {
    Refusal {
        reason,
        line: format!(".mail: {what}. {MAIL_USAGE}"),
    }
}

/// The word after an option, parsed as a number in `min..=i32::MAX`.
fn number(
    args: &[&str],
    at: usize,
    min: i32,
    reason: &'static str,
    what: &str,
) -> Result<i32, Refusal> {
    args.get(at)
        .and_then(|w| w.parse::<i32>().ok())
        .filter(|n| *n >= min)
        .ok_or_else(|| refusal(reason, what))
}

/// Parse `.mail`'s words.
pub(crate) fn parse_mail(args: &[&str]) -> Result<MailArgs, Refusal> {
    let mut parsed = MailArgs {
        to: None,
        cash: 0,
        item: None,
        cod: None,
        subject: String::new(),
    };
    let mut seen_cash = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].to_ascii_lowercase().as_str() {
            "to" if parsed.to.is_none() => {
                let name = args
                    .get(i + 1)
                    .ok_or_else(|| refusal("no_recipient_name", "`to` needs a name"))?;
                parsed.to = Some((*name).to_string());
                i += 2;
            }
            "cash" if !seen_cash => {
                let n = number(
                    args,
                    i + 1,
                    0,
                    "invalid_cash",
                    "`cash` needs an amount from 0 to 2147483647",
                )?;
                parsed.cash = i64::from(n);
                seen_cash = true;
                i += 2;
            }
            "item" if parsed.item.is_none() => {
                let type_id = number(
                    args,
                    i + 1,
                    1,
                    "invalid_item_type",
                    "`item` needs an item type id",
                )?;
                // A number right after the type id is the quantity.
                match args.get(i + 2).map(|w| w.parse::<i32>()) {
                    Some(Ok(qty)) if qty >= 1 => {
                        parsed.item = Some((type_id, qty));
                        i += 3;
                    }
                    Some(Ok(_)) => {
                        return Err(refusal(
                            "invalid_item_quantity",
                            "the item quantity must be 1 or more",
                        ));
                    }
                    _ => {
                        parsed.item = Some((type_id, 1));
                        i += 2;
                    }
                }
            }
            "cod" if parsed.cod.is_none() => {
                let n = number(
                    args,
                    i + 1,
                    1,
                    "invalid_cod",
                    "`cod` needs a price of 1 or more",
                )?;
                parsed.cod = Some(n);
                i += 2;
            }
            _ => break,
        }
    }
    parsed.subject = if i < args.len() {
        args[i..].join(" ")
    } else {
        DEFAULT_SUBJECT.to_string()
    };
    if parsed.cod.is_some() {
        if parsed.item.is_none() {
            return Err(refusal("cod_without_item", "`cod` needs an `item`"));
        }
        if parsed.cash != 0 {
            return Err(refusal(
                "cod_with_cash",
                "`cod` cannot be combined with `cash` (the price is the mail's cash)",
            ));
        }
    }
    Ok(parsed)
}

/// Parse `.mail_expire`'s mail id.
pub(crate) fn parse_mail_expire(args: &[&str]) -> Result<i32, Refusal> {
    args.first()
        .and_then(|w| w.parse::<i32>().ok())
        .filter(|id| *id > 0)
        .ok_or(Refusal {
            reason: "no_mail_id",
            line: MAIL_EXPIRE_USAGE.to_string(),
        })
}

/// The GM as the base needs them, or `None` for an entity with no
/// character (never a GM who passed the console gate in world).
fn actor(caller_id: u32, space_mgr: &SpaceManager) -> Option<MailGmActor> {
    let id = space_mgr.player_identity(caller_id);
    Some(MailGmActor {
        entity_id: caller_id,
        player_id: id.player_id?,
        account_id: id.account_id,
    })
}

/// `.mail`.
pub(super) async fn mail(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let parsed = match parse_mail(args) {
        Ok(p) => p,
        Err(r) => return refuse(caller_id, "mail", r, tx, space_mgr).await,
    };
    let Some(actor) = actor(caller_id, space_mgr) else {
        return refuse(caller_id, "mail", no_character("mail"), tx, space_mgr).await;
    };
    let msg = CellToBaseMsg::MailGm(MailGmCellToBase::Send {
        actor,
        to: parsed.to,
        cash: parsed.cash,
        item: parsed.item,
        cod: parsed.cod,
        subject: parsed.subject,
    });
    forward(caller_id, "mail", msg, tx, space_mgr).await;
}

/// `.mailbox [name]`.
pub(super) async fn mailbox(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(actor) = actor(caller_id, space_mgr) else {
        return refuse(caller_id, "mailbox", no_character("mailbox"), tx, space_mgr).await;
    };
    let msg = CellToBaseMsg::MailGm(MailGmCellToBase::Mailbox {
        actor,
        name: args.first().map(|n| (*n).to_string()),
    });
    forward(caller_id, "mailbox", msg, tx, space_mgr).await;
}

/// `.mail_expire <mailId>`: forwarded to the base, which makes the mail
/// due now and expires it (SS-M4).
pub(super) async fn mail_expire(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let mail_id = match parse_mail_expire(args) {
        Ok(id) => id,
        Err(r) => return refuse(caller_id, "mail_expire", r, tx, space_mgr).await,
    };
    let Some(actor) = actor(caller_id, space_mgr) else {
        let r = no_character("mail_expire");
        return refuse(caller_id, "mail_expire", r, tx, space_mgr).await;
    };
    let msg = CellToBaseMsg::MailGm(MailGmCellToBase::Expire { actor, mail_id });
    forward(caller_id, "mail_expire", msg, tx, space_mgr).await;
}

fn no_character(cmd: &str) -> Refusal {
    Refusal {
        reason: "no_player_id",
        line: format!(".{cmd}: you have no character loaded."),
    }
}

/// Hand a GM mail command to the base, which answers the GM.
async fn forward(
    caller_id: u32,
    cmd: &'static str,
    msg: CellToBaseMsg,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if tx.send(msg).await.is_err() {
        let gm = space_mgr.player_identity(caller_id);
        tracing::warn!(
            target: "mail",
            event = "mail.gm_rejected",
            account_id = gm.account_id,
            player_id = gm.player_id,
            entity_id = caller_id,
            command = cmd,
            reason = "base_channel_closed",
            "GM mail command dropped: the base channel is closed"
        );
    }
}

/// Log a refused GM mail command (`mail.gm_rejected`) and tell the GM why.
async fn refuse(
    caller_id: u32,
    cmd: &'static str,
    r: Refusal,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let gm = space_mgr.player_identity(caller_id);
    tracing::warn!(
        target: "mail",
        event = "mail.gm_rejected",
        account_id = gm.account_id,
        player_id = gm.player_id,
        entity_id = caller_id,
        command = cmd,
        reason = r.reason,
        "GM mail command refused: nothing was written"
    );
    send_gm_feedback(caller_id, &r.line, tx).await;
}
