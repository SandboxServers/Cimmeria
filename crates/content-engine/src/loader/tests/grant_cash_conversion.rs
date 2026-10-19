//! `grant_cash` row -> [`Action::GrantCash`] conversion (UB-50b1).
//!
//! The loader is the boundary: a missing or out-of-range amount drops the
//! row here rather than paying 0 on every firing. No database.

use super::super::action::convert_action;
use super::super::*;
use crate::actions::Action;

fn cash_row(params: serde_json::Value) -> DbActionRow {
    DbActionRow {
        chain_id: 23480,
        action_type: "grant_cash".to_string(),
        target_id: None,
        target_key: None,
        params,
        delay_ms: 0,
        sort_order: 1,
    }
}

/// A positive integer amount converts, including the `i32` ceiling.
/// Revert: remove the `"grant_cash"` arm, and the row becomes unknown (`None`).
#[test]
fn grant_cash_converts_an_amount() {
    assert_eq!(
        convert_action(&cash_row(serde_json::json!({"amount": 60}))),
        Some(Action::GrantCash { amount: 60 })
    );
    assert_eq!(
        convert_action(&cash_row(serde_json::json!({"amount": 2147483647i64}))),
        Some(Action::GrantCash { amount: i32::MAX })
    );
}

/// A missing, zero, negative, overflowing, string or fractional amount drops
/// the row. Revert: replace the filter with an `unwrap_or(0)` default.
#[test]
fn grant_cash_drops_a_bad_amount() {
    for params in [
        serde_json::json!({}),
        serde_json::json!({"amount": 0}),
        serde_json::json!({"amount": -1}),
        serde_json::json!({"amount": 2147483648i64}),
        serde_json::json!({"amount": "60"}),
        serde_json::json!({"amount": 60.5}),
    ] {
        assert_eq!(convert_action(&cash_row(params.clone())), None, "{params}");
    }
}
