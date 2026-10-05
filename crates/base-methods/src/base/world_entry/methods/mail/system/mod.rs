//! Server-originated gate mail (SS-U1): the one writer every system mail
//! goes through. The Black Market's payouts (BM-02b), the Gate Mail Clerk's
//! content action (SS-U3) and the GM `.mail` command call it; nothing else
//! inserts a mail with an attachment except the player send path.
//!
//! What a system mail is:
//!
//! - **No sender character.** `sender_id` is NULL and `sender_name` is the
//!   label the caller gives ("Black Market", a GM's name). D-SS10: a mail
//!   with no `sender_id` cannot be returned, so a system mail can never be
//!   bounced to a sender that does not exist.
//! - **No postage and no COD.** Nothing is debited from anyone: the cash is
//!   minted (or was already taken from its payer by the caller, in the same
//!   transaction), and the flags carry no `MAIL_COD`.
//! - **Exempt from the mailbox cap** (D-SS03: "server-generated mail ignores
//!   the cap, so money and items are never lost to it"). The open count is
//!   logged, so an over-full box shows in SigNoz.
//! - **Offline recipients are fine.** Only the database is touched; the
//!   recipient sees the mail the next time the mailbox is opened. A caller
//!   that holds the session map tells an online recipient after its commit
//!   with [`SystemMailSent::notify`] (D-SS11, SS-M4).
//! - **It expires like any mail** (SS-M4): 30 days after it is written.
//!   It has no sender to go back to, so if it still holds an item or cash
//!   then, the expiry sweep quarantines it for a GM instead of deleting it.
//!
//! The item, when there is one, lands in `sgw_gate_mail_item` exactly like
//! a player's attachment (D-SS08), so SS-M3's take path treats both alike:
//!
//! - [`SystemItem::Minted`] creates the escrow row from the item template,
//!   with `grant_item`'s instance defaults, under a fresh id from
//!   `sgw_inventory_item_id_seq`. Its `source_character_id` is
//!   [`SYSTEM_SOURCE_CHARACTER_ID`].
//! - [`SystemItem::ExistingInstance`] moves a **server-held** row, whole and
//!   with every instance column, out of `sgw_inventory`. Server-held means
//!   `container_id` is in [`SERVER_HELD_CONTAINERS`] (today only the
//!   auction container, 18): a container no player can move items in or out
//!   of (`inventory/move_/container_policy.rs` makes it `Movable::No`). Any
//!   other container, a bag, equipment, a vault, buyback, is a live item a
//!   player holds, and is refused, so a caller bug cannot yank one. The
//!   caller names the owner it expects, so a wrong id cannot take another
//!   seller's row, and a bound row only ever goes back to its owner.
//!
//! [`send_system_mail_tx`] runs inside the caller's transaction and commits
//! nothing; [`send_system_mail`] wraps it in a transaction of its own. The
//! caller of the `_tx` form logs [`SystemMailSent::log_sent`] after its own
//! commit, so `mail.system_sent` never names a mail that was rolled back.

mod write;

use std::fmt;

use cimmeria_entity::inventory::INV_AUCTION;
use cimmeria_entity::organization::org_text::{self, TextField};
use sqlx::{PgPool, Postgres, Transaction};

use super::notify::{notify_delivered, Delivery};
use crate::base::feedback::FeedbackCtx;

pub(super) use write::{write_mail, MailHeader};

/// `sgw_gate_mail_item.source_character_id` of a minted item: no character
/// sent it. Player ids start at 1, so 0 names nobody. Forensics only; the
/// return path (SS-M3) re-addresses by the mail's `sender_id`, never this.
pub const SYSTEM_SOURCE_CHARACTER_ID: i32 = 0;

/// Containers whose rows the server holds on a player's behalf, so a system
/// mail may move them without the player's allowlist. Only the auction
/// container today: an item listed on the Black Market sits there, and no
/// player move reaches it.
pub const SERVER_HELD_CONTAINERS: &[i32] = &[INV_AUCTION];

