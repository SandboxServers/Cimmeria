//! Team, Command and officer chat (ORG-09): `onPlayerCommunication` to the
//! online members of the speaker's organization of the channel's type.
//!
//! - team (`CHAN_TEAM`, 3): the speaker's Team;
//! - command (`CHAN_COMMAND`, 5): the speaker's Command;
//! - officer (`CHAN_OFFICER`, 6): the members of the speaker's Command whose
//!   rank holds `OfficerChat`, and the speaker must hold it too.
//!
//! Officer is a Command channel: `OfficerChat` is in the Command rank
//! editor only (`Command.lua` `commandPermissions`, audit A-12; Team's
//! editor has no such bit), the officer ranks exist only in a Command, and
//! the legacy mail enum has `MAIL_ToCommandOfficers` but no Team twin.
//!
//! Membership lives in the base's database, so these channels are handled
//! here and never reach the cell. The caller (the base chat dispatch) has
//! already applied the chat bucket (SS-00), the channel allowlist and the
//! GM mute (SS-C3) and the text rules (`org_text::validate`); none is
//! repeated. The speaker's membership is a display read with no org lock:
//! a chat line changes no state, and a demotion that commits while a line
//! is in flight costs at most that one line, the same window
//! `broadcast_to_org`'s own recipient filter has.
//!
//! The speaker gets their own line too, as squad chat does (ORG-04). No
//! channel is registered: the client hardcodes 3, 5 and 6 (D-ORG14), so
//! nothing here sends `onChatJoined`.
//!
//! Telemetry on `org`: one INFO `org.chat` row per line (`outcome`,
//! `reason`, `channel`, `org_id`, `org_type`, `recipients` (members
//! reached, the speaker's own copy not counted), `text_units`, never the
//! text), counted on `org_actions_total{action = "chat"}`. WARN
//! `org.send_failed` for the speaker's copy that could not be sent; the
//! member sends log it in `send_to_members`.

use std::net::SocketAddr;

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::name_intern::intern;
use cimmeria_entity::organization::{OrgPermission, OrgType};
use cimmeria_wire::cell::chat::{
    serialize_on_player_communication, CHAN_COMMAND, CHAN_OFFICER, CHAN_TEAM,
};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;

use super::broadcast::broadcast_except;
use super::fanout::send_to_player;
use super::telemetry::count;
use super::OrgCtx;
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::base::organization::persistence::load_memberships;
use crate::base::session_identity::session_identity;

/// The line a speaker in no Team reads.
pub const NOT_IN_TEAM_TEXT: &str = "You are not in a team.";
/// The line a speaker in no Command reads (command and officer).
pub const NOT_IN_COMMAND_TEXT: &str = "You are not in a command.";
/// The line a speaker whose rank lacks `OfficerChat` reads.
pub const NO_OFFICER_CHAT_TEXT: &str = "Your rank cannot speak on the officer channel.";
/// The line for a membership that could not be read.
pub const ORG_CHAT_UNAVAILABLE_TEXT: &str =
    "Organization chat is unavailable right now. Try again later.";

/// Which organization and permission a channel speaks to, or `None` for a
/// channel this module does not handle.
pub fn org_channel(channel: u8) -> Option<(OrgType, Option<OrgPermission>)> {
    match channel {
        CHAN_TEAM => Some((OrgType::Team, None)),
        CHAN_COMMAND => Some((OrgType::Command, None)),
        CHAN_OFFICER => Some((OrgType::Command, Some(OrgPermission::OFFICER_CHAT))),
        _ => None,
    }
}

/// The speaker, from the base's own session state.
#[derive(Debug, Clone, Copy)]
pub struct ChatSpeaker<'a> {
    pub addr: SocketAddr,
    pub name: &'a str,
    pub flags: u8,
    pub account_id: Option<u32>,
    pub player_id: Option<i32>,
    pub entity_id: Option<u32>,
}

/// The `org.chat` outcome row.
#[derive(Debug, Clone, Copy)]
struct ChatRow {
    channel: u8,
    account_id: Option<u32>,
    player_id: Option<i32>,
    entity_id: Option<u32>,
    /// The names that pair with the ids (Rule 6); `None` when not resolved.
    account_name: Option<&'static str>,
    player_name: Option<&'static str>,
    org_id: Option<i32>,
    org_name: Option<&'static str>,
    org_type: Option<&'static str>,
    recipients: usize,
    text_units: usize,
}

impl ChatRow {
    fn emit(self, reason: Option<&'static str>) {
        let outcome = if reason.is_some() { "rejected" } else { "ok" };
        tracing::info!(
            target: "org",
            event = "org.chat",
            outcome,
            reason,
            channel = self.channel,
            account_id = self.account_id,
            account_name = self.account_name,
            player_id = self.player_id,
            player_name = self.player_name,
            entity_id = self.entity_id,
            entity_name = self.player_name,
            org_id = self.org_id,
            org_name = self.org_name,
            org_type = self.org_type,
            recipients = self.recipients,
            text_units = self.text_units,
            "organization chat {}",
            outcome
        );
        count("chat", outcome, reason.unwrap_or("none"));
    }
}

