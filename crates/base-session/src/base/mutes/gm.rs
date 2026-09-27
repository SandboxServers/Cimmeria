//! `.mute <name> <minutes> [reason]` and `.unmute <name>` on the base
//! (SS-C3, D-SS26).
//!
//! The cell's `.`-console has already passed the GM gate (GameMaster and
//! above, from the cell's own `CellEntity::access_level`) and parsed the
//! arguments; it hands over `ChatCellToBase::Mute` / `Unmute` with the GM's
//! ids from its session state. The base owns the rest, because only the base
//! sees every online character: the D-SS13 name lookup, the refusals, the
//! [`MuteTable`] write, and a feedback line to both the GM and the player.
//!
//! Only online characters can be muted or unmuted: the name resolves through
//! the online index. A mute outlives the session it was set on (the table is
//! keyed by `player_id`), so a player who logs off stays muted when they
//! come back.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_entity::organization::org_text::{validate, TextField};

use super::{minutes_left, MuteEntry, MuteTable, MAX_MUTE_MINUTES};
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::base::gm_feedback::send_gm_feedback_to_client;
use crate::base::player_index::{NameLookup, OnlinePlayerIndex};
use crate::base::rate_limit::limits::CHAT_EXEMPT_ACCESS_LEVEL;

/// Longest prefix of a typed name echoed back to the GM.
const SHOWN_NAME_CHARS: usize = 64;

/// The GM who ran the command, from the cell's session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GmActor {
    pub entity_id: u32,
    pub player_id: Option<i32>,
    pub account_id: Option<u32>,
}

/// What the handlers need from the base: the sockets, the session map, and
/// the entity-to-address map that finds the GM's session.
#[derive(Clone, Copy)]
pub struct GmMuteCtx<'a> {
    pub feedback: FeedbackCtx<'a>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// How a `.mute` or `.unmute` ended. Every variant is logged and answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuteOutcome {
    Muted { subject_player_id: i32 },
    Unmuted { subject_player_id: i32 },
    Refused(&'static str),
}

/// The online character a GM named.
struct Subject {
    addr: SocketAddr,
    name: String,
    player_id: i32,
    account_id: u32,
    entity_id: Option<u32>,
    access_level: u32,
}

fn shown(name: &str) -> String {
    name.chars().take(SHOWN_NAME_CHARS).collect()
}

/// Resolve `name` to an online character, or the refusal reason and the
/// GM's line.
fn resolve(ctx: &GmMuteCtx<'_>, name: &str) -> Result<Subject, (&'static str, String)> {
    if validate(TextField::MailRecipient, name).is_err() {
        return Err((
            "bad_name",
            "That is not a valid character name.".to_string(),
        ));
    }
    let clients = ctx
        .feedback
        .connected
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let found = match OnlinePlayerIndex::new(&clients).lookup(name) {
        NameLookup::Found(p) => p,
        NameLookup::Ambiguous => {
            return Err((
                "ambiguous",
                format!(
                    "More than one player is named {}. Type the exact name.",
                    shown(name)
                ),
            ))
        }
        NameLookup::NotFound => {
            return Err((
                "not_online",
                format!("Player {} is not online.", shown(name)),
            ))
        }
    };
    let Some(c) = clients.get(&found.addr) else {
        return Err((
            "not_online",
            format!("Player {} is not online.", shown(name)),
        ));
    };
    Ok(Subject {
        addr: found.addr,
        name: c.player_name.clone().unwrap_or_default(),
        player_id: found.player_id,
        account_id: c.account_id,
        entity_id: c.player_entity_id,
        access_level: c.access_level,
    })
}

async fn tell_gm(ctx: &GmMuteCtx<'_>, actor: GmActor, text: &str) {
    send_gm_feedback_to_client(
        actor.entity_id,
        text,
        ctx.feedback.transport,
        ctx.feedback.connected,
        ctx.entity_to_addr,
    )
    .await;
}

