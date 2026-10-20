//! `grant_cash` row -> [`Action::GrantCash`] (UB-50b1, U-D07).
//! `params` is `{"amount": N}`, N an integer 1 to 2147483647.
//! `target_id` and `target_key` are unused. A missing, non-integer or
//! out-of-range amount drops the row with a `warn!` naming the chain:
//! a defaulted 0 would pay nothing on every firing, silently.

use tracing::warn;

use super::DbActionRow;
use crate::actions::Action;

/// Convert one `grant_cash` `content_actions` row.
pub(super) fn convert_grant_cash(row: &DbActionRow) -> Option<Action> {
    let amount = row
        .params
        .get("amount")
        .and_then(serde_json::Value::as_i64)
        .and_then(|n| i32::try_from(n).ok())
        .filter(|n| *n >= 1);
    if amount.is_none() {
        warn!(
            chain_id = row.chain_id,
            chain_name = cimmeria_names::book().chain(row.chain_id),
            params = ?row.params,
            "grant_cash: `amount` must be an integer 1 to 2147483647; dropping the action row"
        );
    }
    amount.map(|amount| Action::GrantCash { amount })
}
