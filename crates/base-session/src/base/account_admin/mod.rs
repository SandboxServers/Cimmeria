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
//! (no pool, a failed query) is WARN.
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
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::gm_feedback::send_gm_feedback_to_client;
use super::ConnectedClientState;

/// The longest display name the listing prints, in characters.
const MAX_NAME_CHARS: usize = 32;

/// The GM who asked, from the cell's session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdminCaller {
    /// The GM's entity id; the answer is addressed to it.
    pub entity_id: u32,
    /// The GM's `account.account_id`, `None` if the cell has none.
    pub account_id: Option<u32>,
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
    fn label(self) -> &'static str {
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
        entity_id = caller.entity_id,
        target_account_id,
        identity_id,
        "account command answered"
    );
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
            entity_id = caller.entity_id,
            "account command could not reach the identity tables"
        );
        return vec!["account: the database is not available".to_owned()];
    };
    match answer(caller, target, action, pool, codes, now).await {
        Ok(lines) => lines,
        Err(_) => {
            tracing::warn!(
                target: "account_admin",
                event = "gm_action",
                action = action.label(),
                result = "refused",
                reason = "db_error",
                gm_account_id = caller.account_id,
                entity_id = caller.entity_id,
                "account command could not read the identity tables"
            );
            vec!["account: the identity tables could not be read; see the server log".to_owned()]
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
        log_outcome(caller, name, false, Some("unknown_account"), None, None);
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
            log_outcome(caller, name, true, None, ids, None);
            Ok(lines)
        }
        AdminAction::Unlink { identity_id } => {
            if identity::count_methods(pool, account_id).await? <= 1 {
                log_outcome(
                    caller,
                    name,
                    false,
                    Some("last_method"),
                    ids,
                    Some(identity_id),
                );
                return Ok(vec![format!(
                    "account: refusing to remove the last sign-in method of {target}"
                )]);
            }
            // The provider is read before the delete, for the event row and
            // the reply.
            let provider = identity::list_for_account(pool, account_id)
                .await?
                .into_iter()
                .find(|row| row.identity_id == identity_id)
                .map(|row| row.provider);
            let removed = match &provider {
                Some(_) => identity::delete(pool, account_id, identity_id).await?,
                None => false,
            };
            let (true, Some(provider)) = (removed, provider) else {
                log_outcome(
                    caller,
                    name,
                    false,
                    Some("not_linked"),
                    ids,
                    Some(identity_id),
                );
                return Ok(vec![format!(
                    "account: identity #{identity_id} is not linked to {target}"
                )]);
            };
            event::record(
                pool,
                account_id,
                Some(identity_id),
                EventKind::Unlink,
                Some(&provider),
            )
            .await?;
            // A wrongly linked identity must lose the sign-ins it made.
            refresh::revoke_account(pool, account_id, Utc::now()).await?;
            log_outcome(caller, name, true, None, ids, Some(identity_id));
            Ok(vec![format!(
                "account: removed identity #{identity_id} ({provider}) from {target}; its launcher sign-ins are revoked"
            )])
        }
        AdminAction::LinkCode => {
            if !link::linkable(&facts) {
                log_outcome(caller, name, false, Some("not_linkable"), ids, None);
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
            log_outcome(caller, name, true, None, ids, None);
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
