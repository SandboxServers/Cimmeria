//! `.account` — the GM's sign-in methods tool (client-tls-oidc C10).
//!
//! `.account identities <account>` lists an account's sign-in methods,
//! `.account unlink <account> <identity_id>` removes one, and
//! `.account linkcode <account>` issues a one-time link code. GM-only, like
//! every `.`-command. The cell has no DB pool, so it checks the arguments
//! and forwards the command (`AccountCellToBase::{Identities, Unlink,
//! LinkCode}`) to the base, which reads the identity tables and answers
//! on the GM's feedback channel.
//!
//! A typed account name is checked here by hand (3 to 20 characters of
//! ASCII letters, digits, `_` or `-`), so a typo never reaches the DB. The
//! code a link command issues is printed to the GM once and is never
//! logged; neither the typed name nor the base's answer is logged here.
//!
//! A non-GM who types `.account` gets the generic refusal line instead of
//! having it broadcast as chat ([`log_non_gm_account`], called from the chat
//! interceptor).

use tokio::sync::mpsc;
use tracing::{info, warn};

use super::send_gm_feedback;
use crate::cell::messages::{AccountCellToBase, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;

/// The usage line for any malformed `.account` command.
const USAGE: &str =
    "usage: .account identities <account> | unlink <account> <identity_id> | linkcode <account>";

/// One parsed `.account` line, before the GM's entity ids are attached.
#[derive(Debug, PartialEq, Eq)]
enum Parsed {
    Identities(String),
    Unlink(String, i64),
    LinkCode(String),
}

/// `.account <sub> ...`: parse the line, then forward it to the base. A line
/// that does not parse is answered on the feedback channel and sends
/// nothing.
pub(super) async fn run(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let parsed = match parse(args) {
        Ok(parsed) => parsed,
        Err(text) => {
            send_gm_feedback(caller_id, text, tx).await;
            return;
        }
    };
    let id = space_mgr.player_identity(caller_id);
    let (action, cmd) = match parsed {
        Parsed::Identities(account_name) => (
            "account_identities",
            AccountCellToBase::Identities {
                entity_id: caller_id,
                account_id: id.account_id,
                account_name,
            },
        ),
        Parsed::Unlink(account_name, identity_id) => (
            "account_unlink",
            AccountCellToBase::Unlink {
                entity_id: caller_id,
                account_id: id.account_id,
                account_name,
                identity_id,
            },
        ),
        Parsed::LinkCode(account_name) => (
            "account_linkcode",
            AccountCellToBase::LinkCode {
                entity_id: caller_id,
                account_id: id.account_id,
                account_name,
            },
        ),
    };
    if let Err(e) = tx.send(CellToBaseMsg::Account(cmd)).await {
        warn!(
            target: "account_admin",
            event = "gm_action",
            action,
            result = "refused",
            reason = "base_channel_closed",
            account_id = id.account_id,
            account_name = id.account_name,
            entity_id = caller_id,
            entity_name = id.player_name,
            error = %e,
            "gm_action: account could not reach the base",
        );
    }
}

/// Parse the arguments after `.account`. `Err` is the feedback line.
fn parse(args: &[&str]) -> Result<Parsed, &'static str> {
    let sub = args.first().map(|s| s.to_ascii_lowercase());
    let parsed = match (sub.as_deref(), args) {
        (Some("identities"), [_, name]) => Parsed::Identities(checked_name(name)?),
        (Some("linkcode"), [_, name]) => Parsed::LinkCode(checked_name(name)?),
        (Some("unlink"), [_, name, id]) => {
            let account_name = checked_name(name)?;
            Parsed::Unlink(account_name, checked_identity_id(id)?)
        }
        _ => return Err(USAGE),
    };
    Ok(parsed)
}

/// `<account>`: 3 to 20 characters of ASCII letters, digits, `_` or `-`.
fn checked_name(name: &str) -> Result<String, &'static str> {
    let ok = (3..=20).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok {
        Ok(name.to_string())
    } else {
        Err("account: not a valid account name")
    }
}

/// `<identity_id>`: a positive `i64`.
fn checked_identity_id(id: &str) -> Result<i64, &'static str> {
    match id.parse::<i64>() {
        Ok(value) if value > 0 => Ok(value),
        _ => Err("account: identity id must be a positive number"),
    }
}

/// Is `text` a `.account` line? Matched on the command word only.
pub(crate) fn is_account_command(text: &str) -> bool {
    text.strip_prefix('.')
        .and_then(|body| body.split_whitespace().next())
        .is_some_and(|name| name.eq_ignore_ascii_case("account"))
}

/// A non-GM typed `.account`: `gm_action result=refused reason=not_gm` on
/// the `account_admin` target. The caller still sends the generic "is a GM
/// command" line, and the text is never broadcast.
pub(crate) fn log_non_gm_account(entity_id: u32, space_mgr: &SpaceManager) {
    let id = space_mgr.player_identity(entity_id);
    info!(
        target: "account_admin",
        event = "gm_action",
        action = "account",
        result = "refused",
        reason = "not_gm",
        account_id = id.account_id,
        account_name = id.account_name,
        entity_id,
        entity_name = id.player_name,
        "gm_action: account refused",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_subcommand_case_insensitively() {
        assert_eq!(
            parse(&["Identities", "Player_1234"]),
            Ok(Parsed::Identities("Player_1234".to_string()))
        );
        assert_eq!(
            parse(&["unlink", "Player_1234", "42"]),
            Ok(Parsed::Unlink("Player_1234".to_string(), 42))
        );
        assert_eq!(
            parse(&["LINKCODE", "Player_1234"]),
            Ok(Parsed::LinkCode("Player_1234".to_string()))
        );
    }

    #[test]
    fn rejects_bad_names_ids_and_usage() {
        assert_eq!(
            parse(&["identities", "a"]),
            Err("account: not a valid account name")
        );
        assert_eq!(
            parse(&["identities", "x y"]),
            Err("account: not a valid account name")
        );
        assert_eq!(
            parse(&["identities", "abcdefghijklmnopqrstu"]),
            Err("account: not a valid account name")
        );
        assert_eq!(
            parse(&["unlink", "Player_1234", "0"]),
            Err("account: identity id must be a positive number")
        );
        assert_eq!(
            parse(&["unlink", "Player_1234", "-3"]),
            Err("account: identity id must be a positive number")
        );
        assert_eq!(parse(&["ban", "Player_1234"]), Err(USAGE));
        assert_eq!(parse(&[]), Err(USAGE));
    }

    #[test]
    fn is_account_command_matches_word_only() {
        assert!(is_account_command(".account x"));
        assert!(is_account_command(".ACCOUNT"));
        assert!(!is_account_command(".accountx"));
        assert!(!is_account_command("account x"));
    }
}
