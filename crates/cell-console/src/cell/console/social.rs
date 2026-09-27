//! Social console commands: `.announce [space] <text>`, the GM broadcast for
//! a client without the native `/gmshout` binding (SS-C2, D-SS16), and the
//! chat mutes `.mute <name> <minutes> [reason]` / `.unmute <name>` (SS-C3,
//! D-SS26).
//!
//! It sends exactly what `/gmshout` (`sendGMShout`, CM 222) sends, through
//! the same [`gm::shout::broadcast`]: the GM's name, the GM speaker flag,
//! the server channel. Without a scope word the line goes to every online
//! player; a first word of `space` (any case) keeps it to the GM's space
//! instance. To announce a line that itself starts with the word "space",
//! put any other word first.
//!
//! The console splits a line on whitespace, so the text is re-joined with
//! single spaces: runs of spaces in the typed line collapse to one.
//!
//! `.mute` and `.unmute` are parsed and bounded here, then handed to the
//! base (`ChatCellToBase::Mute` / `Unmute`): only the base sees every online
//! character and holds the mute table, and it answers the GM.

use cimmeria_entity::organization::org_text::{validate, TextField};
use tokio::sync::mpsc;

use super::gm::shout::{broadcast, ShoutScope};
use super::send_gm_feedback;
use crate::cell::messages::{CellToBaseMsg, ChatCellToBase, MAX_MUTE_MINUTES};
use crate::cell::space_manager::SpaceManager;

/// Split `.announce`'s positional args into a scope and the text.
pub(crate) fn parse_announce(args: &[&str]) -> Result<(ShoutScope, String), &'static str> {
    let (scope, words) = match args.split_first() {
        Some((first, rest)) if first.eq_ignore_ascii_case("space") => (ShoutScope::Space, rest),
        _ => (ShoutScope::Global, args),
    };
    if words.is_empty() {
        return Err(".announce: nothing to announce. Usage: .announce [space] <text>");
    }
    Ok((scope, words.join(" ")))
}

/// `.announce [space] <text>`.
pub(super) async fn announce(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    match parse_announce(args) {
        Ok((scope, text)) => {
            broadcast(caller_id, scope, &text, "console", tx, space_mgr).await;
        }
        Err(usage) => {
            let id = space_mgr.player_identity(caller_id);
            tracing::warn!(
                target: "chat",
                event = "chat.gm_broadcast_rejected",
                entity_id = caller_id,
                account_id = id.account_id,
                player_id = id.player_id,
                source = "console",
                reason = "no_text",
                "GM broadcast rejected: .announce had no text",
            );
            send_gm_feedback(caller_id, usage, tx).await;
        }
    }
}

/// `.mute`'s arguments, bounded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MuteArgs {
    pub name: String,
    pub minutes: u32,
    /// Empty when the GM gave none.
    pub reason: String,
}

const MUTE_USAGE: &str = ".mute: Usage: .mute <name> <minutes> [reason]";
const UNMUTE_USAGE: &str = ".unmute: Usage: .unmute <name>";

/// Split `.mute <name> <minutes> [reason]`. `Err` is `(reason, GM line)`.
pub(crate) fn parse_mute(args: &[&str]) -> Result<MuteArgs, (&'static str, String)> {
    let [name, minutes, rest @ ..] = args else {
        return Err(("usage", MUTE_USAGE.to_string()));
    };
    let minutes = match minutes.parse::<u32>() {
        Ok(m) if (1..=MAX_MUTE_MINUTES).contains(&m) => m,
        _ => {
            return Err((
                "bad_duration",
                format!(".mute: minutes must be a whole number from 1 to {MAX_MUTE_MINUTES}."),
            ))
        }
    };
    let reason = rest.join(" ");
    // The reason reaches the base and its log, so it gets the chat-line
    // rules (at most 255 units, no control, bidi or format characters).
    if !reason.is_empty() && validate(TextField::ChatText, &reason).is_err() {
        return Err((
            "bad_reason",
            ".mute: the reason is too long or has a character that cannot be sent.".to_string(),
        ));
    }
    Ok(MuteArgs {
        name: (*name).to_string(),
        minutes,
        reason,
    })
}

/// Split `.unmute <name>`.
pub(crate) fn parse_unmute(args: &[&str]) -> Result<String, (&'static str, String)> {
    match args {
        [name] => Ok((*name).to_string()),
        _ => Err(("usage", UNMUTE_USAGE.to_string())),
    }
}

