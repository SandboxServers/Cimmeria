//! Deadline and bridge-failure regression tests for client targeting.

use serde_json::Value;

use super::click::{self, ClickRequest, Expect};
use super::geometry::Vec3;
use super::sim::{Sim, UNITS};
use super::tool_tests::room;

fn target_of(sim: &Sim) -> u32 {
    sim.slots.get(&UNITS.target).copied().unwrap_or(0)
}

#[tokio::test(start_paused = true)]
async fn target_initial_slot_read_is_inside_the_total_deadline() {
    let mut sim = room();
    sim.slots.insert(UNITS.target, 77);
    sim.slots_never_returns = true;
    let started = tokio::time::Instant::now();
    let err = tokio::time::timeout(
        std::time::Duration::from_secs(11),
        click::run(
            &mut sim,
            "client_target",
            ClickRequest {
                entity_id: Some(10),
                ..ClickRequest::default()
            },
            Some((0, Expect::Target)),
        ),
    )
    .await
    .expect("target attempt must finish within 11 s")
    .unwrap_err();
    assert_eq!(err.step, "hover_timeout");
    assert!(err.message.contains("read_before"));
    assert_eq!(started.elapsed(), std::time::Duration::from_secs(10));
    assert_eq!(err.state.as_ref().unwrap()["remaining_deadline_ms"], 0);
    assert_eq!(target_of(&sim), 77);
    assert!(sim.buttons.is_empty());
}

#[tokio::test(start_paused = true)]
async fn target_projection_stall_is_inside_the_total_deadline() {
    let mut sim = room();
    sim.slots.insert(UNITS.target, 77);
    sim.project_never_after = Some(1); // find succeeds; click's own projection stalls
    let err = tokio::time::timeout(
        std::time::Duration::from_secs(11),
        click::run(
            &mut sim,
            "client_target",
            ClickRequest {
                entity_id: Some(10),
                ..ClickRequest::default()
            },
            Some((0, Expect::Target)),
        ),
    )
    .await
    .expect("projection stall must finish within 11 s")
    .unwrap_err();
    assert_eq!(err.step, "hover_timeout");
    assert!(err.message.contains("project"));
    assert_eq!(err.state.as_ref().unwrap()["attempted_pixel"], Value::Null);
    assert_eq!(target_of(&sim), 77);
    assert!(sim.buttons.is_empty());
}

#[tokio::test(start_paused = true)]
async fn target_camera_retry_stall_is_inside_the_total_deadline() {
    let mut sim = room();
    sim.slots.insert(UNITS.target, 77);
    sim.entities[0].pos = Vec3::new(400.0, 0.0, 0.0);
    sim.unhoverable.push(10);
    sim.camera_readout_never_returns = true;
    let err = tokio::time::timeout(
        std::time::Duration::from_secs(11),
        click::run(
            &mut sim,
            "client_target",
            ClickRequest {
                entity_id: Some(10),
                ..ClickRequest::default()
            },
            Some((0, Expect::Target)),
        ),
    )
    .await
    .expect("camera stall must finish within 11 s")
    .unwrap_err();
    assert_eq!(err.step, "hover_timeout");
    assert!(err.message.contains("camera_readout"));
    assert!(err.state.as_ref().unwrap()["attempted_pixel"].is_array());
    assert_eq!(target_of(&sim), 77);
    assert!(sim.buttons.is_empty());
}

#[tokio::test(start_paused = true)]
async fn target_camera_look_stall_is_inside_the_total_deadline() {
    let mut sim = room();
    sim.slots.insert(UNITS.target, 77);
    sim.entities[0].pos = Vec3::new(400.0, 0.0, 0.0);
    sim.unhoverable.push(10);
    sim.look_never_returns = true;
    let err = tokio::time::timeout(
        std::time::Duration::from_secs(11),
        click::run(
            &mut sim,
            "client_target",
            ClickRequest {
                entity_id: Some(10),
                ..ClickRequest::default()
            },
            Some((0, Expect::Target)),
        ),
    )
    .await
    .expect("camera look stall must finish within 11 s")
    .unwrap_err();
    assert_eq!(err.step, "hover_timeout");
    assert!(err.message.contains("view_retry"));
    assert!(err.state.as_ref().unwrap()["attempted_pixel"].is_array());
    assert_eq!(target_of(&sim), 77);
    assert!(sim.buttons.is_empty());
}

#[tokio::test]
async fn bridge_dispatch_timeout_preserves_hover_diagnostics() {
    let mut sim = room();
    sim.slots.insert(UNITS.target, 77);
    sim.slots_dispatch_timeout_after = Some(1);
    let err = click::run(
        &mut sim,
        "client_target",
        ClickRequest {
            entity_id: Some(10),
            ..ClickRequest::default()
        },
        Some((0, Expect::Target)),
    )
    .await
    .unwrap_err();
    assert_eq!(err.step, "read_mouse_over");
    assert!(err.message.contains("dispatch timeout"));
    assert!(err.state.as_ref().unwrap()["attempted_pixel"].is_array());
    assert!(
        err.state.as_ref().unwrap()["remaining_deadline_ms"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(target_of(&sim), 77);
    assert!(sim.buttons.is_empty());
}
