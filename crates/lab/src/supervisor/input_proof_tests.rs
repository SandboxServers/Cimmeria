//! Input verdicts over plain values: no bridge, no client (AX-43).

use serde_json::json;

use super::*;

fn motion(before: Option<u64>, after: Option<u64>) -> MouseFacts {
    MouseFacts {
        dx: 5,
        dy: 0,
        wheel: 0,
        button: false,
        cursor_placed: None,
        di_before: before,
        di_after: after,
    }
}

#[test]
fn a_key_is_posted_only() {
    let ev = key_evidence();
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::PostedOnly);
    assert!(ev.checks.is_empty());
}

#[test]
fn a_button_is_posted_only_even_with_motion() {
    let mut m = motion(Some(10), Some(12));
    m.button = true;
    let ev = mouse_evidence(m);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::PostedOnly);
}

#[test]
fn di_motion_read_is_delivered() {
    let ev = mouse_evidence(motion(Some(10), Some(12)));
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::Delivered);
    assert_eq!(ev.checks[0].name, "di_delivered");
    assert_eq!(ev.checks[0].ok, Some(true));
    assert_eq!(ev.checks[0].before, Some(json!(10)));
    assert_eq!(ev.checks[0].after, Some(json!(12)));
}

#[test]
fn di_count_is_records_handed_over() {
    assert_eq!(
        di_count(&json!({"hooks": {"events_delivered": 3, "data_reads_mouse": 900}})),
        Some(3)
    );
    assert_eq!(di_count(&json!({"hooks": null})), None);
}

#[test]
fn di_motion_unread_is_not_delivered() {
    let ev = mouse_evidence(motion(Some(10), Some(10)));
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::NotDelivered);
}

#[test]
fn di_motion_without_a_counter_is_posted_only() {
    let ev = mouse_evidence(motion(None, None));
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::PostedOnly);
}

#[test]
fn cursor_move_confirms_on_read_back() {
    let placed = cursor_evidence(true, (100, 200));
    assert_eq!(placed.verdict, EvVerdict::Confirmed);
    assert_eq!(placed.checks[0].after, Some(json!([100, 200])));
    let not = cursor_evidence(false, (100, 200));
    assert_eq!(not.verdict, EvVerdict::Denied);
}

#[test]
fn cursor_only_mouse_move_reads_back() {
    let m = MouseFacts {
        cursor_placed: Some(true),
        ..MouseFacts::default()
    };
    assert_eq!(mouse_evidence(m).verdict, EvVerdict::Confirmed);
}

#[test]
fn focus_mismatch_denies() {
    assert_eq!(focus_evidence(true, None).verdict, EvVerdict::Denied);
    assert_eq!(focus_evidence(true, Some(42)).verdict, EvVerdict::Confirmed);
    assert_eq!(focus_evidence(false, None).verdict, EvVerdict::Confirmed);
}

#[test]
fn release_with_a_hold_left_denies() {
    let held = json!({"keys": [], "buttons": []});
    let left = json!([{"vk": 87}]);
    assert_eq!(
        release_evidence(&held, Some(&left)).verdict,
        EvVerdict::Denied
    );
    let empty = json!([]);
    assert_eq!(
        release_evidence(&held, Some(&empty)).verdict,
        EvVerdict::Confirmed
    );
    assert_eq!(release_evidence(&held, None).verdict, EvVerdict::Confirmed);
}

#[test]
fn release_with_a_key_still_held_denies() {
    let held = json!({"keys": [87], "buttons": []});
    assert_eq!(release_evidence(&held, None).verdict, EvVerdict::Denied);
}

#[test]
fn release_without_a_held_reading_is_unmeasured() {
    let ev = release_evidence(&json!(null), None);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::NoPostRead);
}
