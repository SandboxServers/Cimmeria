//! The one place a refused persistence call is logged.
//!
//! Every public function in `persistence` runs its body through
//! [`observed`]: a success logs its own DEBUG `event` (with `rows_affected`
//! and any before/after values) from inside the body, and a refusal logs
//! exactly one WARN here, on target `org`, with the function's name as
//! `event` and the [`OrgStoreError::reason`] as `reason`
//! (`docs/architecture/negative-logging-convention.md`).
//!
//! These are persistence-level transitions written inside the caller's
//! transaction. The handler that owns the action logs its own INFO outcome
//! row after deciding to commit.

use std::future::Future;

use super::OrgStoreError;

/// Await `body`, logging a WARN if it is refused.
pub(super) async fn observed<T>(
    event: &'static str,
    org_id: Option<i32>,
    player_id: Option<i32>,
    body: impl Future<Output = Result<T, OrgStoreError>>,
) -> Result<T, OrgStoreError> {
    let result = body.await;
    if let Err(e) = &result {
        match e {
            OrgStoreError::Db(db) => tracing::warn!(
                target: "org",
                event,
                org_id, // nt:id-only shared refusal helper receives ids only, the handler's outcome row names the org
                player_id, // nt:id-only shared refusal helper receives ids only, the handler's outcome row names the member
                reason = e.reason(),
                error = %db,
                "Organization write failed"
            ),
            _ => tracing::warn!(
                target: "org",
                event,
                org_id, // nt:id-only shared refusal helper receives ids only, the handler's outcome row names the org
                player_id, // nt:id-only shared refusal helper receives ids only, the handler's outcome row names the member
                reason = e.reason(),
                "Organization write refused"
            ),
        }
    }
    result
}

/// UTF-16 length, the unit every organization text cap uses (D-ORG10).
pub(super) fn units(text: &str) -> usize {
    text.encode_utf16().count()
}
