//! `.account` (client-tls-oidc C10, CT-45): the GM's sign-in method
//! commands, answered from the identity tables.
//!
//! The cell owns the console and the GM gate but has no DB pool, so it
//! forwards [`AccountCellToBase`] and the base answers here, the same round
//! trip `.bankdump` takes. Output never carries a subject, an email or a
//! link code's log copy: only the provider, the display name and dates.
//!
//! Every outcome is one `gm_action` event on the `account_admin` target,
//! with `action`, `result`, a stable `reason` on refusals, and the GM and
//! target ids. A refusal the GM caused is INFO; a refusal the server caused
//! (no pool, a failed query) is WARN. `revoke_failed`: the sign-ins could not
//! be revoked, so the unlink rolled back and the identity is still linked.
//!
//! [`AccountCellToBase`]: cimmeria_wire::cell::messages::AccountCellToBase

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::{DateTime, Utc};
use cimmeria_identity::link;
use cimmeria_identity::link_code::{self, LinkCodeStore};
use cimmeria_identity::repo::account;
use cimmeria_identity::repo::event::{self, EventKind};
use cimmeria_identity::repo::identity;
use cimmeria_identity::repo::refresh;
use cimmeria_identity::repo::RepoError;
use cimmeria_identity::revoke;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::gm_feedback::send_gm_feedback_to_client;
use super::ConnectedClientState;

/// The longest display name the listing prints, in characters.
const MAX_NAME_CHARS: usize = 32;

/// The GM who asked, from the cell's session state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminCaller {
    /// The GM's entity id; the answer is addressed to it.
    pub entity_id: u32,
    /// The GM's entity name, for the log.
    pub entity_name: String,
    /// The GM's `account.account_id`, `None` if the cell has none.
    pub account_id: Option<u32>,
    /// The GM's `account.account_name`, empty if the cell has none.
    pub gm_account_name: String,
}

/// One `.account` command, with the identity row it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminAction {
    /// `.account identities <name>`.
    Identities,
    /// `.account unlink <name> <identity_id>`.
    Unlink {
        /// The identity row to remove.
        identity_id: i64,
    },
    /// `.account linkcode <name>`.
    LinkCode,
}

impl AdminAction {
    /// The `action` field the log line carries.
    pub fn label(self) -> &'static str {
        match self {
            Self::Identities => "account_identities",
            Self::Unlink { .. } => "account_unlink",
            Self::LinkCode => "account_linkcode",
        }
    }
}

/// Keeps printable ASCII only, drops `"` and caps the length, so a display
/// name the provider chose cannot break the one-line listing.
fn sanitize(name: &str) -> String {
    name.chars()
        .filter(|c| matches!(c, ' '..='~') && *c != '"')
        .take(MAX_NAME_CHARS)
        .collect()
}

/// A Unix-millisecond timestamp as `YYYY-MM-DD` (UTC).
fn date(ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(ms).map_or_else(
        || "unknown".to_owned(),
        |t| t.format("%Y-%m-%d").to_string(),
    )
}

/// Writes the `gm_action` line for one outcome. Never the code, a subject,
/// an email or a display name (C11).
fn log_outcome(
    caller: &AdminCaller,
    action: &'static str,
    ok: bool,
    reason: Option<&str>,
    target: &str,
    target_account_id: Option<u32>,
    identity_id: Option<i64>,
) {
    let result = if ok { "ok" } else { "refused" };
    tracing::info!(
        target: "account_admin",
        event = "gm_action",
        action,
        result,
        reason,
        gm_account_id = caller.account_id,
        gm_account_name = caller.gm_account_name.as_str(),
        entity_id = caller.entity_id,
        entity_name = caller.entity_name.as_str(),
        target_account_id,
        target_account_name = target,
        identity_id, // nt:id-only identity row id, no name exists
        "account command answered"
    );
}

/// What [`remove_identity`] did.
enum Removal {
    /// The account would be left with no sign-in method.
    LastMethod,
    /// The identity row is not this account's.
    NotLinked,
    /// The row was removed; carries its provider.
    Removed(String),
    /// The sign-ins could not be revoked, so nothing was removed (C16.6).
    RevokeFailed,
}

