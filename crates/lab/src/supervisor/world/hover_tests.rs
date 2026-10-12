//! `client_hover_probe` against the simulated client.

use serde_json::json;

use super::super::click::HOVER_OFFSETS;
use super::super::geometry::Vec3;
use super::super::sim::Sim;
use super::super::{PointArg, Space};
use super::{run, HoverRequest};

fn client_point(x: f64, y: f64, z: f64) -> PointArg {
    PointArg {
        x,
        y,
        z,
        space: Some(Space::Client),
    }
}

/// A guard 10 m straight ahead (centre pixel 500, 425) and the cursor
/// parked in a corner.
fn guard_ahead() -> Sim {
    let mut s = Sim::new().with_entity(10, "Cellblock Guard", Vec3::new(1000.0, 0.0, 0.0));
    s.cursor = (3, 4);
    s
}

#[tokio::test]
async fn an_open_target_scores_full_jitter() {
    let mut sim = guard_ahead();
    let out = run(
        &mut sim,
        HoverRequest {
            entity_id: Some(10),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["verdict"], "ok");
    assert_eq!(out["hover"], "entity");
    assert_eq!(out["jitter_hits"], 8);
    assert_eq!(out["jitter"], 8);
    assert_eq!(out["pixel"], json!([500, 425]));
    assert_eq!(out["edge_px"], 375.0);
    assert_eq!(out["native_level"], "real_input");
    // A hover probe never presses a button, and puts the cursor back.
    assert!(
        sim.buttons.is_empty(),
        "the probe pressed {:?}",
        sim.buttons
    );
    assert_eq!(sim.cursor, (3, 4));
    assert_eq!(out["cursor_restored"], true);
}

#[tokio::test]
async fn the_avatar_in_front_costs_the_pose() {
    // Looking steeply down from the head puts the avatar on screen, below
    // a target standing just ahead.
    let mut sim = Sim::new().with_entity(10, "Cellblock Guard", Vec3::new(60.0, 0.0, 0.0));
    sim.cam_pitch = -1.2;
    let avatar = sim.project_point(sim.pos).expect("the avatar projects");
    // A radius that covers every body height of the target and the ring.
    let radius = HOVER_OFFSETS
        .iter()
        .map(|dz| {
            let p = sim
                .project_point(Vec3::new(60.0, 0.0, *dz))
                .expect("the target projects");
            (p.0 - avatar.0).hypot(p.1 - avatar.1)
        })
        .fold(0.0, f64::max)
        + 30.0;
    sim.avatar_radius_px = Some(radius);
    let out = run(
        &mut sim,
        HoverRequest {
            entity_id: Some(10),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["hover"], "own");
    assert_ne!(out["verdict"], "ok");
    assert_eq!(out["jitter_hits"], 0);
    let avatar_px = out["avatar_px"].as_f64().expect("avatar_px is reported");
    assert!(
        avatar_px < radius,
        "avatar_px {avatar_px} vs radius {radius}"
    );
    assert!(sim.buttons.is_empty());
}

#[tokio::test]
async fn a_hud_window_over_the_point_is_a_miss() {
    let mut sim = guard_ahead();
    // The point projects to (650, 475); the bar covers it and the ring.
    sim.window_rects = vec![("QuickBar".into(), [600.0, 450.0, 700.0, 500.0])];
    let out = run(
        &mut sim,
        HoverRequest {
            point: Some(client_point(1000.0, 300.0, 0.0)),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["pixel"], json!([650, 475]));
    assert_eq!(out["hover"], json!({ "ui": "QuickBar" }));
    assert_eq!(out["verdict"], json!({ "ui": "QuickBar" }));
    assert_eq!(out["jitter_hits"], 0);
    assert!(sim.buttons.is_empty());
}

#[tokio::test]
async fn a_point_aim_accepts_nothing_under_the_cursor() {
    let mut sim = guard_ahead();
    // (350, 475): 150 px left of the guard, no window.
    let out = run(
        &mut sim,
        HoverRequest {
            point: Some(client_point(1000.0, -300.0, 0.0)),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["verdict"], "ok");
    assert_eq!(out["hover"], "nothing");
    assert_eq!(out["jitter_hits"], 8);
    assert!(sim.buttons.is_empty());
    assert_eq!(sim.cursor, (3, 4));
}

#[tokio::test]
async fn a_point_aim_on_another_entity_needs_expect_entity() {
    // The point is the guard's feet, so the guard is under the cursor.
    let on_guard = || client_point(1000.0, 0.0, 0.0);
    let mut sim = guard_ahead().with_entity(11, "Elsewhere", Vec3::new(1000.0, -300.0, 0.0));
    let bare = run(
        &mut sim,
        HoverRequest {
            point: Some(on_guard()),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(bare["pixel"], json!([500, 475]));
    assert_eq!(bare["verdict"], json!({ "other_entity": 10 }));
    assert_eq!(bare["jitter_hits"], 0);

    let expected = run(
        &mut sim,
        HoverRequest {
            point: Some(on_guard()),
            expect_entity: Some(10),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(expected["verdict"], "ok");
    assert_eq!(expected["hover"], "entity");
    assert_eq!(expected["jitter_hits"], 8);

    let wrong = run(
        &mut sim,
        HoverRequest {
            point: Some(on_guard()),
            expect_entity: Some(11),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(wrong["verdict"], json!({ "other_entity": 10 }));
    assert!(sim.buttons.is_empty());
    assert_eq!(sim.cursor, (3, 4));
}

#[tokio::test]
async fn an_entity_aim_with_nothing_under_the_cursor_is_nothing() {
    let mut sim = guard_ahead();
    sim.unhoverable.push(10);
    let out = run(
        &mut sim,
        HoverRequest {
            entity_id: Some(10),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["hover"], "nothing");
    assert_eq!(out["verdict"], "nothing");
    assert_eq!(out["jitter_hits"], 0);
    assert!(sim.buttons.is_empty());
}

#[tokio::test]
async fn a_window_edge_through_the_ring_costs_part_of_the_jitter() {
    let mut sim = guard_ahead();
    // Centre (500, 425). A 20 px ring puts 3 points at x >= 510 (520, 514,
    // 514), all of which a window from x = 510 covers; the default 12 px
    // ring would lose only 1 (512).
    sim.window_rects = vec![("Tracker".into(), [510.0, 0.0, 900.0, 800.0])];
    let out = run(
        &mut sim,
        HoverRequest {
            entity_id: Some(10),
            jitter_px: Some(20),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["verdict"], "ok");
    let hits = out["jitter_hits"].as_u64().unwrap();
    assert!(0 < hits && hits < 8, "jitter_hits {hits}");
    assert_eq!(hits, 5);
    assert!(sim.buttons.is_empty());
}

#[tokio::test]
async fn a_failing_probe_still_puts_the_cursor_back() {
    let mut sim = guard_ahead();
    // The centre's read answers; the first ring read fails.
    sim.fail_mouse_over_after = Some(1);
    let err = run(
        &mut sim,
        HoverRequest {
            point: Some(client_point(1000.0, -300.0, 0.0)),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(err.step, "read_mouse_over");
    assert_eq!(sim.cursor, (3, 4));
    assert!(err.state.is_none(), "a clean restore adds nothing");
    assert!(sim.buttons.is_empty());
}

#[tokio::test]
async fn a_failed_restore_after_a_failure_is_reported() {
    let mut sim = guard_ahead();
    sim.fail_mouse_over_after = Some(1);
    sim.fail_cursor_restore = true;
    let err = run(
        &mut sim,
        HoverRequest {
            point: Some(client_point(1000.0, -300.0, 0.0)),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(err.step, "read_mouse_over");
    let state = err.state.clone().expect("the restore error is attached");
    assert_eq!(state["cursor_restored"], false);
    assert_eq!(
        state["cursor_restore_error"],
        "injected cursor restore failure"
    );
    assert_eq!(state["cursor_start"], json!([3, 4]));
    assert_ne!(sim.cursor, (3, 4));
}

#[tokio::test]
async fn off_screen_is_reported_not_turned() {
    let mut sim = Sim::new().with_entity(10, "Behind", Vec3::new(-1000.0, 0.0, 0.0));
    let yaw = sim.cam_yaw;
    let out = run(
        &mut sim,
        HoverRequest {
            entity_id: Some(10),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["verdict"], "off_screen");
    assert_eq!(sim.cam_yaw, yaw);
    assert!(sim.looks.is_empty(), "the probe looked: {:?}", sim.looks);
    assert!(sim.buttons.is_empty());
}

#[tokio::test]
async fn the_aim_must_be_one_entity_or_one_point() {
    let mut sim = guard_ahead();
    let both = run(
        &mut sim,
        HoverRequest {
            entity_id: Some(10),
            point: Some(client_point(1000.0, 0.0, 0.0)),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(both.step, "arguments");
    let expect_on_entity = run(
        &mut sim,
        HoverRequest {
            entity_id: Some(10),
            expect_entity: Some(10),
            ..HoverRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(expect_on_entity.step, "arguments");
    assert!(sim.buttons.is_empty());
}
