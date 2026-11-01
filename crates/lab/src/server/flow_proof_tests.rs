use serde_json::json;

use super::*;
use crate::proof::EvVerdict;

#[test]
fn a_judged_flow_confirms_on_its_end_screen() {
    let ev = flow_evidence("lab_play_character", &json!({"at": "world"}));
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    let check = &ev.checks[0];
    assert_eq!(check.name, "end_state");
    assert_eq!(check.ok, Some(true));
    assert!(check.required);
    assert_eq!(check.after, Some(json!({"screen": "world_hud"})));
}

#[test]
fn every_judged_flow_names_its_screen() {
    for (tool, screen) in [
        ("lab_login", "character_select"),
        ("lab_create_character", "character_select"),
        ("lab_delete_character", "character_select"),
        ("lab_ensure_character_slot", "character_select"),
        ("lab_play_character", "world_hud"),
        ("lab_finish_dialog", "dialog_closed"),
        ("lab_logout", "character_select"),
        ("lab_disconnect", "login_screen"),
    ] {
        assert_eq!(end_screen(tool), Some(screen), "{tool}");
    }
}

#[test]
fn lab_characters_stays_out_of_the_flow_evidence() {
    assert_eq!(end_screen("lab_characters"), None);
}

#[test]
fn in_world_true_confirms_with_the_world_and_position() {
    let v = json!({"in_world": true, "world_id": 7, "pos": [1.0, 2.0, 3.0]});
    let ev = in_world_evidence(&v);
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    assert_eq!(ev.checks[0].name, "in_world");
    assert_eq!(
        ev.checks[0].after,
        Some(json!({"world_id": 7, "pos": [1.0, 2.0, 3.0]}))
    );
}

#[test]
fn a_stop_at_result_confirms_instead_of_denying() {
    let ev = in_world_evidence(&json!({"at": "character_select", "steps_ms": {}}));
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    assert_eq!(ev.checks[0].name, "reached_stop");
    assert_eq!(ev.checks[0].after, Some(json!({"at": "character_select"})));
}

#[test]
fn in_world_false_denies() {
    let ev = in_world_evidence(&json!({"in_world": false, "at": "character_select"}));
    assert_eq!(ev.verdict, EvVerdict::Denied);
}
