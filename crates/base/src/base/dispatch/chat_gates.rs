//! Two gates on `sendPlayerCommunication` that run on the base, before the
//! cell or a tell recipient sees the line (SS-C3):
//!
//! - the **channel allowlist** (CAT-L-03): a player may speak on say, emote,
//!   yell, tell, and the organization channels (team, squad, command,
//!   officer), which are forwarded to the cell for the organizations
//!   campaign to handle. The system channels (server, feedback, splash),
//!   user channels (12 and up, none of which this server registers) and
//!   every id `EChannel` does not name are refused with feedback;
//! - the **mute gate** (D-SS26): a player a GM muted gets a feedback line
//!   instead of their spatial line or tell, until the mute expires.
//!
//! Both run after the chat bucket, so a refused line still costs a token
//! and a client cannot turn a flood of refused packets into a flood of
//! feedback lines.

use std::net::SocketAddr;
use std::time::Instant;

use super::super::feedback::{send_feedback_line, FeedbackCtx};
use super::super::mutes::{mute_table, muted_text};
use super::super::rate_limit::limits::CHAT_EXEMPT_ACCESS_LEVEL;

/// The `EChannel` ids the client sends and renders, from
/// `entities/defs/enumerations.xml`. The client hardcodes the same literals
/// (D-ORG14, ORG-E1 Q5). The workspace `CHAN_*` constants still carry the
/// old server 7 / tell 9 / splash 10 until ORG-09 aligns them (D-SS17), so
/// the allowlist names its own copies, pinned against the XML by
/// `echannel_ids_match_enumerations_xml`.
pub(super) mod echannel {
    pub(in crate::base::dispatch) const SAY: u8 = 0;
    pub(in crate::base::dispatch) const EMOTE: u8 = 1;
    pub(in crate::base::dispatch) const YELL: u8 = 2;
    pub(in crate::base::dispatch) const TEAM: u8 = 3;
    pub(in crate::base::dispatch) const SQUAD: u8 = 4;
    pub(in crate::base::dispatch) const COMMAND: u8 = 5;
    pub(in crate::base::dispatch) const OFFICER: u8 = 6;
    pub(in crate::base::dispatch) const SERVER: u8 = 8;
    pub(in crate::base::dispatch) const FEEDBACK: u8 = 9;
    pub(in crate::base::dispatch) const TELL: u8 = 10;
    pub(in crate::base::dispatch) const SPLASH: u8 = 11;
    /// The first user-created channel id ("Anything starting at CHAN_chat is
    /// a user-created chat channel").
    pub(in crate::base::dispatch) const CHAT: u8 = 12;
}

/// Feedback for a line on a system channel.
pub(super) const SYSTEM_CHANNEL_TEXT: &str =
    "That channel is for system messages only. Players cannot post there.";
/// Feedback for a line on a user channel (none exist on this server).
pub(super) const USER_CHANNEL_TEXT: &str = "Custom chat channels are not available yet.";
/// Feedback for a line on an id `EChannel` does not name.
pub(super) const UNKNOWN_CHANNEL_TEXT: &str = "That chat channel does not exist.";

/// Why a channel is refused: the `reason` field and the player's line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChannelRefusal {
    pub reason: &'static str,
    pub text: &'static str,
}

/// The allowlist itself. `Ok` for a channel a player may speak on.
pub(super) fn check_channel(channel: u8) -> Result<(), ChannelRefusal> {
    use echannel::*;
    match channel {
        SAY | EMOTE | YELL | TELL | TEAM | SQUAD | COMMAND | OFFICER => Ok(()),
        SERVER | FEEDBACK | SPLASH => Err(ChannelRefusal {
            reason: "system_channel",
            text: SYSTEM_CHANNEL_TEXT,
        }),
        c if c >= CHAT => Err(ChannelRefusal {
            reason: "user_channel",
            text: USER_CHANNEL_TEXT,
        }),
        _ => Err(ChannelRefusal {
            reason: "unknown_channel",
            text: UNKNOWN_CHANNEL_TEXT,
        }),
    }
}

/// The speaker, from server session state, for the refusal logs.
#[derive(Debug, Clone, Copy)]
pub(super) struct Speaker {
    pub addr: SocketAddr,
    pub player_id: Option<i32>,
    pub account_id: u32,
    pub entity_id: Option<u32>,
    pub access_level: u32,
}

/// Refuse `channel` if the allowlist does not hold it: log
/// `chat.channel_rejected` and send one feedback line. Returns `true` when
/// the line was refused.
pub(super) async fn refuse_channel(feedback: &FeedbackCtx<'_>, who: Speaker, channel: u8) -> bool {
    let Err(refusal) = check_channel(channel) else {
        return false;
    };
    tracing::warn!(
        target: "chat",
        event = "chat.channel_rejected",
        addr = %who.addr,
        player_id = who.player_id,
        account_id = who.account_id,
        entity_id = who.entity_id,
        channel,
        reason = refusal.reason,
        "sendPlayerCommunication refused at the base: players cannot speak on this channel, not forwarded",
    );
    send_feedback_line(feedback, who.addr, refusal.text).await;
    true
}

/// Refuse the line if the speaker is muted at `now`: log
/// `chat.muted_refused` and send the time left. GameMaster and above are
/// never refused here: they run the `.` console over chat, and `.mute`
/// refuses a GM target anyway. Returns `true` when the line was refused.
pub(super) async fn refuse_if_muted(
    feedback: &FeedbackCtx<'_>,
    who: Speaker,
    channel: u8,
    now: Instant,
) -> bool {
    if who.access_level >= CHAT_EXEMPT_ACCESS_LEVEL {
        return false;
    }
    let Some(player_id) = who.player_id else {
        return false;
    };
    let Some(mute) = mute_table().active(player_id, now) else {
        return false;
    };
    let remaining = mute.until.saturating_duration_since(now);
    tracing::debug!(
        target: "chat",
        event = "chat.muted_refused",
        addr = %who.addr,
        player_id,
        account_id = who.account_id,
        entity_id = who.entity_id,
        channel,
        tell = channel == echannel::TELL,
        remaining_secs = remaining.as_secs(),
        muted_by_account_id = mute.by_account_id,
        reason = "muted",
        "chat line refused: the speaker is muted by a GM, not forwarded",
    );
    send_feedback_line(feedback, who.addr, &muted_text(remaining)).await;
    true
}
