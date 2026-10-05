//! Two gates on `sendPlayerCommunication` that run on the base, before the
//! cell, a tell recipient, or a user channel's members see the line
//! (SS-C3):
//!
//! - the **channel allowlist** (CAT-L-03): a player may speak on say, emote,
//!   yell, tell, the organization channels (team, squad, command, officer)
//!   and any user channel (12 and up). Squad is forwarded to the cell
//!   (ORG-04); team, command and officer are handled on the base (ORG-09,
//!   `chat.rs`); a user channel is also handled on the base
//!   (`chat.rs::post_to_user_channel`), which is the layer that checks the
//!   caller is actually a member -- this allowlist only knows the id
//!   *shape*, not membership. The system channels (server, feedback,
//!   splash) and every id `EChannel` does not name are refused here with
//!   feedback;
//! - the **mute gate** (D-SS26): a player a GM muted gets a feedback line
//!   instead of their spatial line or tell, until the mute expires.
//!
//! Both run after the chat bucket, so a refused line still costs a token
//! and a client cannot turn a flood of refused packets into a flood of
//! feedback lines.

use std::net::SocketAddr;
use std::time::Instant;

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::cell::chat::{
    CHAN_CHAT, CHAN_COMMAND, CHAN_EMOTE, CHAN_FEEDBACK, CHAN_OFFICER, CHAN_SAY, CHAN_SERVER,
    CHAN_SPLASH, CHAN_SQUAD, CHAN_TEAM, CHAN_TELL, CHAN_YELL,
};

use super::super::feedback::{send_feedback_line, FeedbackCtx};
use super::super::mutes::{mute_table, muted_text};
use super::super::rate_limit::limits::CHAT_EXEMPT_ACCESS_LEVEL;

/// Feedback for a line on a system channel.
pub(super) const SYSTEM_CHANNEL_TEXT: &str =
    "That channel is for system messages only. Players cannot post there.";
/// Feedback for a line on an id `EChannel` does not name.
pub(super) const UNKNOWN_CHANNEL_TEXT: &str = "That chat channel does not exist.";

/// Why a channel is refused: the `reason` field and the player's line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChannelRefusal {
    pub reason: &'static str,
    pub text: &'static str,
}

/// The allowlist itself. `Ok` for a channel a player may speak on -- for a
/// user channel (12 and up) this is a shape check only, never a membership
/// check, that being `chat.rs::post_to_user_channel`'s job once it has the
/// registry. The ids are the workspace `CHAN_*` constants, which SS-C4
/// aligned with `EChannel` (D-ORG14).
pub(super) fn check_channel(channel: u8) -> Result<(), ChannelRefusal> {
    match channel {
        CHAN_SAY | CHAN_EMOTE | CHAN_YELL | CHAN_TELL | CHAN_TEAM | CHAN_SQUAD | CHAN_COMMAND
        | CHAN_OFFICER => Ok(()),
        CHAN_SERVER | CHAN_FEEDBACK | CHAN_SPLASH => Err(ChannelRefusal {
            reason: "system_channel",
            text: SYSTEM_CHANNEL_TEXT,
        }),
        c if c >= CHAN_CHAT => Ok(()),
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
    /// The session's names, for the refusal logs (Rule 6).
    pub identity: PlayerIdentity,
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
        player_name = who.identity.player_name,
        account_id = who.account_id,
        account_name = who.identity.account_name,
        entity_id = who.entity_id,
        entity_name = who.identity.player_name,
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
        player_name = who.identity.player_name,
        account_id = who.account_id,
        account_name = who.identity.account_name,
        entity_id = who.entity_id,
        entity_name = who.identity.player_name,
        channel,
        tell = channel == CHAN_TELL,
        remaining_secs = remaining.as_secs(),
        muted_by_account_id = mute.by_account_id, // nt:id-only MuteEntry stores the muting GM's account id only, no name
        reason = "muted",
        "chat line refused: the speaker is muted by a GM, not forwarded",
    );
    send_feedback_line(feedback, who.addr, &muted_text(remaining)).await;
    true
}
