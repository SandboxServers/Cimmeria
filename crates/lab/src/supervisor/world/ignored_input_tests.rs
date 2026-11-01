//! AX-34: ignored input versus a blocking window.

use super::ignored_input::{classify, look_delivered, HUD_WINDOWS};
use super::movement::{self, MoveRequest};
use super::sim::Sim;
use super::tool_tests::client_point;
use crate::proof::stop::StopKind;

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_dialog_makes_it_blocked_ui() {
    assert_eq!(
        classify(&names(&["DialogWin"]), HUD_WINDOWS),
        StopKind::BlockedUi
    );
    // A dialog among HUD windows still blocks.
    assert_eq!(
        classify(&names(&["SelfStatusWin", "DialogWin"]), HUD_WINDOWS),
        StopKind::BlockedUi
    );
}

#[test]
fn only_hud_is_ignored_input() {
    assert_eq!(
        classify(&names(&["SelfStatusWin", "ChatWin"]), HUD_WINDOWS),
        StopKind::IgnoredInput
    );
    assert_eq!(classify(&[], HUD_WINDOWS), StopKind::IgnoredInput);
}

#[test]
fn counters_that_did_not_move_deny_delivery() {
    let c = look_delivered(Some(5), Some(5));
    assert_eq!(c.ok, Some(false));
    assert_eq!(c.why, Some(crate::proof::Why::NotDelivered));
    assert_eq!(look_delivered(Some(5), Some(9)).ok, Some(true));
}

#[test]
fn no_counters_is_unmeasured() {
    assert_eq!(look_delivered(None, None).ok, None);
    assert_eq!(look_delivered(Some(5), None).ok, None);
}

async fn dead_w_stop(windows: &[&str]) -> serde_json::Value {
    let mut sim = Sim::live();
    sim.speed_uu_s = 0.0;
    sim.windows = names(windows);
    let e = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(2500.0, 0.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "input_probe");
    e.to_json()
}

#[tokio::test]
async fn sim_probe_with_dialog_stops_blocked_ui() {
    let v = dead_w_stop(&["SelfStatusWin", "DialogWin"]).await;
    assert_eq!(v["stop"]["kind"], "blocked_ui");
    assert_eq!(v["stop"]["suggest"], serde_json::json!(["close_dialogs"]));
}

#[tokio::test]
async fn sim_probe_with_only_hud_stops_ignored_input() {
    let v = dead_w_stop(&["SelfStatusWin", "ChatWin"]).await;
    assert_eq!(v["stop"]["kind"], "ignored_input");
}
