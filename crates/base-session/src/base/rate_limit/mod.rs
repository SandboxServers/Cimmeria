//! Per-player, per-category rate limits (D-SS14, D-SS21).
//!
//! Every limit lives on the base: chat, tells and duel challenges arrive
//! there, and a mail send reaches the base as `MailOp::Send` before any SQL
//! runs. Each session carries a [`PlayerRateState`] on
//! `ConnectedClientState::rate_limits`, so the buckets die with the session
//! and a reconnect starts full.
//!
//! The clock is the caller's: [`PlayerRateState::check`] takes `now`, so
//! tests step time by exact durations instead of sleeping.
//!
//! An over-limit action is **dropped**, never queued. The caller sends one
//! feedback line when the decision says `notify` (at most once per
//! [`limits::NOTIFY_INTERVAL`] per category) and logs through
//! [`log_exceeded`].
//!
//! The numbers are all in [`limits`], and they are project policy, not
//! recovered data.

mod bucket;
pub mod limits;

use std::net::SocketAddr;
use std::time::Instant;

pub use bucket::TokenBucket;
use limits::BucketSpec;

/// What is being limited. One bucket per category per player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateCategory {
    /// Every player chat channel, tells included.
    Chat,
    /// A mail send (`MailOp::Send`).
    MailSend,
    /// A duel challenge.
    DuelChallenge,
}

impl RateCategory {
    pub const ALL: [RateCategory; 3] = [
        RateCategory::Chat,
        RateCategory::MailSend,
        RateCategory::DuelChallenge,
    ];

    /// The bucket size and refill period for this category.
    pub fn spec(self) -> BucketSpec {
        match self {
            RateCategory::Chat => limits::CHAT,
            RateCategory::MailSend => limits::MAIL_SEND,
            RateCategory::DuelChallenge => limits::DUEL_CHALLENGE,
        }
    }

    /// Stable log value for the `category` field.
    pub fn name(self) -> &'static str {
        match self {
            RateCategory::Chat => "chat",
            RateCategory::MailSend => "mail_send",
            RateCategory::DuelChallenge => "duel_challenge",
        }
    }

    /// The one feedback line for an over-limit action (D-SS14). The duel
    /// wording is provisional; SS-D1 may replace it with a client text id.
    pub fn feedback_text(self) -> &'static str {
        match self {
            RateCategory::Chat | RateCategory::MailSend => "You are sending messages too quickly.",
            RateCategory::DuelChallenge => "You are sending duel challenges too quickly.",
        }
    }

    fn slot(self) -> usize {
        match self {
            RateCategory::Chat => 0,
            RateCategory::MailSend => 1,
            RateCategory::DuelChallenge => 2,
        }
    }
}

/// The answer to one [`PlayerRateState::check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateDecision {
    /// A token was taken; go ahead.
    Allowed,
    /// Drop the action. `notify` is true at most once per
    /// [`limits::NOTIFY_INTERVAL`]: send the category's feedback line then,
    /// and stay silent otherwise.
    Limited { notify: bool },
}

/// Every bucket one player has. Buckets are created full on first use.
#[derive(Debug, Clone, Default)]
pub struct PlayerRateState {
    buckets: [Option<TokenBucket>; 3],
}

impl PlayerRateState {
    /// Take one token from `category`'s bucket at `now`.
    pub fn check(&mut self, category: RateCategory, now: Instant) -> RateDecision {
        self.buckets[category.slot()]
            .get_or_insert_with(|| TokenBucket::new(category.spec(), now))
            .check(now)
    }

    /// `category`'s bucket, if this player has used it yet.
    pub fn bucket(&self, category: RateCategory) -> Option<&TokenBucket> {
        self.buckets[category.slot()].as_ref()
    }
}

/// Who was limited, for [`log_exceeded`]: server session state only.
#[derive(Debug, Clone, Copy)]
pub struct RateActor {
    pub addr: SocketAddr,
    pub player_id: Option<i32>,
    pub account_id: u32,
}

/// Log a dropped action as `rate_limit.exceeded` and count it.
///
/// WARN for the drop that also notifies the player (at most one per
/// category per 5 seconds per player, so a flood cannot flood the log);
/// DEBUG for the silent drops between them. Both carry the bucket state
/// (`tokens`, `burst`, `refill_ms`, `next_token_ms`) so SigNoz alone shows
/// how far over the limit the player was. Every drop, notified or not,
/// increments `rate_limit_exceeded_total{category}`.
pub fn log_exceeded(
    category: RateCategory,
    actor: RateActor,
    notify: bool,
    state: &PlayerRateState,
    now: Instant,
) {
    cimmeria_observability::counter!(
        "rate_limit_exceeded_total",
        "category" => category.name(),
    );
    let spec = category.spec();
    let (tokens, next_token_ms) = state
        .bucket(category)
        .map(|b| (b.tokens(), b.until_next_token(now).as_millis() as u64))
        .unwrap_or((spec.burst, 0));
    let burst = spec.burst;
    let refill_ms = spec.refill_every.as_millis() as u64;
    let addr = actor.addr;
    let player_id = actor.player_id;
    let account_id = actor.account_id;
    if notify {
        tracing::warn!(
            target: "rate_limit",
            event = "rate_limit.exceeded",
            category = category.name(),
            %addr,
            player_id,
            account_id,
            tokens,
            burst,
            refill_ms,
            next_token_ms,
            reason = "bucket_empty",
            notified = true,
            "rate_limit.exceeded: action dropped, player notified",
        );
    } else {
        tracing::debug!(
            target: "rate_limit",
            event = "rate_limit.exceeded",
            category = category.name(),
            %addr,
            player_id,
            account_id,
            tokens,
            burst,
            refill_ms,
            next_token_ms,
            reason = "bucket_empty",
            notified = false,
            "rate_limit.exceeded: action dropped, notify suppressed",
        );
    }
}

#[cfg(test)]
mod tests;
