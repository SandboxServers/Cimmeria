//! Duel console commands (SS-U2): `.duel_status [name]` and `.duel_end
//! <name>`, GM tools for testing duels.
//!
//! Neither has a legacy counterpart; the wording is plain English (the
//! owner's preference for GM commands). The duel registry lives on this
//! cell (`cell::duel`, keyed by `player_id`), so both commands resolve the
//! typed name to a `player_id` through the online-name lookup `.summon`
//! uses, and read or change the registry through `cell::duel::gm`.
//!
//! - `.duel_status` with no name reports the caller's own entry.
//! - `.duel_end` ends whatever the named player is part of (a pending
//!   challenge in either direction, the countdown, or a fight), sends "Duel
//!   aborted" (878) to both players, and starts no pair cooldown.

use std::time::Instant;

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::duel::gm::{gm_end, online_name, status, DuelStatus};
use crate::cell::duel::DuelState;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{PlayerNameLookup, SpaceManager};

pub(crate) const DUEL_END_USAGE: &str =
    ".duel_end: name the player whose duel to end. Usage: .duel_end <name>";

/// `.duel_status`'s one optional argument: the player to look up.
pub(crate) fn parse_duel_status<'a>(args: &[&'a str]) -> Option<&'a str> {
    args.first().copied()
}

/// `.duel_end`'s one required argument: the player whose duel to end.
pub(crate) fn parse_duel_end<'a>(args: &[&'a str]) -> Result<&'a str, &'static str> {
    args.first().copied().ok_or(DUEL_END_USAGE)
}

/// The player a duel command is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Subject {
    pub player_id: i32,
    /// For the feedback line: the character name, or the typed name.
    pub name: String,
}

/// Why no subject was resolved: a stable `reason` and the GM's line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unresolved {
    pub reason: &'static str,
    pub line: String,
}

/// Resolve `name`, or the caller when there is none, to a `player_id`.
pub(crate) fn resolve_subject(
    mgr: &SpaceManager,
    caller_id: u32,
    cmd: &str,
    name: Option<&str>,
) -> Result<Subject, Unresolved> {
    let Some(name) = name else {
        let caller = mgr.get_entity(caller_id);
        return match caller.and_then(|e| e.player_id) {
            Some(player_id) => Ok(Subject {
                player_id,
                name: caller
                    .and_then(|e| e.character_name.clone())
                    .unwrap_or_else(|| "you".to_string()),
            }),
            None => Err(Unresolved {
                reason: "caller_not_player",
                line: format!(".{cmd}: you have no character id; name a player instead."),
            }),
        };
    };
    let entity_id = match mgr.find_online_player_by_name(name) {
        PlayerNameLookup::Found { entity_id, .. }
        | PlayerNameLookup::InTransition { entity_id } => entity_id,
        PlayerNameLookup::NotFound => {
            return Err(Unresolved {
                reason: "target_not_found",
                line: format!(
                    ".{cmd}: no online player is named {name} (names are case-sensitive)."
                ),
            })
        }
        PlayerNameLookup::Ambiguous { entity_ids } => {
            return Err(Unresolved {
                reason: "target_ambiguous",
                line: format!(
                    ".{cmd}: {} players are named {name}; refusing to guess.",
                    entity_ids.len()
                ),
            })
        }
    };
    match mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(player_id) => Ok(Subject {
            player_id,
            name: name.to_string(),
        }),
        None => Err(Unresolved {
            reason: "target_not_player",
            line: format!(".{cmd}: {name} is not a player character."),
        }),
    }
}