/// One system mail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemMail {
    /// Shown as the sender (`sender_name`), e.g. "Black Market". 1-128
    /// characters, one line (the D-SS12 subject rules).
    pub sender_name: String,
    /// The recipient's `sgw_player.player_id`. Online or not.
    pub recipient_player_id: i32,
    /// 1-128 characters, one line (D-SS12).
    pub subject: String,
    /// Up to 1,000 characters (D-SS12).
    pub body: String,
    /// Gift naquadah, `0..=i32::MAX` (a balance is an `i32`, so more could
    /// never be taken).
    pub cash: i64,
    pub item: SystemItem,
}

/// The item a system mail carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemItem {
    None,
    /// A new instance of `type_id` (`resources.items`), `qty` in one stack,
    /// `1..=max_stack_size`.
    Minted {
        type_id: i32,
        qty: i32,
    },
    /// An existing `sgw_inventory` row in a [`SERVER_HELD_CONTAINERS`]
    /// container, moved whole.
    ///
    /// `owner_player_id` is the character the caller expects to hold it
    /// (the seller); any other owner is refused. A bound row may only be
    /// mailed back to its owner.
    ExistingInstance {
        item_id: i32,
        owner_player_id: i32,
    },
}

impl SystemItem {
    /// Stable `item_source` log value.
    pub fn source(&self) -> &'static str {
        match self {
            SystemItem::None => "none",
            SystemItem::Minted { .. } => "minted",
            SystemItem::ExistingInstance { .. } => "existing_instance",
        }
    }
}

/// The escrow row a system mail created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemEscrow {
    /// The escrow row's instance id: new for a minted item, the moved row's
    /// own id for an existing instance.
    pub item_id: i32,
    pub type_id: i32,
    pub stack_size: i32,
    /// [`SYSTEM_SOURCE_CHARACTER_ID`] for a minted item, else the character
    /// whose server-held row moved.
    pub source_character_id: i32,
}

/// A system mail written by [`send_system_mail_tx`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemMailSent {
    pub mail_id: i32,
    pub recipient_player_id: i32,
    pub sender_name: String,
    pub cash: i64,
    pub item_source: &'static str,
    pub item: Option<SystemEscrow>,
    /// The recipient's open (not archived) mail after this one.
    pub recipient_open_mail: i64,
}

impl SystemMailSent {
    /// Log `mail.system_sent`. Call it after the transaction commits.
    pub fn log_sent(&self) {
        let book = cimmeria_names::book();
        tracing::info!(
            target: "mail",
            event = "mail.system_sent",
            sender_name = %self.sender_name,
            target_player_id = self.recipient_player_id, // nt:id-only recipient is locked by id only here, the calling path logs the named recipient
            mail_id = self.mail_id, // nt:id-only mail row, its subject is player text kept out of logs
            cash = self.cash,
            item_source = self.item_source,
            item_id = self.item.map(|i| i.item_id),
            item_type_id = self.item.map(|i| i.type_id),
            item_name = self
                .item
                .and_then(|i| book.item(i.type_id)),
            stack_size = self.item.map(|i| i.stack_size),
            source_character_id = self.item.map(|i| i.source_character_id), // nt:id-only mint sentinel or the holder of a server-held row, no name loaded
            recipient_open_mail = self.recipient_open_mail,
            over_cap = self.recipient_open_mail > super::send::MAILBOX_CAP,
            "system gate-mail delivered",
        );
    }

    /// Tell the recipient, if online, that the mail arrived (D-SS11): a
    /// feedback line and the header. Call it after the commit, beside
    /// [`Self::log_sent`]; an offline recipient is skipped.
    pub async fn notify(&self, pool: &PgPool, ctx: &FeedbackCtx<'_>) {
        notify_delivered(
            pool,
            ctx,
            self.recipient_player_id,
            self.mail_id,
            Delivery::System,
        )
        .await;
    }
}

