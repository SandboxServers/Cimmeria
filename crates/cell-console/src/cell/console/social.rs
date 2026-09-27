//! Social console commands: `.announce [space] <text>`, the GM broadcast for
//! a client without the native `/gmshout` binding (SS-C2, D-SS16).
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

use tokio::sync::mpsc;

use super::gm::shout::{broadcast, ShoutScope};
use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
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

    /// `space` with nothing after it has no text to send.
    #[test]
    fn announce_parse_rejects_scope_without_text() {
        assert!(parse_announce(&["space"]).is_err());
        assert!(parse_announce(&[]).is_err());
    }
}