/// `.mute <name> <minutes> [reason]`.
pub async fn apply_gm_mute(
    ctx: &GmMuteCtx<'_>,
    table: &MuteTable,
    actor: GmActor,
    target_name: &str,
    minutes: u32,
    reason: &str,
    now: Instant,
) -> MuteOutcome {
    let refuse = |why: &'static str, subject_player_id: Option<i32>| {
        tracing::warn!(
            target: "chat",
            event = "chat.gm_mute_refused",
            entity_id = actor.entity_id,
            player_id = actor.player_id,
            account_id = actor.account_id,
            subject_player_id,
            duration_minutes = minutes,
            reason = why,
            "GM .mute refused, the GM was told why",
        );
        MuteOutcome::Refused(why)
    };

    if minutes == 0 || minutes > MAX_MUTE_MINUTES {
        // The console parser already bounds this; a message from anywhere
        // else is refused the same way.
        let outcome = refuse("bad_duration", None);
        let text = format!(".mute: minutes must be 1 to {MAX_MUTE_MINUTES}.");
        tell_gm(ctx, actor, &text).await;
        return outcome;
    }
    let subject = match resolve(ctx, target_name) {
        Ok(s) => s,
        Err((why, text)) => {
            let outcome = refuse(why, None);
            tell_gm(ctx, actor, &format!(".mute: {text}")).await;
            return outcome;
        }
    };
    if subject.access_level >= CHAT_EXEMPT_ACCESS_LEVEL {
        // GameMaster and above are never refused by the mute gate (they run
        // the `.` console over chat), so a mute on one would do nothing.
        let outcome = refuse("target_is_gm", Some(subject.player_id));
        let text = format!(".mute: {} is a GM and cannot be muted.", subject.name);
        tell_gm(ctx, actor, &text).await;
        return outcome;
    }

    let duration = Duration::from_secs(u64::from(minutes) * 60);
    let entry = MuteEntry {
        until: now + duration,
        by_account_id: actor.account_id,
    };
    let previous = table.mute(subject.player_id, entry, now);
    tracing::info!(
        target: "chat",
        event = "chat.gm_mute",
        entity_id = actor.entity_id,
        player_id = actor.player_id,
        account_id = actor.account_id,
        subject_player_id = subject.player_id,
        subject_account_id = subject.account_id,
        subject_entity_id = subject.entity_id,
        duration_minutes = minutes,
        reason,
        previous_remaining_secs = previous.map(|d| d.as_secs()),
        remaining_secs = duration.as_secs(),
        "GM muted a player",
    );

    let unit = if minutes == 1 { "minute" } else { "minutes" };
    let gm_text = match previous {
        Some(_) => format!(
            "Muted {} for {minutes} {unit} (replaced the earlier mute).",
            subject.name
        ),
        None => format!("Muted {} for {minutes} {unit}.", subject.name),
    };
    tell_gm(ctx, actor, &gm_text).await;
    let subject_text = format!(
        "A GM has muted you for {minutes} {unit}. You cannot chat or send tells until it ends."
    );
    send_feedback_line(&ctx.feedback, subject.addr, &subject_text).await;
    MuteOutcome::Muted {
        subject_player_id: subject.player_id,
    }
}

/// `.unmute <name>`.
pub async fn apply_gm_unmute(
    ctx: &GmMuteCtx<'_>,
    table: &MuteTable,
    actor: GmActor,
    target_name: &str,
    now: Instant,
) -> MuteOutcome {
    let refuse = |why: &'static str, subject_player_id: Option<i32>| {
        tracing::warn!(
            target: "chat",
            event = "chat.gm_unmute_refused",
            entity_id = actor.entity_id,
            player_id = actor.player_id,
            account_id = actor.account_id,
            subject_player_id,
            reason = why,
            "GM .unmute refused, the GM was told why",
        );
        MuteOutcome::Refused(why)
    };

    let subject = match resolve(ctx, target_name) {
        Ok(s) => s,
        Err((why, text)) => {
            let outcome = refuse(why, None);
            tell_gm(ctx, actor, &format!(".unmute: {text}")).await;
            return outcome;
        }
    };
    let Some(remaining) = table.unmute(subject.player_id, now) else {
        let outcome = refuse("not_muted", Some(subject.player_id));
        let text = format!(".unmute: {} is not muted.", subject.name);
        tell_gm(ctx, actor, &text).await;
        return outcome;
    };
    tracing::info!(
        target: "chat",
        event = "chat.gm_unmute",
        entity_id = actor.entity_id,
        player_id = actor.player_id,
        account_id = actor.account_id,
        subject_player_id = subject.player_id,
        subject_account_id = subject.account_id,
        subject_entity_id = subject.entity_id,
        previous_remaining_secs = remaining.as_secs(),
        previous_remaining_minutes = minutes_left(remaining),
        remaining_secs = 0u64,
        "GM lifted a player's mute",
    );
    tell_gm(ctx, actor, &format!("Unmuted {}.", subject.name)).await;
    send_feedback_line(
        &ctx.feedback,
        subject.addr,
        "A GM has lifted your mute. You can chat again.",
    )
    .await;
    MuteOutcome::Unmuted {
        subject_player_id: subject.player_id,
    }
}