/// Why a system mail was refused. Nothing was written.
#[derive(Debug)]
pub enum SystemMailError {
    /// A text field broke the D-SS12 rules.
    InvalidText {
        field: &'static str,
        reason: &'static str,
    },
    /// `cash < 0`.
    NegativeCash,
    /// `cash > i32::MAX`.
    CashTooLarge,
    /// A minted quantity below 1.
    InvalidQuantity,
    /// A minted `type_id` with no `resources.items` row.
    UnknownItemType,
    /// A minted quantity above the type's `max_stack_size`.
    QuantityExceedsStack {
        max_stack_size: i32,
    },
    /// No `sgw_inventory` row has that id.
    ItemNotFound,
    /// The row is in a container a player holds.
    ItemNotServerHeld {
        container_id: i32,
        owner: i32,
    },
    /// The row belongs to someone other than the owner the caller named.
    ItemOwnerMismatch {
        owner: i32,
    },
    /// The row is bound and the recipient is not its owner.
    ItemBound {
        owner: i32,
    },
    /// The type fits no carried bag (`take::carried_bag` is `None`), so no
    /// take could ever deliver it (#959).
    ItemNoCarriedBag {
        type_id: i32,
    },
    /// No `sgw_player` row has that id.
    RecipientNotFound,
    Db(sqlx::Error),
}

impl SystemMailError {
    /// Stable `reason` log value.
    pub fn reason(&self) -> &'static str {
        match self {
            SystemMailError::InvalidText { reason, .. } => reason,
            SystemMailError::NegativeCash => "negative_cash",
            SystemMailError::CashTooLarge => "cash_too_large",
            SystemMailError::InvalidQuantity => "invalid_item_quantity",
            SystemMailError::UnknownItemType => "unknown_item_type",
            SystemMailError::QuantityExceedsStack { .. } => "item_quantity_exceeds_stack",
            SystemMailError::ItemNotFound => "item_not_found",
            SystemMailError::ItemNotServerHeld { .. } => "item_not_server_held",
            SystemMailError::ItemOwnerMismatch { .. } => "item_owner_mismatch",
            SystemMailError::ItemBound { .. } => "item_bound",
            SystemMailError::ItemNoCarriedBag { .. } => "item_no_carried_bag",
            SystemMailError::RecipientNotFound => "recipient_not_found",
            SystemMailError::Db(_) => "db_error",
        }
    }
}

impl fmt::Display for SystemMailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemMailError::InvalidText { field, reason } => write!(f, "{field}: {reason}"),
            SystemMailError::QuantityExceedsStack { max_stack_size } => {
                write!(f, "quantity above the stack size {max_stack_size}")
            }
            SystemMailError::ItemNotServerHeld {
                container_id,
                owner,
            } => write!(
                f,
                "item is in container {container_id} of player {owner}, not server-held"
            ),
            SystemMailError::ItemNoCarriedBag { type_id } => write!(
                f,
                "item type {type_id} fits no backpack or crafting bag, so it could never be taken"
            ),
            SystemMailError::Db(e) => write!(f, "database error: {e}"),
            other => f.write_str(other.reason()),
        }
    }
}

impl std::error::Error for SystemMailError {}

impl From<sqlx::Error> for SystemMailError {
    fn from(e: sqlx::Error) -> Self {
        SystemMailError::Db(e)
    }
}

/// Write `mail` inside the caller's transaction. Commits nothing: if the
/// caller rolls back, neither the mail nor its escrow row remains, and an
/// existing instance is back in its container.
///
/// Lock order is the shared inventory order: for an existing instance, the
/// owner's advisory locks and the item row, then the recipient's
/// `sgw_player` row. A caller that also locks player rows (a payout that
/// debits a buyer) takes its inventory locks first, then every player row
/// in ascending `player_id` order, including the recipient's, before this
/// call.
///
/// Refusals are logged (`mail.system_refused`, WARN, with `reason`). Call
/// [`SystemMailSent::log_sent`] after the commit.
pub async fn send_system_mail_tx(
    tx: &mut Transaction<'_, Postgres>,
    mail: &SystemMail,
) -> Result<SystemMailSent, SystemMailError> {
    let result = async {
        validate(mail)?;
        let header = MailHeader {
            recipient_player_id: mail.recipient_player_id,
            sender_id: None,
            sender_name: &mail.sender_name,
            subject: &mail.subject,
            body: &mail.body,
            cash: mail.cash,
            flags: 0,
        };
        write_mail(tx, &header, mail.item, now_secs()).await
    }
    .await;
    match result {
        Ok(written) => {
            let book = cimmeria_names::book();
            tracing::debug!(
                target: "mail",
                event = "mail.system_staged",
                sender_name = %mail.sender_name,
                target_player_id = mail.recipient_player_id, // nt:id-only recipient is locked by id only here, the calling path logs the named recipient
                mail_id = written.mail_id, // nt:id-only mail row, its subject is player text kept out of logs
                cash = mail.cash,
                item_source = mail.item.source(),
                item_id = written.item.map(|i| i.item_id),
                item_name = written
                    .item
                    .and_then(|i| book.item(i.type_id)),
                "system gate-mail written, awaiting the caller's commit",
            );
            Ok(SystemMailSent {
                mail_id: written.mail_id,
                recipient_player_id: mail.recipient_player_id,
                sender_name: mail.sender_name.clone(),
                cash: mail.cash,
                item_source: mail.item.source(),
                item: written.item,
                recipient_open_mail: written.recipient_open_mail,
            })
        }
        Err(e) => {
            log_refused(mail, &e);
            Err(e)
        }
    }
}

