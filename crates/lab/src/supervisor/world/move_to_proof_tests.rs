//! `client_move_to` evidence and stops end to end against the simulated client.

use crate::supervisor::world::geometry::{client_to_server, Vec3};
use crate::supervisor::world::movement::{self, MoveRequest};
use crate::supervisor::world::sim::Sim;
use crate::supervisor::world::tool_tests::client_point;

fn req(x: f64) -> MoveRequest {
    MoveRequest {
        point: Some(client_point(x, 0.0, 0.0)),
        ..MoveRequest::default()
    }
}

#[tokio::test]
async fn sim_move_confirms_with_server() {
    let mut sim = Sim::live();
    sim.server_pos = Some(client_to_server(Vec3::new(0.0, 0.0, 0.0)));
    sim.server_pos_end = Some(client_to_server(Vec3::new(3000.0, 0.0, 0.0)));
    let out = movement::run(&mut sim, req(3000.0)).await.unwrap();
    assert_eq!(out["evidence"]["verdict"], "confirmed");
    assert_eq!(out["counts_as_native_pass"], true);
    assert!(out["server_end"].is_object());
}

#[tokio::test]
async fn sim_move_without_server_is_inconclusive() {
    let mut sim = Sim::live();
    let out = movement::run(&mut sim, req(3000.0)).await.unwrap();
    assert_eq!(out["arrived"], true);
    assert_eq!(out["evidence"]["verdict"], "inconclusive");
    assert_eq!(out["evidence"]["why"], "server_unreachable");
    assert_eq!(out["counts_as_native_pass"], false);
    assert!(out["server_end"].is_null());
}

#[tokio::test]
async fn sim_rubber_band_is_denied() {
    let mut sim = Sim::live();
    let goal = client_to_server(Vec3::new(3000.0, 0.0, 0.0));
    sim.server_pos = Some(goal);
    sim.server_pos_end = Some(goal);
    let out = movement::run(&mut sim, req(3000.0)).await.unwrap();
    assert_eq!(out["evidence"]["verdict"], "denied");
    assert_eq!(out["evidence"]["why"], "server_disagrees");
}

#[tokio::test]
async fn sim_stuck_carries_stop() {
    let mut sim = Sim::live();
    sim.wall_x = Some(1000.0);
    sim.jump_clears_wall = false;
    let e = movement::run(&mut sim, req(2500.0)).await.unwrap_err();
    let v = e.to_json();
    assert_eq!(v["stop"]["kind"], "stuck");
    let tried = v["stop"]["tried"].as_array().unwrap();
    assert!(tried.iter().any(|t| t == "jump"), "{tried:?}");
    assert_eq!(v["stop"]["suggest"][0], "unstick");
    // The evidence JSON is kept beside the stop.
    assert!(v["state"]["path"].is_array());
}

#[tokio::test]
async fn sim_timeout_carries_stop() {
    let mut sim = Sim::live();
    sim.speed_uu_s = 100.0;
    let mut r = req(5000.0);
    r.timeout_ms = Some(2000);
    let e = movement::run(&mut sim, r).await.unwrap_err();
    assert_eq!(e.to_json()["stop"]["kind"], "timeout");
}
