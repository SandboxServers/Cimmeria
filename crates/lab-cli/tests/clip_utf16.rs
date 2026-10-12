//! LX-18c1 (F-LX11): the golden value budget counts UTF-16 units, as the
//! PowerShell's `Limit-UatText` did.

use cimmeria_lab_cli::commands::uat::golden::{golden_value, VALUE_MAX};
use serde_json::json;

fn diverged(first: &str) -> String {
    golden_value(
        Some(&json!({"ok": true, "match": false, "first": first})),
        None,
    )
}

#[test]
fn a_divergence_with_emoji_is_cut_by_units_not_characters() {
    assert_eq!(VALUE_MAX, 300);
    // 297 x + 2 emoji = 299 characters but 301 units: over the budget.
    let first = format!("{}\u{1F600}\u{1F600}", "x".repeat(297));
    assert_eq!(diverged(&first), format!("{}...", "x".repeat(297)));
}

#[test]
fn a_divergence_of_exactly_the_budget_in_units_is_kept() {
    // 298 x + 1 emoji = 300 units.
    let first = format!("{}\u{1F600}", "x".repeat(298));
    assert_eq!(diverged(&first), first);
}

#[test]
fn an_ascii_divergence_keeps_its_budget() {
    let cut = diverged(&"y".repeat(400));
    assert_eq!(cut.len(), 300);
    assert_eq!(cut, format!("{}...", "y".repeat(297)));
}
