use super::*;
use crate::proof::{EvVerdict, Why};

#[test]
fn a_refusal_is_an_answer() {
    // D-AX14: DA-U22 expects a refused press to pass the action.
    for verdict in ["refused", "refused_client_side"] {
        let ev = ability_evidence(verdict, true);
        assert_eq!(ev.verdict, EvVerdict::Confirmed, "{verdict}");
        assert_eq!(ev.checks[0].after.as_ref().unwrap()["verdict"], verdict);
    }
}

#[test]
fn cast_without_result_is_inconclusive() {
    let ev = ability_evidence("cast_started", true);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::NoPostRead);
}

#[test]
fn nothing_observed_is_ignored() {
    let ev = ability_evidence("nothing_observed", false);
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::Ignored);
}

#[test]
fn an_unknown_verdict_is_no_evidence() {
    let ev = ability_evidence("something_new", true);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::NoEvidence);
}
