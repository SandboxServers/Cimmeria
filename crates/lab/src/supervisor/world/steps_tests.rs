//! `Steps::finish` reports judged evidence and lets it decide the native pass.

use serde_json::json;

use super::{NativeLevel, Steps};
use crate::proof::stop::{StopKind, StopPlayer, StopReport};
use crate::proof::{Check, Evidence, Source};

fn stuck_report() -> StopReport {
    StopReport {
        kind: StopKind::Stuck,
        step: "walk".to_string(),
        seconds: 4.0,
        tried: vec!["steer".to_string()],
        player: Some(StopPlayer::new([0.0, 0.0, 0.0], [3.0, 4.0, 0.0])),
        suggest: vec!["unstick".to_string()],
    }
}

#[test]
fn fail_stop_serialises_the_report() {
    let s = Steps::new("client_move_to", 0);
    let e = s.fail_stop(10, "walk", "no progress", stuck_report());
    let v = e.to_json();
    assert_eq!(v["stop"]["kind"], json!("stuck"));
    assert_eq!(v["stop"]["suggest"], json!(["unstick"]));
    assert_eq!(v["stop"]["player"]["moved_m"], json!(5.0));
}

#[test]
fn plain_fail_has_no_stop_key() {
    let s = Steps::new("client_move_to", 0);
    let e = s.fail(10, "walk", "no progress");
    assert!(e.to_json().get("stop").is_none());
}

#[test]
fn message_names_the_stop_kind() {
    let s = Steps::new("client_move_to", 0);
    let e = s.fail_stop(10, "walk", "no progress", stuck_report());
    assert!(e.summary().ends_with("; stop: stuck"));
}

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
