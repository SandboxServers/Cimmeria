//! `Steps::finish` reports judged evidence and lets it decide the native pass.

use serde_json::json;

use super::{NativeLevel, Steps};
use crate::proof::{Check, Evidence, Source};

fn denied() -> Evidence {
    Evidence::judge(vec![Check::new(
        "hover_matches",
        Source::Client,
        Some(false),
        true,
    )])
}

#[test]
fn finish_with_denied_evidence_does_not_count() {
    let mut s = Steps::new("client_target", 0);
    s.used(NativeLevel::RealInput);
    s.judged(denied());
    let out = s.finish(10, json!({}));
    assert_eq!(out["counts_as_native_pass"], json!(false));
    assert_eq!(out["evidence"]["verdict"], json!("denied"));
    assert_eq!(out["evidence"]["why"], json!("no_effect"));
}

#[test]
fn finish_without_judged_has_no_evidence_key() {
    let mut s = Steps::new("client_target", 0);
    s.used(NativeLevel::RealInput);
    let out = s.finish(10, json!({}));
    assert_eq!(out["counts_as_native_pass"], json!(true));
    assert!(out.get("evidence").is_none());
}