/// Removes one identity in a single transaction that locks the account row
/// first, so two unlinks cannot both pass the last-method check (C15 e);
/// the unlink event commits with the delete, and so do the revoke of the
/// account's refresh families and the unlink event (C16.6). Mirrors
/// `remove_identity` in `cimmeria_identity::routes::account`.
async fn remove_identity(
    pool: &PgPool,
    account_id: u32,
    identity_id: i64,
) -> Result<Removal, RepoError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT account_id FROM account WHERE account_id = $1 FOR UPDATE")
        .bind(account_id as i32)
        .fetch_optional(&mut *tx)
        .await?;
    let (methods,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM account_identity WHERE account_id = $1")
            .bind(account_id as i32)
            .fetch_one(&mut *tx)
            .await?;
    // A password-only account has no identity row to remove: not a last method.
    if methods == 0 {
        return Ok(Removal::NotLinked);
    }
    if methods <= 1 {
        return Ok(Removal::LastMethod);
    }
    let removed: Option<(String,)> = sqlx::query_as(
        "DELETE FROM account_identity WHERE account_id = $1 AND identity_id = $2 \
         RETURNING provider",
    )
    .bind(account_id as i32)
    .bind(identity_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((provider,)) = removed else {
        return Ok(Removal::NotLinked);
    };
    event::record_tx(
        &mut tx,
        account_id,
        Some(identity_id),
        EventKind::Unlink,
        Some(&provider),
    )
    .await?;
    // A failed revoke returns before the commit: dropping `tx` rolls the
    // delete back, so the identity is still there and the GM is told so.
    if let Err(err) = refresh::revoke_account_on(&mut tx, account_id, None, Utc::now()).await {
        tracing::warn!(
            target: "account_admin",
            event = "gm_action",
            action = "account_unlink",
            result = "refused",
            reason = "revoke_failed",
            target_account_id = account_id,
            identity_id,
            error = %err,
            "unlink rolled back: the account's refresh families could not be revoked"
        );
        return Ok(Removal::RevokeFailed);
    }
    tx.commit().await?;
    Ok(Removal::Removed(provider))
}

/// Answers one command as the lines the GM reads, logging its outcome. With
/// no pool the answer is the `db_unavailable` line.
pub async fn run(
    caller: &AdminCaller,
    target: &str,
    action: AdminAction,
    pool: Option<&PgPool>,
    codes: &LinkCodeStore,
    now: Instant,
) -> Vec<String> {
    let Some(pool) = pool else {
        tracing::warn!(
            target: "account_admin",
            event = "gm_action",
            action = action.label(),
            result = "refused",
            reason = "db_unavailable",
            gm_account_id = caller.account_id,
            gm_account_name = caller.gm_account_name.as_str(),
            entity_id = caller.entity_id,
            entity_name = caller.entity_name.as_str(),
            target_account_name = target,
            "account command could not reach the identity tables"
        );
        return vec!["account: the database is not available".to_owned()];
    };
    match answer(caller, target, action, pool, codes, now).await {
        Ok(lines) => lines,
        Err(error) => {
            tracing::warn!(
                target: "account_admin",
                event = "gm_action",
                action = action.label(),
                result = "refused",
                reason = "db_error",
                error = %error,
                gm_account_id = caller.account_id,
                gm_account_name = caller.gm_account_name.as_str(),
                entity_id = caller.entity_id,
                entity_name = caller.entity_name.as_str(),
                target_account_name = target,
                "account command could not read the identity tables"
            );
            vec![
                "account: the identity tables could not be read or updated; check the result with .account identities <name> and see the server log".to_owned(),
            ]
        }
    }
}

/// The database half of [`run`]: looks the account up, then acts on it.
async fn answer(
    caller: &AdminCaller,
    target: &str,
    action: AdminAction,
    pool: &PgPool,
    codes: &LinkCodeStore,
    now: Instant,
) -> Result<Vec<String>, RepoError> {
    let name = action.label();
    let Some(facts) = account::facts_by_name(pool, target).await? else {
        log_outcome(
            caller,
            name,
            false,
            Some("unknown_account"),
            target,
            None,
            None,
        );
        return Ok(vec![format!("account: no account named {target}")]);
    };
    let account_id = facts.account_id;
    let ids = Some(account_id);
    match action {
        AdminAction::Identities => {
            let rows = identity::list_for_account(pool, account_id).await?;
            let password = if account::has_password(pool, account_id).await? {
                "set"
            } else {
                "none"
            };
            let mut lines = vec![format!(
                "account {target} (id {account_id}): {} sign-in methods; password {password}",
                rows.len()
            )];
            if rows.is_empty() {
                lines.push("  (no linked sign-in methods)".to_owned());
            }
            for row in rows {
                lines.push(format!(
                    "  #{} {} \"{}\" added {} last used {}",
                    row.identity_id,
                    row.provider,
                    sanitize(&row.display_name),
                    date(row.created_at),
                    date(row.last_used_at)
                ));
            }
            log_outcome(caller, name, true, None, target, ids, None);
            Ok(lines)
        }
        AdminAction::Unlink { identity_id } => {
            match remove_identity(pool, account_id, identity_id).await? {
                Removal::LastMethod => {
                    log_outcome(
                        caller,
                        name,
                        false,
                        Some("last_method"),
                        target,
                        ids,
                        Some(identity_id),
                    );
                    Ok(vec![format!(
                        "account: refusing to remove the last sign-in method of {target}"
                    )])
                }
                Removal::NotLinked => {
                    log_outcome(
                        caller,
                        name,
                        false,
                        Some("not_linked"),
                        target,
                        ids,
                        Some(identity_id),
                    );
                    Ok(vec![format!(
                        "account: identity #{identity_id} is not linked to {target}"
                    )])
                }
                Removal::RevokeFailed => {
                    log_outcome(
                        caller,
                        name,
                        false,
                        Some("revoke_failed"),
                        target,
                        ids,
                        Some(identity_id),
                    );
                    Ok(vec![format!(
                        "account: could not remove identity #{identity_id} from {target}; nothing changed, see the server log"
                    )])
                }
                Removal::Removed(provider) => {
                    // The refresh rows were revoked in the delete's transaction
                    // (C16.6); the access tokens and launch tickets in memory end
                    // after the commit.
                    revoke::end_live_sessions(account_id);
                    log_outcome(caller, name, true, None, target, ids, Some(identity_id));
                    Ok(vec![format!(
                        "account: removed identity #{identity_id} ({provider}) from {target}; its launcher sign-ins are revoked; a game session already running is not ended"
                    )])
                }
            }
        }
        AdminAction::LinkCode => {
            if !link::linkable(&facts) {
                log_outcome(caller, name, false, Some("not_linkable"), target, ids, None);
                return Ok(vec![format!(
                    "account: {target} is a staff, test or disabled account and cannot be linked (not_linkable)"
                )]);
            }
            let code = codes.issue(account_id, now);
            event::record(pool, account_id, None, EventKind::LinkCodeIssued, None).await?;
            let mut lines = vec![
                format!("account: link code for {target}: {code}"),
                "account: valid 15 minutes, single use, replaces any older code. Give it to the player privately; they enter it in the launcher under Link with a code.".to_owned(),
            ];
            if account::has_password(pool, account_id).await? {
                lines.push(format!(
                    "account: {target} has a password; linking with this code removes it for good (D-CT11)."
                ));
            }
            log_outcome(caller, name, true, None, target, ids, None);
            Ok(lines)
        }
    }
}

/// Runs one command and sends each line to the GM (the `.bankdump` shape).
pub async fn handle(
    caller: AdminCaller,
    target: String,
    action: AdminAction,
    pool: Option<&PgPool>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let lines = run(
        &caller,
        &target,
        action,
        pool,
        link_code::shared(),
        Instant::now(),
    )
    .await;
    for line in lines {
        send_gm_feedback_to_client(
            caller.entity_id,
            &line,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
}

#[cfg(test)]
#[path = "tests_live_db.rs"]
mod tests_live_db;
