use std::time::Duration;

use serde_json::json;

use super::{action_checks, sequence_evidence};
use crate::proof::{EvVerdict, Why};
use crate::supervisor::composite::sequence::Action;
use crate::supervisor::ui::type_into::typed_evidence;

fn click(window: &str) -> Action {
    Action::Click {
        window: window.into(),
        button: 0,
    }
}

fn wait(window: &str) -> Action {
    Action::WaitWindow {
        window: window.into(),
        gone: false,
        timeout: Duration::from_secs(2),
    }
}

/// Every action's checks, in order, judged as one sequence.
fn judge(steps: &[(Action, Result<serde_json::Value, String>)]) -> crate::proof::Evidence {
    let checks = steps
        .iter()
        .enumerate()
        .flat_map(|(i, (a, r))| action_checks(i, a, r, None))
        .collect();
    sequence_evidence(checks)
}

/// Bare clicks and keys prove no effect: nothing required, `posted_only`.
#[test]
fn bare_clicks_are_posted_only() {
    let key = Action::Key {
        key: "B".into(),
        action: "tap".into(),
        hold_ms: None,
    };
    let ev = judge(&[
        (click("Dialog_Done"), Ok(json!(true))),
        (key, Ok(json!(true))),
    ]);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::PostedOnly);
}

/// A click followed by a wait that found its window confirms the sequence,
/// and the window check names the wait's position.
#[test]
fn click_then_wait_window_confirms() {
    let steps = [
        (click("VendorBtn"), Ok(json!(true))),
        (wait("VendorWin"), Ok(json!({ "ms": 120 }))),
    ];
    let ev = judge(&steps);
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    let c = ev.checks.iter().find(|c| c.name == "window_1").unwrap();
    assert_eq!(c.before, Some(json!({ "action": 1 })));
    assert_eq!(c.after.as_ref().unwrap()["window"], "VendorWin");
}

/// A failed step denies the sequence, with its error as the reading.
#[test]
fn a_failed_step_denies() {
    let err = "VendorWin not visible after 2000 ms".to_string();
    let ev = judge(&[
        (click("VendorBtn"), Ok(json!(true))),
        (wait("VendorWin"), Err(err.clone())),
    ]);
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::NoEffect);
    let c = ev.checks.iter().find(|c| c.name == "action_1").unwrap();
    assert_eq!(c.after, Some(json!(err)));
}

/// A typing step with `into` contributes its edit box's `text_entered`
/// check, renamed `typed_<index>`.
#[test]
fn a_typed_box_is_checked_as_typed() {
    let typed = Action::Type {
        text: "hi".into(),
        into: Some("ChatEdit".into()),
    };
    let result =
        json!({ "evidence": typed_evidence(Some("ChatEdit"), "hi", Some("hi")).to_value() });
    let ev = judge(&[(typed, Ok(result))]);
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    assert!(ev.checks.iter().any(|c| c.name == "typed_0" && c.required));
}
