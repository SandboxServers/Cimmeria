//! Focused tests for click expectations, range reporting, and arguments.

use super::*;
use serde_json::json;

#[test]
fn pitch_restore_counts_undo_the_offset_change() {
    let start = json!({ "pitch_offset_deg": 0.0, "pitch_gain": 20.0 });
    let now = json!({ "pitch_offset_deg": -66.0, "pitch_gain": 20.0 });
    assert_eq!(restore_pitch_counts(&start, &now), 601);
    let inverted = json!({ "pitch_offset_deg": -66.0, "pitch_gain": -20.0 });
    assert_eq!(restore_pitch_counts(&start, &inverted), -601);
    assert_eq!(restore_pitch_counts(&start, &json!({})), 0);
}

fn snap(target: u32, windows: &[&str]) -> UiSnap {
    UiSnap {
        target,
        mouse_over: 0,
        windows: windows.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn expectations_read_target_and_windows() {
    let before = snap(0, &["ChatWin"]);
    let targeted = snap(42, &["ChatWin"]);
    let opened = snap(0, &["ChatWin", "DialogWin"]);
    assert!(met(Expect::Target, Some(42), &before, &targeted));
    assert!(!met(Expect::Target, Some(7), &before, &targeted));
    assert!(met(Expect::Window, Some(42), &before, &opened));
    assert!(!met(Expect::Window, Some(42), &before, &targeted));
    assert!(met(Expect::Any, Some(42), &before, &opened));
    assert!(met(Expect::Nothing, Some(42), &before, &before));
}

#[test]
fn a_far_right_click_gets_a_range_hint() {
    assert!(range_hint(1, Some(6.4)).contains("6.4 m away"));
    assert!(range_hint(1, Some(6.4)).contains("5 m interact range"));
    assert_eq!(range_hint(0, Some(6.4)), "");
    assert_eq!(range_hint(1, Some(4.2)), "");
    assert_eq!(range_hint(1, None), "");
    assert_eq!(range_json(Some(4.2))["in_interact_range"], true);
    assert_eq!(range_json(Some(6.4))["in_interact_range"], false);
    assert!(range_json(None).is_null());
}

#[test]
fn buttons_parse_by_name() {
    assert_eq!(button_index(None, 1), Ok(1));
    assert_eq!(button_index(Some("Left"), 1), Ok(0));
    assert!(button_index(Some("middle"), 1).is_err());
}