/// The `org.chat` row for a line on 3, 5 or 6 that the base chat dispatch
/// refused before this module ran (`rate_limited`, `text_invalid`), so an
/// org line's refusal is found under one event. `log_row` is false for a
/// rate-limited drop whose feedback is throttled; the counter still counts
/// it. A no-op for any other channel.
pub fn log_refused_before_relay(
    channel: u8,
    reason: &'static str,
    log_row: bool,
    speaker: &ChatSpeaker<'_>,
    text_units: usize,
) {
    let Some((org_type, _)) = org_channel(channel) else {
        return;
    };
    let row = ChatRow {
        channel,
        account_id: speaker.account_id,
        player_id: speaker.player_id,
        entity_id: speaker.entity_id,
        // No session lookup here: the caller already refused the line.
        account_name: None,
        player_name: intern(speaker.name),
        org_id: None,
        org_name: None,
        org_type: Some(org_type.name()),
        recipients: 0,
        text_units,
    };
    if log_row {
        row.emit(Some(reason));
    } else {
        count("chat", "rejected", reason);
    }
}

/// The speaker's feedback line for a refusal.
async fn refusal_line(ctx: &OrgCtx<'_>, addr: SocketAddr, line: &'static str) {
    let feedback = FeedbackCtx {
        transport: ctx.transport,
        connected: ctx.connected,
    };
    send_feedback_line(&feedback, addr, line).await;
}

/// Relay `text` from `speaker` on `channel` (3, 5 or 6) to their
/// organization. Every path ends in one `org.chat` row; every refusal also
/// sends the speaker one feedback line.
#[tracing::instrument(
    name = "org.chat",
    level = "info",
    target = "org",
    skip_all,
    fields(channel, player_id = speaker.player_id, entity_id = speaker.entity_id)
)]
pub async fn relay_org_chat(ctx: &OrgCtx<'_>, speaker: ChatSpeaker<'_>, channel: u8, text: &str) {
    let Some((org_type, required)) = org_channel(channel) else {
        return;
    };
    let identity = ctx
        .connected
        .lock()
        .ok()
        .and_then(|c| c.get(&speaker.addr).map(session_identity))
        .unwrap_or(PlayerIdentity::UNKNOWN);
    let mut row = ChatRow {
        channel,
        account_id: speaker.account_id,
        player_id: speaker.player_id,
        entity_id: speaker.entity_id,
        account_name: identity.account_name,
        player_name: identity.player_name,
        org_id: None,
        org_name: None,
        org_type: Some(org_type.name()),
        recipients: 0,
        text_units: text.encode_utf16().count(),
    };
    let refuse = |row: ChatRow, reason: &'static str, line: &'static str| {
        row.emit(Some(reason));
        refusal_line(ctx, speaker.addr, line)
    };

    let (Some(player_id), Some(entity_id)) = (speaker.player_id, speaker.entity_id) else {
        // No character in the world: there is no org to speak to and no
        // entity to address a feedback line to.
        row.emit(Some("not_in_world"));
        return;
    };
    let Some(pool) = ctx.db_pool.as_deref() else {
        refuse(row, "no_db", ORG_CHAT_UNAVAILABLE_TEXT).await;
        return;
    };
    let memberships = match load_memberships(pool, player_id).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(
                target: "org",
                event = "org.chat_lookup_failed",
                account_id = speaker.account_id,
                account_name = row.account_name,
                player_id,
                player_name = row.player_name,
                channel,
                reason = e.reason(),
                error = %e,
                "organization chat: the speaker's memberships could not be read"
            );
            refuse(row, "db_error", ORG_CHAT_UNAVAILABLE_TEXT).await;
            return;
        }
    };
    let Some(membership) = memberships
        .into_iter()
        .find(|m| m.header.org_type == org_type)
    else {
        let line = match org_type {
            OrgType::Team => NOT_IN_TEAM_TEXT,
            _ => NOT_IN_COMMAND_TEXT,
        };
        refuse(row, "not_in_org", line).await;
        return;
    };
    row.org_id = Some(membership.header.org_id);
    row.org_name = intern(&membership.header.name);
    if let Some(bits) = required {
        if !membership.display_permissions.contains(bits) {
            refuse(row, "missing_permission", NO_OFFICER_CHAT_TEXT).await;
            return;
        }
    }

    let args = serialize_on_player_communication(speaker.name, speaker.flags, channel, text);
    row.recipients = broadcast_except(
        ctx,
        membership.header.org_id,
        ON_PLAYER_COMMUNICATION,
        &args,
        required,
        Some(player_id),
        "chat",
    )
    .await;
    if let Err(reason) = send_to_player(ctx, entity_id, &[(ON_PLAYER_COMMUNICATION, args)]).await {
        tracing::warn!(
            target: "org",
            event = "org.send_failed",
            what = "chat_echo",
            org_id = membership.header.org_id,
            org_name = membership.header.name.as_str(),
            account_id = speaker.account_id,
            account_name = row.account_name,
            player_id,
            player_name = row.player_name,
            entity_id,
            entity_name = row.player_name,
            reason,
            "organization chat: the speaker's own copy could not be sent"
        );
    }
    row.emit(None);
}
