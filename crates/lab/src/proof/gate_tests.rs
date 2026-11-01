//! Tests for the evidence gate rules (C3 a-f).

use serde_json::{json, Value};

use super::{stamp, stamp_with, verdict_of};
use crate::proof::claims::Claim;
use crate::proof::{Check, Evidence, Source, Why};

/// Evidence in the wire shape: a confirmed result read back by the client.
fn confirmed() -> Value {
    Evidence::judge(vec![Check::new(
        "hover_matches",
        Source::Client,
        Some(true),
        true,
    )])
    .to_value()
}

#[test]
fn errors_pass_untouched() {
    let mut v = json!({"error": "x", "effect_ok": true});
    stamp("client_world_click", &mut v);
    assert_eq!(v, json!({"error": "x", "effect_ok": true}));
}

#[test]
fn no_claim_tools_pass_untouched() {
    let mut v = json!({"output": 1, "effect_ok": true});
    stamp("client_lua_eval", &mut v);
    assert_eq!(v, json!({"output": 1, "effect_ok": true}));
}

#[test]
fn judged_without_evidence_is_no_evidence() {
    let mut v = json!({"effect_ok": true});
    stamp_with(Claim::Judged, &mut v);
    assert_eq!(v["evidence"]["why"], "no_evidence");
    assert_eq!(v["evidence"]["verdict"], "inconclusive");
    assert_eq!(verdict_of(&v), Some("inconclusive"));
    assert!(v.get("effect_ok").is_none());
}

#[test]
fn unjudged_gets_unjudged_and_keeps_effect_ok() {
    let mut v = json!({"effect_ok": true, "counts_as_native_pass": true});
    stamp_with(Claim::Unjudged, &mut v);
    assert_eq!(v["evidence"]["verdict"], "unjudged");
    assert_eq!(v["evidence"]["why"], "not_yet_judged");
    assert_eq!(v["effect_ok"], true);
    assert_eq!(v["counts_as_native_pass"], false);
    assert_eq!(v["claimed"]["counts_as_native_pass"], true);
}

#[test]
fn denied_flips_effect_ok() {
    let denied = Evidence::judge(vec![Check::new(
        "in_range",
        Source::Client,
        Some(false),
        true,
    )
    .with_why(Why::OutOfRange)])
    .to_value();
    let mut v = json!({"effect_ok": true, "evidence": denied});
    stamp_with(Claim::Judged, &mut v);
    assert_eq!(v["effect_ok"], false);
    assert_eq!(v["claimed"]["effect_ok"], true);
    assert_eq!(verdict_of(&v), Some("denied"));
}

#[test]
fn inconclusive_removes_effect_ok() {
    let mut v = json!({"effect_ok": true, "evidence": Evidence::no_evidence().to_value()});
    stamp_with(Claim::Judged, &mut v);
    assert!(v.get("effect_ok").is_none());
    assert_eq!(v["claimed"]["effect_ok"], true);
}

#[test]
fn ui_native_pass_is_downgraded_too() {
    let mut v = json!({"native_pass": true, "native_level": "real_input"});
    stamp_with(Claim::Unjudged, &mut v);
    assert_eq!(v["native_pass"], false);
    assert_eq!(v["claimed"]["native_pass"], true);
    assert_eq!(v["native_level"], "real_input");
}

#[test]
fn confirmed_keeps_counts() {
    let mut v = json!({"counts_as_native_pass": true, "evidence": confirmed()});
    stamp_with(Claim::Judged, &mut v);
    assert_eq!(v["counts_as_native_pass"], true);
    assert_eq!(v["verdict"], "confirmed");
    assert!(v.get("claimed").is_none());
}
