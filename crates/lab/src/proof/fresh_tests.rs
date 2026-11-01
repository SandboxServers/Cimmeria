use serde_json::json;

use super::*;

#[test]
fn freshness_rules() {
    assert_eq!(freshness(Some(10), Some(12)), Freshness::Fresh);
    assert_eq!(freshness(Some(10), Some(11)), Freshness::Stale);
    assert_eq!(freshness(None, Some(5)), Freshness::Unstamped);
    assert_eq!(freshness(Some(5), None), Freshness::Unstamped);
}

#[test]
fn mark_unstamped_wraps_scalars() {
    assert_eq!(
        mark_unstamped(json!(4711)),
        json!({ "value": 4711, "unstamped": true })
    );
    assert_eq!(
        mark_unstamped(json!({ "x": 1 })),
        json!({ "x": 1, "unstamped": true })
    );
}
