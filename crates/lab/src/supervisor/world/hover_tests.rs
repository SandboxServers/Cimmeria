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

/// A guard 10 m straight ahead (centre pixel 500, 475) and the cursor
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
    assert_eq!(out["pixel"], json!([500, 475]));
    assert_eq!(out["edge_px"], 325.0);
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
