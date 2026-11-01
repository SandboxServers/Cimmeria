//! The evidence a world click carries (AX-31): the rule table, then two
//! `Sim` runs through `click::run`.

use serde_json::json;

use super::click::{self, ClickRequest, Expect, UiSnap};
use super::click_proof::{checks, ClickFacts};
use super::geometry::{client_to_server, Vec3};
use super::sim::Sim;
use super::PointArg;
use crate::proof::claims::{claim, Claim};
use crate::proof::fresh::Freshness;
use crate::proof::{EvVerdict, Evidence, Why};

fn snap(target: u32, windows: &[&str]) -> UiSnap {
    UiSnap {
        target,
        mouse_over: 0,
        windows: windows.iter().map(|s| s.to_string()).collect(),
    }
}

fn judge(
    expect: Expect,
    button: u8,
    range_m: Option<f64>,
    after_target: u32,
    fresh: Freshness,
) -> Evidence {
    let (before, after) = (snap(0, &["ChatWin"]), snap(after_target, &["ChatWin"]));
    Evidence::judge(checks(&ClickFacts {
        expect,
        entity: Some(42),
        before: &before,
        after: &after,
        hover_verified: true,
        button,
        range_m,
        fresh,
        read_frame: Some(30),
    }))
}

#[test]
fn target_met_is_confirmed() {
    let ev = judge(Expect::Target, 0, None, 42, Freshness::Fresh);
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    assert_eq!(ev.checks[0].name, "target_is_entity");
    assert_eq!(ev.checks[0].frame, Some(30));
}

#[test]
fn target_missed_is_denied_no_effect() {
    let ev = judge(Expect::Target, 0, None, 7, Freshness::Fresh);
    assert_eq!((ev.verdict, ev.why), (EvVerdict::Denied, Why::NoEffect));
}

#[test]
fn expect_nothing_is_inconclusive() {
    let ev = judge(Expect::Nothing, 0, None, 0, Freshness::Fresh);
    assert_eq!(
        (ev.verdict, ev.why),
        (EvVerdict::Inconclusive, Why::NoPostRead)
    );
}

#[test]
fn far_right_click_is_denied_out_of_range() {
    let ev = judge(Expect::Nothing, 1, Some(9.0), 0, Freshness::Fresh);
    assert_eq!((ev.verdict, ev.why), (EvVerdict::Denied, Why::OutOfRange));
    assert_eq!(ev.checks[0].name, "in_range");
}

#[test]
fn near_right_click_has_optional_range() {
    let ev = judge(Expect::Target, 1, Some(3.0), 42, Freshness::Fresh);
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    let range = ev.checks.iter().find(|c| c.name == "in_range").unwrap();
    assert!(!range.required);
    assert_eq!(range.ok, Some(true));
}

#[test]
fn far_left_click_has_no_range_check() {
    let ev = judge(Expect::Target, 0, Some(9.0), 42, Freshness::Fresh);
    assert!(ev.checks.iter().all(|c| c.name != "in_range"));
}

#[test]
fn stale_read_is_inconclusive() {
    let ev = judge(Expect::Target, 0, None, 42, Freshness::Stale);
    assert_eq!(
        (ev.verdict, ev.why),
        (EvVerdict::Inconclusive, Why::StaleRead)
    );
}

#[test]
fn unstamped_marks_after() {
    let ev = judge(Expect::Target, 0, None, 42, Freshness::Unstamped);
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    let after = ev.checks[0].after.as_ref().unwrap();
    assert_eq!(after["unstamped"], json!(true));
}

#[test]
fn window_and_any_read_the_opened_list() {
    let (before, after) = (snap(0, &["ChatWin"]), snap(0, &["ChatWin", "DialogWin"]));
    for (expect, name) in [
        (Expect::Window, "window_opened"),
        (Expect::Any, "effect_seen"),
    ] {
        let ev = Evidence::judge(checks(&ClickFacts {
            expect,
            entity: Some(42),
            before: &before,
            after: &after,
            hover_verified: false,
            button: 0,
            range_m: None,
            fresh: Freshness::Fresh,
            read_frame: None,
        }));
        assert_eq!(ev.verdict, EvVerdict::Confirmed, "{name}");
        assert_eq!(ev.checks[0].name, name);
        let hover = ev.checks.last().unwrap();
        assert_eq!(
            (hover.name.as_str(), hover.required),
            ("hover_verified", false)
        );
    }
}

#[test]
fn claims_lists_click_tools() {
    assert_eq!(claim("client_world_click"), Claim::Judged);
    assert_eq!(claim("client_target"), Claim::Judged);
}

fn medic() -> Sim {
    let mut s = Sim::new().with_entity(11, "Medic Ogilvie", Vec3::new(400.0, 0.0, 0.0));
    s.entities[0].window = Some("DialogWin".into());
    s.frames = Some(0);
    s
}

#[tokio::test]
async fn sim_world_click_carries_evidence() {
    let mut sim = medic();
    let req = ClickRequest {
        entity_id: Some(11),
        ..ClickRequest::default()
    };
    let out = click::run(&mut sim, "client_target", req, Some((0, Expect::Target)))
        .await
        .unwrap();
    assert_eq!(out["evidence"]["verdict"], "confirmed", "{out}");
    assert_eq!(out["counts_as_native_pass"], true, "{out}");
}

#[tokio::test]
async fn sim_point_click_is_inconclusive() {
    let mut sim = medic();
    let p = client_to_server(Vec3::new(400.0, 0.0, 0.0));
    let req = ClickRequest {
        point: Some(PointArg {
            x: p.x,
            y: p.y,
            z: p.z,
            space: None,
        }),
        ..ClickRequest::default()
    };
    let out = click::run(&mut sim, "client_world_click", req, None)
        .await
        .unwrap();
    assert_eq!(out["evidence"]["verdict"], "inconclusive", "{out}");
    assert_eq!(out["evidence"]["why"], "no_post_read", "{out}");
}