/// [`send_system_mail_tx`] in a transaction of its own, committed, with
/// `mail.system_sent` logged.
pub async fn send_system_mail(
    pool: &PgPool,
    mail: &SystemMail,
) -> Result<SystemMailSent, SystemMailError> {
    let mut tx = pool.begin().await?;
    let sent = send_system_mail_tx(&mut tx, mail).await?;
    tx.commit().await?;
    sent.log_sent();
    Ok(sent)
}

/// The checks that need no database.
pub(super) fn validate(mail: &SystemMail) -> Result<(), SystemMailError> {
    for (field, text) in [
        (TextField::MailSubject, mail.sender_name.as_str()),
        (TextField::MailSubject, mail.subject.as_str()),
        (TextField::MailBody, mail.body.as_str()),
    ] {
        org_text::validate(field, text).map_err(|e| SystemMailError::InvalidText {
            field: field.name(),
            reason: e.reason(),
        })?;
    }
    if mail.cash < 0 {
        return Err(SystemMailError::NegativeCash);
    }
    if mail.cash > i64::from(i32::MAX) {
        return Err(SystemMailError::CashTooLarge);
    }
    if let SystemItem::Minted { qty, .. } = mail.item {
        if qty < 1 {
            return Err(SystemMailError::InvalidQuantity);
        }
    }
    Ok(())
}

fn log_refused(mail: &SystemMail, e: &SystemMailError) {
    let (container_id, owner) = match e {
        SystemMailError::ItemNotServerHeld {
            container_id,
            owner,
        } => (Some(*container_id), Some(*owner)),
        SystemMailError::ItemOwnerMismatch { owner } | SystemMailError::ItemBound { owner } => {
            (None, Some(*owner))
        }
        _ => (None, None),
    };
    let (item_id, expected_owner) = match mail.item {
        SystemItem::ExistingInstance {
            item_id,
            owner_player_id,
        } => (Some(item_id), Some(owner_player_id)),
        _ => (None, None),
    };
    let type_id = match (mail.item, e) {
        (SystemItem::Minted { type_id, .. }, _) => Some(type_id),
        (_, SystemMailError::ItemNoCarriedBag { type_id }) => Some(*type_id),
        _ => None,
    };
    let book = cimmeria_names::book();
    tracing::warn!(
        target: "mail",
        event = "mail.system_refused",
        reason = e.reason(),
        sender_name = %mail.sender_name,
        target_player_id = mail.recipient_player_id, // nt:id-only refusal fires before the recipient row is read, no name loaded
        cash = mail.cash,
        item_source = mail.item.source(),
        item_id,
        item_type_id = type_id,
        item_name = type_id.and_then(|t| book.item(t)),
        container_id, // nt:id-only inventory container of the refused row, not a named row
        owner_player_id = owner, // nt:id-only holder of the refused row, no name loaded on this path
        expected_owner_player_id = expected_owner, // nt:id-only caller-named owner, no name loaded on this path
        error = %e,
        "system gate-mail refused; nothing written",
    );
}

/// Epoch seconds, like `sgw_gate_mail.sent_time`.
pub(super) fn now_secs() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i32::try_from(d.as_secs()).unwrap_or(i32::MAX))
        .unwrap_or(0)
}
