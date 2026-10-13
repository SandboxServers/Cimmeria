//! The avatar standing between the camera and the target, and targets that
//! hover nothing near the avatar (#1660 item 7).

use serde_json::Value;

use super::click::{self, ClickRequest, Expect};
use super::geometry::{client_to_server, Vec3};
use super::sim::{Sim, REAL_BODY_DZ};
use super::PointArg;

/// The `hover` step's detail from a result or an error's steps.
fn hover_detail(steps: &[Value]) -> &Value {
    &steps
        .iter()
        .find(|s| s["step"] == "hover")
        .expect("a hover step")["detail"]
}

/// A medic 10 m ahead with a window, hidden behind the avatar while the
/// camera looks within about 17 degrees of it.
fn medic_behind_the_avatar() -> Sim {
    let mut s = Sim::new().with_entity(11, "Medic Ogilvie", Vec3::new(1000.0, 0.0, 0.0));
    s.entities[0].window = Some("DialogWin".into());
    s.avatar_hides = Some((11, 0.3));
    s
}

fn point(pos: Vec3) -> PointArg {
    let p = client_to_server(pos);
    PointArg {
        x: p.x,
        y: p.y,
        z: p.z,
        space: None,
    }
}

#[tokio::test]
async fn point_on_entity_covered_by_own_avatar_uses_entity_hover_search() {
    let mut sim = medic_behind_the_avatar();
    let out = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            point: Some(point(Vec3::new(1000.0, 0.0, 0.0))),
            expect: Some(Expect::Window),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap();
    assert_eq!(out["hover_verified"], true, "{out}");
    assert_eq!(out["entity"]["id"], 11, "{out}");
    assert_eq!(out["result"]["windows_opened"][0], "DialogWin");
    let detail = hover_detail(out["steps"].as_array().unwrap());
    assert_eq!(detail["saw_own_avatar"], true, "{detail}");
    assert_eq!(detail["view_retries"][0]["yaw_counts"], 250, "{detail}");
}

#[tokio::test]
async fn point_on_covered_entity_refuses_input_and_reports_candidate() {
    let mut sim = medic_behind_the_avatar();
    let e = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            point: Some(point(Vec3::new(1000.0, 0.0, 0.0))),
            rotate_camera: Some(false),
            force: true,
            expect: Some(Expect::Window),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "hover");
    assert_eq!(
        e.state.as_ref().unwrap()["candidate_ids"],
        serde_json::json!([11])
    );
    assert!(sim.buttons.is_empty());
}

#[tokio::test]
async fn ambiguous_point_reports_all_candidates_without_clicking() {
    let mut sim = Sim::new()
        .with_entity(11, "First", Vec3::new(1000.0, 0.0, 0.0))
        .with_entity(12, "Second", Vec3::new(1004.0, 0.0, 0.0));
    let e = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            point: Some(point(Vec3::new(1000.0, 0.0, 0.0))),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "resolve_target");
    assert_eq!(
        e.state.as_ref().unwrap()["candidate_ids"],
        serde_json::json!([11, 12])
    );
    assert!(sim.buttons.is_empty());
}

#[tokio::test]
async fn unmatched_point_covered_by_avatar_refuses_input() {
    let mut sim = medic_behind_the_avatar();
    // This point is outside the entity matching radius yet projects through
    // the avatar while the camera looks toward the medic.
    let e = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            point: Some(point(Vec3::new(1050.0, 0.0, 0.0))),
            rotate_camera: Some(false),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "hover");
    assert_eq!(e.state.as_ref().unwrap()["tried"][0]["hover"], "own_avatar");
    assert!(sim.buttons.is_empty());
}

#[tokio::test]
async fn a_target_behind_the_avatar_is_clicked_after_a_yaw_retry() {
    let mut sim = medic_behind_the_avatar();
    let out = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            entity_id: Some(11),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap();
    assert_eq!(out["hover_verified"], true, "{out}");
    assert_eq!(out["result"]["windows_opened"][0], "DialogWin");
    let steps = out["steps"].as_array().unwrap();
    let detail = hover_detail(steps);
    assert_eq!(detail["saw_own_avatar"], true, "{detail}");
    // The first change of view is a pure yaw turn, not a pitch.
    assert_eq!(detail["view_retries"][0]["yaw_counts"], 250, "{detail}");
    assert_eq!(detail["view_retries"][0]["pitch_counts"], 0, "{detail}");
    assert_eq!(detail["view_retries"].as_array().unwrap().len(), 1);
}

