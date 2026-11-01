//! Tests for the C1 evidence types and the C2 reason codes.

use serde_json::json;

use super::{Check, EvVerdict, Evidence, Source, Why};

fn check(name: &str, ok: Option<bool>, required: bool, why: Option<Why>) -> Check {
    let c = Check::new(name, Source::Client, ok, required);
    match why {
        Some(w) => c.with_why(w),
        None => c,
    }
}

#[test]
fn a_required_false_check_denies_with_its_why() {
    let ev = Evidence::judge(vec![
        check("optional_false", Some(false), false, None),
        check("hover_matches", Some(true), true, Some(Why::NoHover)),
        check("in_range", Some(false), true, Some(Why::OutOfRange)),
    ]);
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::OutOfRange);
    assert_eq!(ev.hint, Why::OutOfRange.hint());
    assert!(!ev.counts());
}

#[test]
fn a_required_unmeasured_check_is_inconclusive() {
    let ev = Evidence::judge(vec![
        check("moved", Some(true), true, None),
        check("hp_after", None, true, None),
    ]);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::NoPostRead);
}

#[test]
fn no_required_checks_is_no_evidence() {
    let ev = Evidence::judge(vec![check("optional", Some(true), false, None)]);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::NoEvidence);
}

#[test]
fn all_required_true_confirms() {
    let ev = Evidence::judge(vec![
        check("a", Some(true), true, None),
        check("b", Some(true), true, None),
    ]);
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    assert_eq!(ev.why, Why::Confirmed);
    assert_eq!(ev.hint, Why::Confirmed.hint());
    assert!(ev.counts());
}

#[test]
fn evidence_round_trips_without_ok() {
    let c = Check::new("hover_matches", Source::Client, None, true)
        .with_before(json!(0))
        .with_after(json!(4711))
        .with_frame(81234)
        .with_why(Why::NoHover);
    let v = serde_json::to_value(&c).unwrap();
    assert!(v.get("ok").is_none(), "a null ok is dropped: {v}");
    let back: Check = serde_json::from_value(v).unwrap();
    assert_eq!(back, c);
    assert_eq!(back.ok, None);

    let ev = Evidence::from_verdict(EvVerdict::Inconclusive, Why::NoPostRead, vec![c]);
    assert_eq!(Evidence::from_value(&ev.to_value()), Some(ev));
    assert_eq!(Evidence::from_value(&json!({"verdict": "nope"})), None);
}

#[test]
fn why_serde_matches_as_str() {
    assert_eq!(Why::ALL.len(), 18);
    for w in Why::ALL {
        assert_eq!(serde_json::to_value(w).unwrap(), json!(w.as_str()));
        assert!(!w.hint().is_empty(), "{} has no hint", w.as_str());
    }
}

#[test]
fn unjudged_and_no_evidence_are_fixed_verdicts() {
    let u = Evidence::unjudged();
    assert_eq!((u.verdict, u.why), (EvVerdict::Unjudged, Why::NotYetJudged));
    let n = Evidence::no_evidence();
    assert_eq!(
        (n.verdict, n.why),
        (EvVerdict::Inconclusive, Why::NoEvidence)
    );
    assert!(!u.counts() && !n.counts());
}
