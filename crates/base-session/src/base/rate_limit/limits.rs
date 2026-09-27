//! Every rate-limit number, in one place.
//!
//! All of these are **project policy, not recovered data**: nothing in the
//! client or the legacy server says how fast a player may chat, send mail or
//! challenge to a duel. They come from the social-systems ledger
//! (`docs/analysis/social-systems/README.md`): D-SS14 for chat and mail,
//! D-SS21 for duel challenges, D-SS12 for the chat text cap.

use std::time::Duration;

/// Burst size and refill period of one category's token bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BucketSpec {
    /// Tokens a full bucket holds: the number of actions allowed back to back.
    pub burst: u32,
    /// One token comes back every `refill_every`.
    pub refill_every: Duration,
}

/// Chat (D-SS14): every player channel, tells included. Burst 5, then one
/// line a second. The proposal in `server-infrastructure-proposals.md` §2
/// suggested 5 per second; five lines a second is still a flood to the reader.
pub const CHAT: BucketSpec = BucketSpec {
    burst: 5,
    refill_every: Duration::from_secs(1),
};

/// Mail send (D-SS14): burst 3, then one every 10 seconds.
pub const MAIL_SEND: BucketSpec = BucketSpec {
    burst: 3,
    refill_every: Duration::from_secs(10),
};

/// Duel challenge (D-SS21): burst 2, then one every 15 seconds. The
/// challenge prompt is modal on the target's screen, so spam is griefing.
pub const DUEL_CHALLENGE: BucketSpec = BucketSpec {
    burst: 2,
    refill_every: Duration::from_secs(15),
};

/// An over-limit action gets at most one feedback line per this interval,
/// per category (D-SS14), so a flood cannot turn into a flood of replies.
pub const NOTIFY_INTERVAL: Duration = Duration::from_secs(5);

/// Access level at or above which the **chat** bucket is skipped (D-SS14:
/// "GameMaster and above are exempt from the chat bucket only").
/// `cimmeria_commands::permissions::AccessLevel::GameMaster` is 2; the
/// session holds the raw `account.accesslevel` value, so the number is
/// repeated here rather than pulling the commands crate into the session
/// layer.
pub const CHAT_EXEMPT_ACCESS_LEVEL: u32 = 2;

/// Longest chat line accepted, in UTF-16 units (D-SS12: "never more than
/// 255"). Provisional until SS-E1 reports the client's own input cap; the
/// server cap may only go down to meet it, never above 255.
pub const MAX_CHAT_TEXT_UNITS: usize = 255;