/// Probe B-P2: the actor `Location` is the feet and hovering it gives
/// nothing, so the click must aim at a body point. In this sim the feet hover
/// nothing (`REAL_BODY_DZ`). Reverting `HOVER_OFFSETS` to a set that starts at
/// the feet makes `tried[0]` a `nothing` at `dz` 0 and this test fails.
#[tokio::test]
async fn the_click_aims_above_the_feet() {
    let mut sim = Sim::new().with_entity(11, "Trainer", Vec3::new(1000.0, 0.0, 0.0));
    sim.entities[0].window = Some("DialogWin".into());
    sim.body_dz = REAL_BODY_DZ.to_vec();
    let feet = sim.project_point(Vec3::new(1000.0, 0.0, 0.0)).unwrap();
    let out = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            entity_id: Some(11),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap();
    assert_eq!(out["hover_verified"], true, "{out}");
    let steps = out["steps"].as_array().unwrap();
    let detail = hover_detail(steps);
    let first = &detail["tried"][0];
    assert_eq!(first["dz"], 100.0, "{detail}");
    assert_eq!(first["hover"], "entity", "{detail}");
    // 100 units above the feet at 10 m is 50 px higher on screen (smaller y).
    let y = first["at"][1].as_f64().unwrap();
    assert!(y < feet.1 - 40.0, "aimed at y {y}, feet at {}", feet.1);
    assert!(detail["view_retries"].is_null(), "{detail}");
}

/// The avatar covers the target's lower body only (the camera is behind and
/// above it): the height ladder reaches the trainer's shoulders in the SAME
/// pass, with no yaw retry. With the old heights (all low) every height is
/// covered and a yaw retry runs, so this test fails on a revert.
#[tokio::test]
async fn the_height_ladder_clears_the_avatar_before_any_yaw_retry() {
    let mut sim = medic_behind_the_avatar();
    sim.body_dz = REAL_BODY_DZ.to_vec();
    sim.avatar_cover_dz = 120.0;
    let out = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            entity_id: Some(11),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap();
    assert_eq!(out["hover_verified"], true, "{out}");
    let steps = out["steps"].as_array().unwrap();
    let detail = hover_detail(steps);
    assert_eq!(detail["tried"][0]["dz"], 100.0, "{detail}");
    assert_eq!(detail["tried"][0]["hover"], "own_avatar", "{detail}");
    assert_eq!(detail["tried"][1]["dz"], 140.0, "{detail}");
    assert_eq!(detail["tried"][1]["hover"], "entity", "{detail}");
    assert_eq!(detail["saw_own_avatar"], true, "{detail}");
    assert!(
        detail["view_retries"].is_null(),
        "no yaw was needed: {detail}"
    );
}

/// With nowhere to go (force off, rotation off) the failure names the avatar.
#[tokio::test]
async fn an_avatar_that_cannot_be_dodged_is_named() {
    let mut sim = medic_behind_the_avatar();
    sim.body_dz = REAL_BODY_DZ.to_vec();
    let e = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            entity_id: Some(11),
            rotate_camera: Some(false),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "hover");
    assert!(e.message.contains("own avatar"), "{}", e.message);
    assert!(sim.buttons.is_empty());
}

/// An entity that never answers mouse-over, close to the avatar: the view is
/// changed five times and put back, then the candidate is clicked anyway.
#[tokio::test]
async fn a_near_target_that_hovers_nothing_retries_then_clicks_the_candidate() {
    let mut sim = Sim::new().with_entity(11, "Corpse", Vec3::new(400.0, 0.0, 0.0));
    sim.unhoverable.push(11);
    let e = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            entity_id: Some(11),
            expect: Some(Expect::Target),
            settle_ms: Some(300),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap_err();
    // The click happened (the failure is the observe step, not the hover).
    assert_eq!(e.step, "observe");
    assert_eq!(sim.buttons, vec![(1, true), (1, false)]);
    let detail = hover_detail(&e.steps);
    assert_eq!(
        detail["view_retries"].as_array().unwrap().len(),
        click::VIEW_RETRIES.len(),
        "{detail}"
    );
    assert_eq!(detail["view_restored"], true, "{detail}");
    assert!(detail["tried_after_restore"].is_array(), "{detail}");
}

/// The same entity 20 m away keeps the old behaviour: no retries, click.
#[tokio::test]
async fn a_far_target_that_hovers_nothing_is_clicked_without_retries() {
    let mut sim = Sim::new().with_entity(11, "Corpse", Vec3::new(2000.0, 0.0, 0.0));
    sim.unhoverable.push(11);
    let e = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            entity_id: Some(11),
            expect: Some(Expect::Target),
            settle_ms: Some(300),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "observe");
    assert_eq!(sim.buttons, vec![(1, true), (1, false)]);
    assert!(hover_detail(&e.steps)["view_retries"].is_null());
}
