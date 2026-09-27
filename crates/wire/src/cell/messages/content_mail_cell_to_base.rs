//! `ContentSystemMail`: the content engine's `send_system_mail` action
//! (SS-U3), carried by `CellToBaseMsg::ContentSystemMail`.
//!
//! A chain runs on the cell, but the mail writer (`mail::system`) and the
//! cooldown table live on the base, so the action is one message and the
//! base does the whole transaction: the cooldown claim, the mail and its
//! escrow row commit together or not at all. The recipient is always the
//! player whose event fired the chain; the ids come from the cell's own
//! entity, never from a client payload.

/// A per-player cooldown on one content mail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentMailCooldown {
    /// `sgw_player_content_cooldown.cooldown_key`, at most 64 characters.
    /// The action derives it from its chain id, so two chains never share
    /// a cooldown by accident.
    pub key: String,
    /// Seconds between two mails, at least 1.
    pub secs: u32,
}

/// One system mail a content chain asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentSystemMail {
    /// The recipient's entity, for the feedback line.
    pub entity_id: u32,
    /// The recipient's `sgw_player.player_id`.
    pub player_id: i32,
    /// The recipient's `account.account_id`, `None` if the cell has none.
    pub account_id: Option<u32>,
    /// The chain that fired the action, for the log.
    pub chain_id: i64,
    /// Shown as the sender ("Gate Mail Clerk").
    pub sender_name: String,
    pub subject: String,
    pub body: String,
    /// Naquadah, minted.
    pub cash: i64,
    /// `(type_id, quantity)`, minted into escrow.
    pub item: Option<(i32, i32)>,
    /// `None` sends on every firing.
    pub cooldown: Option<ContentMailCooldown>,
}