/// The `.duel_status` line for `subject`'s entry at `now`.
pub(crate) fn status_line(mgr: &SpaceManager, subject: &Subject, now: Instant) -> String {
    let who = format!("{} (player {})", subject.name, subject.player_id);
    let other = |player_id: i32| {
        let name = online_name(mgr, player_id).unwrap_or_else(|| "not in the world".to_string());
        format!("{name} (player {player_id})")
    };
    let secs = |at: Instant| at.saturating_duration_since(now).as_secs();
    match status(&mgr.duels, subject.player_id) {
        DuelStatus::Idle => format!("{who} is not in a duel and has no duel challenge."),
        DuelStatus::InDuel(d) => {
            let opponent = d.opponent_of(subject.player_id).unwrap_or(d.target);
            let stage = match d.state {
                DuelState::StartPending { engage_at } => {
                    format!("in the countdown, starting in {}s", secs(engage_at))
                }
                DuelState::Engaged => "fighting".to_string(),
            };
            format!(
                "{who} is in duel #{} with {}, {stage}, in space {}.",
                d.duel_id,
                other(opponent),
                d.space_id
            )
        }
        DuelStatus::Challenged(p) => format!(
            "{who} has duel challenge #{} from {} to answer; it expires in {}s.",
            p.duel_id,
            other(p.challenger),
            secs(p.expires_at)
        ),
        DuelStatus::Challenging(p) => format!(
            "{who} challenged {} (duel #{}); it expires in {}s.",
            other(p.target),
            p.duel_id,
            secs(p.expires_at)
        ),
    }
}

/// `.duel_status [name]`.
pub(super) async fn duel_status(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let subject =
        match resolve_subject(space_mgr, caller_id, "duel_status", parse_duel_status(args)) {
            Ok(s) => s,
            Err(u) => return refuse(caller_id, "duel_status", None, u, tx, space_mgr).await,
        };
    let gm = space_mgr.player_identity(caller_id);
    tracing::debug!(
        target: "duel",
        event = "duel.gm_status",
        account_id = gm.account_id,
        player_id = gm.player_id,
        entity_id = caller_id,
        subject_player_id = subject.player_id,
        "GM read a duel registry entry"
    );
    let line = status_line(space_mgr, &subject, Instant::now());
    send_gm_feedback(caller_id, &format!(".duel_status: {line}"), tx).await;
}

/// `.duel_end <name>`.
pub(super) async fn duel_end(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let name = match parse_duel_end(args) {
        Ok(n) => n,
        Err(usage) => {
            let u = Unresolved {
                reason: "no_name",
                line: usage.to_string(),
            };
            return refuse(caller_id, "duel_end", None, u, tx, space_mgr).await;
        }
    };
    let subject = match resolve_subject(space_mgr, caller_id, "duel_end", Some(name)) {
        Ok(s) => s,
        Err(u) => return refuse(caller_id, "duel_end", None, u, tx, space_mgr).await,
    };
    let Some(aborted) = gm_end(tx, space_mgr, caller_id, subject.player_id).await else {
        let u = Unresolved {
            reason: "nothing_to_end",
            line: format!(
                ".duel_end: {} is not in a duel and has no duel challenge.",
                subject.name
            ),
        };
        return refuse(
            caller_id,
            "duel_end",
            Some(subject.player_id),
            u,
            tx,
            space_mgr,
        )
        .await;
    };
    let (a, b) = aborted.players();
    let name_of = |pid: i32| online_name(space_mgr, pid).unwrap_or_else(|| format!("player {pid}"));
    let line = format!(
        ".duel_end: ended duel #{} ({}) between {} and {}; both were told \"Duel aborted\".",
        aborted.duel_id(),
        aborted.stage(),
        name_of(a),
        name_of(b)
    );
    send_gm_feedback(caller_id, &line, tx).await;
}

/// Log a refused duel command (`duel.gm_rejected`) and tell the GM why.
async fn refuse(
    caller_id: u32,
    cmd: &'static str,
    subject_player_id: Option<i32>,
    u: Unresolved,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let gm = space_mgr.player_identity(caller_id);
    tracing::debug!(
        target: "duel",
        event = "duel.gm_rejected",
        account_id = gm.account_id,
        player_id = gm.player_id,
        entity_id = caller_id,
        subject_player_id,
        command = cmd,
        reason = u.reason,
        "GM duel command refused: nothing was read or changed"
    );
    send_gm_feedback(caller_id, &u.line, tx).await;
}
