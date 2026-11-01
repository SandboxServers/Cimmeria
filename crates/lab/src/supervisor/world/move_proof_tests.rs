//! Pure cases for the move proof: no `Sim`, plain positions.

use super::{arrival_checks, stop_report};
use crate::proof::stop::StopKind;
use crate::proof::{EvVerdict, Evidence, Why};
use crate::supervisor::world::geometry::{client_to_server, Vec3};

const GOAL: Vec3 = Vec3::new(3000.0, 0.0, 0.0);
const START: Vec3 = Vec3::new(0.0, 0.0, 0.0);

fn judge(
    end_client: Vec3,
    arrival: f64,
    start_server: Option<Vec3>,
    end_server: Option<Vec3>,
) -> Evidence {
    Evidence::judge(arrival_checks(
        GOAL,
        START,
        end_client,
        arrival,
        start_server,
        end_server,
    ))
}

#[test]
fn both_agree_confirms() {
    let at_goal = client_to_server(GOAL);
    let ev = judge(GOAL, 1.5, Some(client_to_server(START)), Some(at_goal));
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    assert!(ev.checks.iter().any(|c| c.name == "no_rubber_band"));
}

#[test]
fn server_far_is_denied() {
    // 6 m off along the server's z (client x).
    let far = Vec3::new(0.0, 0.0, 30.0 - 6.0);
    let ev = judge(GOAL, 1.5, Some(client_to_server(START)), Some(far));
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::ServerDisagrees);
}

#[test]
fn server_inside_the_slack_confirms() {
    // 2.2 m off with arrival 1.5: inside arrival + 1.0.
    let near = Vec3::new(0.0, 0.0, 30.0 - 2.2);
    let ev = judge(GOAL, 1.5, Some(client_to_server(START)), Some(near));
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
}

#[test]
fn no_server_is_inconclusive() {
    let ev = judge(GOAL, 1.5, None, None);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::ServerUnreachable);
    assert!(
        !ev.checks.iter().any(|c| c.name == "no_rubber_band"),
        "a rubber band needs both server reads"
    );
}

#[test]
fn client_short_of_the_goal_is_denied() {
    let ev = judge(
        Vec3::new(1000.0, 0.0, 0.0),
        1.5,
        None,
        Some(client_to_server(GOAL)),
    );
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::NoEffect);
}

#[test]
fn rubber_band_is_denied() {
    // The client walked 30 m; the server's player never left its start.
    let s = client_to_server(GOAL);
    let ev = judge(GOAL, 1.5, Some(s), Some(s));
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::ServerDisagrees);
    let c = ev
        .checks
        .iter()
        .find(|c| c.name == "no_rubber_band")
        .unwrap();
    assert_eq!(c.ok, Some(false));
}

#[test]
fn a_short_client_walk_is_not_a_rubber_band() {
    // Already in range: the client moved 0.2 m, the server did not move.
    let end = Vec3::new(2980.0, 0.0, 0.0);
    let s = client_to_server(GOAL);
    let ev = Evidence::judge(arrival_checks(GOAL, GOAL, end, 1.5, Some(s), Some(s)));
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    assert!(!ev.checks.iter().any(|c| c.name == "no_rubber_band"));
}

#[test]
fn stuck_stop_report() {
    let r = stop_report(
        StopKind::Stuck,
        "leg_0",
        4.5,
        vec!["jump".into(), "input_probe".into()],
        START,
        Vec3::new(1000.0, 0.0, 0.0),
    );
    assert_eq!(r.kind, StopKind::Stuck);
    assert_eq!(r.suggest[0], "unstick");
    assert_eq!(r.suggest, ["unstick", "release_input"]);
    let p = r.player.unwrap();
    // 1000 client units are 10 m, and the client x axis is the server's z.
    assert!((p.moved_m - 10.0).abs() < 1e-9, "{}", p.moved_m);
    assert!((p.now[2] - 10.0).abs() < 1e-9);
}

#[test]
fn suggestions_per_kind() {
    let s = |k| stop_report(k, "x", 0.0, vec![], START, START).suggest;
    assert_eq!(
        s(StopKind::IgnoredInput),
        ["release_input", "close_dialogs"]
    );
    assert_eq!(s(StopKind::Timeout), ["unstick"]);
    assert_eq!(s(StopKind::Stalled), ["unstick"]);
    assert_eq!(s(StopKind::BlockedUi), ["close_dialogs"]);
    assert!(s(StopKind::Ledge).is_empty());
}