/// `.mute <name> <minutes> [reason]`.
pub(super) async fn mute(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(caller_id);
    match parse_mute(args) {
        Ok(m) => {
            let msg = ChatCellToBase::Mute {
                entity_id: caller_id,
                player_id: id.player_id,
                account_id: id.account_id,
                target_name: m.name,
                minutes: m.minutes,
                reason: m.reason,
            };
            forward(caller_id, "mute", msg, tx).await;
        }
        Err((reason, line)) => {
            tracing::warn!(
                target: "chat",
                event = "chat.gm_mute_refused",
                entity_id = caller_id,
                account_id = id.account_id,
                player_id = id.player_id,
                source = "console",
                reason,
                "GM .mute refused on the cell: bad arguments, the GM was told",
            );
            send_gm_feedback(caller_id, &line, tx).await;
        }
    }
}

/// `.unmute <name>`.
pub(super) async fn unmute(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(caller_id);
    match parse_unmute(args) {
        Ok(name) => {
            let msg = ChatCellToBase::Unmute {
                entity_id: caller_id,
                player_id: id.player_id,
                account_id: id.account_id,
                target_name: name,
            };
            forward(caller_id, "unmute", msg, tx).await;
        }
        Err((reason, line)) => {
            tracing::warn!(
                target: "chat",
                event = "chat.gm_unmute_refused",
                entity_id = caller_id,
                account_id = id.account_id,
                player_id = id.player_id,
                source = "console",
                reason,
                "GM .unmute refused on the cell: bad arguments, the GM was told",
            );
            send_gm_feedback(caller_id, &line, tx).await;
        }
    }
}

/// Hand a mute command to the base, which answers the GM. A closed channel
/// means the base is gone; nothing can reach the GM then either, so it is
/// only logged.
async fn forward(
    caller_id: u32,
    command: &'static str,
    msg: ChatCellToBase,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    if tx.send(CellToBaseMsg::Chat(msg)).await.is_err() {
        tracing::warn!(
            target: "chat",
            event = "chat.gm_mute_refused",
            entity_id = caller_id,
            command,
            reason = "base_channel_closed",
            "GM mute command not delivered: the cell-to-base channel is closed",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No scope word: global, text re-joined with single spaces.
    #[test]
    fn announce_parse_defaults_to_global() {
        assert_eq!(
            parse_announce(&["Server", "restart", "soon"]),
            Ok((ShoutScope::Global, "Server restart soon".to_string()))
        );
    }

    /// A leading `space` (any case) selects the space scope and is not part
    /// of the text.
    #[test]
    fn announce_parse_space_keyword_selects_space_scope() {
        assert_eq!(
            parse_announce(&["SPACE", "Event", "here"]),
            Ok((ShoutScope::Space, "Event here".to_string()))
        );
        // Only the first word is a scope word.
        assert_eq!(
            parse_announce(&["The", "space", "gate"]),
            Ok((ShoutScope::Global, "The space gate".to_string()))
        );
    }

    /// `.mute` bounds the minutes, re-joins the reason and refuses a bad one.
    #[test]
    fn mute_parse_bounds_minutes_and_reason() {
        assert_eq!(
            parse_mute(&["Loudmouth", "30", "spamming", "trade"]),
            Ok(MuteArgs {
                name: "Loudmouth".into(),
                minutes: 30,
                reason: "spamming trade".into(),
            })
        );
        assert_eq!(parse_mute(&["Loudmouth", "5"]).unwrap().reason, "");
        assert_eq!(parse_mute(&["Loudmouth", "10080"]).unwrap().minutes, 10080);
        for bad in ["0", "-1", "ten", "10081"] {
            assert_eq!(
                parse_mute(&["Loudmouth", bad]).unwrap_err().0,
                "bad_duration",
                "{bad}"
            );
        }
        assert_eq!(parse_mute(&["Loudmouth"]).unwrap_err().0, "usage");
        assert_eq!(parse_mute(&[]).unwrap_err().0, "usage");
        assert_eq!(
            parse_mute(&["Loudmouth", "5", "a\u{202e}b"]).unwrap_err().0,
            "bad_reason"
        );
    }

    #[test]
    fn unmute_parse_takes_exactly_one_name() {
        assert_eq!(parse_unmute(&["Loudmouth"]), Ok("Loudmouth".to_string()));
        assert_eq!(parse_unmute(&[]).unwrap_err().0, "usage");
        assert_eq!(parse_unmute(&["a", "b"]).unwrap_err().0, "usage");
    }

    /// `space` with nothing after it has no text to send.
    #[test]
    fn announce_parse_rejects_scope_without_text() {
        assert!(parse_announce(&["space"]).is_err());
        assert!(parse_announce(&[]).is_err());
    }
}
