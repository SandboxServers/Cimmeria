//! Account dispatch arm for `CellToBaseMsg::Account` (client-tls-oidc C10).
//!
//! Every `AccountCellToBase` variant lands here. The handler reads the
//! identity tables and answers the GM; it lives in `base::account_admin`.

use crate::base::account_admin::{handle, AdminAction, AdminCaller};
use crate::cell::messages::AccountCellToBase;

use super::DispatchCtx;

/// Route one `.account` message from the cell.
pub(super) async fn route(msg: AccountCellToBase, ctx: &DispatchCtx<'_>) {
    let (caller, target, action) = match msg {
        AccountCellToBase::Identities {
            entity_id,
            account_id,
            account_name,
        } => (
            AdminCaller {
                entity_id,
                account_id,
            },
            account_name,
            AdminAction::Identities,
        ),
        AccountCellToBase::Unlink {
            entity_id,
            account_id,
            account_name,
            identity_id,
        } => (
            AdminCaller {
                entity_id,
                account_id,
            },
            account_name,
            AdminAction::Unlink { identity_id },
        ),
        AccountCellToBase::LinkCode {
            entity_id,
            account_id,
            account_name,
        } => (
            AdminCaller {
                entity_id,
                account_id,
            },
            account_name,
            AdminAction::LinkCode,
        ),
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
