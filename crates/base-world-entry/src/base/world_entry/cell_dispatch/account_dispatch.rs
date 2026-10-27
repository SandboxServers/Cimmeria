//! Account dispatch arm for `CellToBaseMsg::Account` (client-tls-oidc C10).
//!
//! Every `AccountCellToBase` variant lands here. The handler reads the
//! identity tables and answers the GM; it lives in `base::account_admin`.
//! The cell is the only producer and gates the GM, but the base does not
//! trust the message alone: a sender below the GM level, or a message with no
//! account, is refused here (client-tls-oidc D3).

use cimmeria_base_session::base::world_entry::gm_only_worlds::GM_ACCESS_LEVEL;

use crate::base::account_admin::{handle, AdminAction, AdminCaller};
use crate::cell::messages::AccountCellToBase;

use super::DispatchCtx;

/// The GM's session state the base checks a command against.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct GmSession {
    /// The session's access level; a GM is at [`GM_ACCESS_LEVEL`] or above.
    access_level: u32,
    /// The GM's `account.account_name`, empty if the session has none.
    account_name: String,
    /// The GM's active character name, empty if the session has none.
    player_name: String,
}

/// Route one `.account` message from the cell.
pub(super) async fn route(msg: AccountCellToBase, ctx: &DispatchCtx<'_>) {
    let (entity_id, account_id, target, action) = match msg {
        AccountCellToBase::Identities {
            entity_id,
            account_id,
            account_name,
        } => (entity_id, account_id, account_name, AdminAction::Identities),
        AccountCellToBase::Unlink {
            entity_id,
            account_id,
            account_name,
            identity_id,
        } => (
            entity_id,
            account_id,
            account_name,
            AdminAction::Unlink { identity_id },
        ),
        AccountCellToBase::LinkCode {
            entity_id,
            account_id,
            account_name,
        } => (entity_id, account_id, account_name, AdminAction::LinkCode),
    };
    let gm = gm_session(ctx, entity_id);
    if let Some(reason) = refusal(&gm, account_id) {
        refuse(&gm, entity_id, account_id, action, reason);
        return;
    }
    let caller = AdminCaller {
        entity_id,
        entity_name: gm.player_name,
        account_id,
        gm_account_name: gm.account_name,
    };
    handle(
        caller,
        target,
        action,
        ctx.db_pool.as_deref(),
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
    )
    .await
}

/// The session state of the GM entity `entity_id`. With no session the
/// access level is zero, so the command is refused.
fn gm_session(ctx: &DispatchCtx<'_>, entity_id: u32) -> GmSession {
    let addr = ctx
        .entity_to_addr
        .lock()
        .ok()
        .and_then(|map| map.get(&entity_id).copied());
    let (Some(addr), Ok(clients)) = (addr, ctx.connected.lock()) else {
        return GmSession::default();
    };
    clients
        .get(&addr)
        .map_or_else(GmSession::default, |c| GmSession {
            access_level: c.access_level,
            account_name: c.account_name.clone().unwrap_or_default(),
            player_name: c.player_name.clone().unwrap_or_default(),
        })
}

/// Why the base refuses a command from `gm`, or `None` when it may run: the
/// sender must be a GM and the message must name an account.
fn refusal(gm: &GmSession, account_id: Option<u32>) -> Option<&'static str> {
    if gm.access_level < GM_ACCESS_LEVEL {
        Some("not_gm")
    } else if account_id.is_none() {
        Some("no_account")
    } else {
        None
    }
}

/// Logs a refused command as the `gm_action` WARN line. The GM gets no answer.
fn refuse(
    gm: &GmSession,
    entity_id: u32,
    account_id: Option<u32>,
    action: AdminAction,
    reason: &'static str,
) {
    tracing::warn!(
        target: "account_admin",
        event = "gm_action",
        action = action.label(),
        result = "refused",
        reason,
        gm_account_id = account_id,
        gm_account_name = gm.account_name.as_str(),
        entity_id,
        entity_name = gm.player_name.as_str(),
        "account command refused by the base"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gm(access_level: u32) -> GmSession {
        GmSession {
            access_level,
            ..GmSession::default()
        }
    }

    #[test]
    fn a_gm_with_an_account_may_run_the_command() {
        assert_eq!(refusal(&gm(GM_ACCESS_LEVEL), Some(6)), None);
    }

    #[test]
    fn a_sender_below_the_gm_level_is_refused_not_gm() {
        assert_eq!(refusal(&gm(0), Some(6)), Some("not_gm"));
        assert_eq!(refusal(&gm(GM_ACCESS_LEVEL - 1), Some(6)), Some("not_gm"));
    }

    #[test]
    fn a_missing_session_is_refused_not_gm() {
        assert_eq!(refusal(&GmSession::default(), Some(6)), Some("not_gm"));
    }

    #[test]
    fn a_message_with_no_account_is_refused() {
        assert_eq!(refusal(&gm(GM_ACCESS_LEVEL), None), Some("no_account"));
    }
}
