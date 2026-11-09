//! `client_interact`: argument handling, the native call's shape and the
//! verdict. No live client; the sim records what would have been fired.

use serde_json::json;

use super::super::geometry::Vec3;
use super::super::sim::{Sim, PLAYER_ID};
use super::{interact_params, run, InteractRequest, INTERACT_FIRER_VA};

fn req(entity_id: u32) -> InteractRequest {
    InteractRequest { entity_id }
}

/// A vial `x` UE3 units straight ahead of the avatar.
fn vial_at(x: f64) -> Sim {
    Sim::new().with_entity(11, "Ambernol Vial", Vec3::new(x, 0.0, 0.0))
}

#[test]
fn params_are_a_stdcall_call_of_the_rebased_firer_with_the_full_id_word() {
    let p = interact_params(0x10_0000, 0x1234_5678);
    assert_eq!(p["conv"], "stdcall");
    assert_eq!(p["ret"], "void");
    assert_eq!(p["addr"], format!("{:#x}", INTERACT_FIRER_VA + 0x10_0000));
    assert_eq!(p["args"], json!(["0x12345678"]));
    assert_eq!(INTERACT_FIRER_VA, 0x00de_f4b0);
}

#[test]
fn a_negative_slide_rebases_down() {
    let p = interact_params(-0x1000, 7);
    assert_eq!(p["addr"], format!("{:#x}", INTERACT_FIRER_VA - 0x1000));
}

#[tokio::test]
async fn fires_the_interact_for_the_entity_id_and_reports_n3_native_call() {
    let mut sim = vial_at(400.0);
    let out = run(&mut sim, req(11)).await.unwrap();
    assert_eq!(sim.interact_calls, vec![11]);
    assert_eq!(out["fired"], true, "{out}");
    assert_eq!(out["native_level"], "native_call", "{out}");
    assert_eq!(out["native_tier"], "N3", "{out}");
    assert_eq!(out["counts_as_native_pass"], false, "{out}");
    assert_eq!(out["range"]["in_interact_range"], true, "{out}");
}

#[tokio::test]
async fn the_verdict_is_inconclusive_because_the_server_is_graded_elsewhere() {
    let mut sim = vial_at(400.0);
    let out = run(&mut sim, req(11)).await.unwrap();
    assert_eq!(out["evidence"]["verdict"], "inconclusive", "{out}");
    assert_eq!(out["evidence"]["why"], "no_post_read", "{out}");
}

#[tokio::test]
async fn beyond_the_interact_range_the_verdict_is_denied_out_of_range() {
    let mut sim = vial_at(2000.0);
    let out = run(&mut sim, req(11)).await.unwrap();
    assert_eq!(out["evidence"]["verdict"], "denied", "{out}");
    assert_eq!(out["evidence"]["why"], "out_of_range", "{out}");
    assert_eq!(out["range"]["distance_m"], 20.0, "{out}");
    assert!(sim.interact_calls.is_empty(), "{out}");
    assert_eq!(out["fired"], false, "{out}");
}

#[tokio::test]
async fn an_entity_the_client_does_not_know_fires_nothing() {
    let mut sim = vial_at(400.0);
    let e = run(&mut sim, req(999)).await.unwrap_err();
    assert_eq!(e.step, "resolve");
    assert!(
        e.message.contains("not in the client's entity map"),
        "{}",
        e.message
    );
    assert!(sim.interact_calls.is_empty());
}

#[tokio::test]
async fn zero_and_the_players_own_id_fire_nothing() {
    let mut sim = vial_at(400.0);
    let e = run(&mut sim, req(0)).await.unwrap_err();
    assert_eq!(e.step, "args");
    let e = run(&mut sim, req(PLAYER_ID)).await.unwrap_err();
    assert_eq!(e.step, "resolve");
    assert!(e.message.contains("own id"), "{}", e.message);
    assert!(sim.interact_calls.is_empty());
}

#[tokio::test]
async fn a_failed_native_call_is_a_fire_step_failure_at_native_call_level() {
    let mut sim = vial_at(400.0);
    sim.interact_error = Some("call_native: seh fault".into());
    let e = run(&mut sim, req(11)).await.unwrap_err();
    assert_eq!(e.step, "fire");
    assert!(e.message.contains("seh fault"), "{}", e.message);
    assert_eq!(e.to_json()["native_level"], "native_call");
}

#[test]
fn the_tool_needs_the_lease_and_is_judged() {
    use crate::lease::policy::{gate, Gate, LEASED};
    use crate::proof::claims::{claim, Claim};
    assert_eq!(gate("client_interact"), Gate::Lease);
    assert!(LEASED.contains(&"client_interact"));
    assert_eq!(claim("client_interact"), Claim::Judged);
}
