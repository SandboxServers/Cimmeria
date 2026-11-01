//! `client_camera` evidence (AX-33).

use serde_json::{json, Value};

use super::camera::{self, CameraRequest};
use super::camera_proof::checks;
use super::sim::Sim;
use crate::proof::fresh::Freshness;
use crate::proof::{EvVerdict, Evidence, Why};

fn view(yaw: f64, pitch: f64, zoom: f64) -> Value {
    json!({ "yaw_offset_deg": yaw, "pitch_offset_deg": pitch, "zoom": zoom })
}

fn judge(req: &CameraRequest, before: &Value, after: &Value, fresh: Freshness) -> Evidence {
    Evidence::judge(checks(req, before, after, &Value::Null, None, fresh, None))
}

fn yaw_req() -> CameraRequest {
    CameraRequest {
        yaw_counts: Some(500),
        ..CameraRequest::default()
    }
}

#[test]
fn look_that_moved_confirms() {
    let ev = judge(
        &yaw_req(),
        &view(10.0, 0.0, 300.0),
        &view(25.0, 0.0, 300.0),
        Freshness::Fresh,
    );
    assert_eq!(ev.verdict, EvVerdict::Confirmed, "{ev:?}");
}

#[test]
fn look_that_did_nothing_is_ignored() {
    let v = view(10.0, 5.0, 300.0);
    let ev = judge(&yaw_req(), &v, &v, Freshness::Fresh);
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::Ignored);
}

#[test]
fn yaw_delta_wraps_across_the_seam() {
    // 179.9 to -179.9 is a 0.2 degree turn, not 359.8.
    let ev = judge(
        &yaw_req(),
        &view(179.9, 0.0, 300.0),
        &view(-179.9, 0.0, 300.0),
        Freshness::Fresh,
    );
    assert_eq!(ev.verdict, EvVerdict::Confirmed, "{ev:?}");
    let ev = judge(
        &yaw_req(),
        &view(179.99, 0.0, 300.0),
        &view(-179.99, 0.0, 300.0),
        Freshness::Fresh,
    );
    assert_eq!(ev.verdict, EvVerdict::Denied, "{ev:?}");
}

#[test]
fn pinned_pitch_notes_the_clamp() {
    let req = CameraRequest {
        pitch_counts: Some(900),
        ..CameraRequest::default()
    };
    let v = view(0.0, 78.75, 300.0);
    let ev = judge(&req, &v, &v, Freshness::Fresh);
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.checks[0].after.as_ref().unwrap()["pitch_clamped"], true);
}

#[test]
fn no_args_is_no_evidence() {
    let v = view(0.0, 0.0, 300.0);
    let ev = judge(&CameraRequest::default(), &v, &v, Freshness::Fresh);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::NoEvidence);
}

#[test]
fn readout_error_is_no_post_read() {
    let ev = judge(
        &yaw_req(),
        &view(0.0, 0.0, 300.0),
        &json!({ "error": "x" }),
        Freshness::Fresh,
    );
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::NoPostRead);
}

#[test]
fn stale_read_is_not_a_denial() {
    let v = view(10.0, 0.0, 300.0);
    let ev = judge(&yaw_req(), &v, &v, Freshness::Stale);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::StaleRead);
}

#[test]
fn unstamped_read_keeps_its_verdict_and_is_marked() {
    let v = view(10.0, 0.0, 300.0);
    let ev = judge(&yaw_req(), &v, &v, Freshness::Unstamped);
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.checks[0].after.as_ref().unwrap()["unstamped"], true);
}

#[test]
fn zoom_cases() {
    let notches = CameraRequest {
        zoom_notches: Some(2),
        ..CameraRequest::default()
    };
    let ev = judge(
        &notches,
        &view(0.0, 0.0, 300.0),
        &view(0.0, 0.0, 240.0),
        Freshness::Fresh,
    );
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    let same = view(0.0, 0.0, 300.0);
    let ev = judge(&notches, &same, &same, Freshness::Fresh);
    assert_eq!((ev.verdict, ev.why), (EvVerdict::Denied, Why::Ignored));
    // Already at the asked distance: unchanged but met.
    let to = CameraRequest {
        zoom_to: Some(310.0),
        ..CameraRequest::default()
    };
    let ev = judge(&to, &same, &same, Freshness::Fresh);
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
}

#[test]
fn abs_pitch_within_tolerance() {
    let req = CameraRequest {
        pitch_deg: Some(10.0),
        ..CameraRequest::default()
    };
    let b = view(0.0, 0.0, 300.0);
    assert_eq!(
        judge(&req, &b, &view(0.0, 10.3, 300.0), Freshness::Fresh).verdict,
        EvVerdict::Confirmed
    );
    let ev = judge(&req, &b, &view(0.0, 12.0, 300.0), Freshness::Fresh);
    assert_eq!((ev.verdict, ev.why), (EvVerdict::Denied, Why::NoEffect));
}

#[test]
fn face_that_did_not_centre_is_denied() {
    let req = CameraRequest {
        face_entity_id: Some(10),
        ..CameraRequest::default()
    };
    let v = view(0.0, 0.0, 300.0);
    let run = |centred: bool| {
        Evidence::judge(checks(
            &req,
            &v,
            &v,
            &json!({ "centred": centred }),
            None,
            Freshness::Fresh,
            None,
        ))
    };
    assert_eq!(run(true).verdict, EvVerdict::Confirmed);
    assert_eq!(run(false).why, Why::NoEffect);
}

#[test]
fn yaw_at_reads_the_loop_result() {
    let req = CameraRequest {
        yaw_offset_deg: Some(25.0),
        ..CameraRequest::default()
    };
    let v = view(0.0, 0.0, 300.0);
    let abs = |got: Value| json!({ "yaw": { "want": 25.0, "got": got } });
    let run = |a: Value| {
        Evidence::judge(checks(
            &req,
            &v,
            &v,
            &Value::Null,
            Some(&a),
            Freshness::Fresh,
            None,
        ))
    };
    assert_eq!(run(abs(json!(25.2))).verdict, EvVerdict::Confirmed);
    assert_eq!(run(abs(json!(20.0))).verdict, EvVerdict::Denied);
    assert_eq!(run(abs(Value::Null)).verdict, EvVerdict::Denied);
}

#[test]
fn yaw_at_falls_back_to_the_controller_readout() {
    let req = CameraRequest {
        yaw_offset_deg: Some(25.0),
        ..CameraRequest::default()
    };
    let run = |after: Value| {
        Evidence::judge(checks(
            &req,
            &view(0.0, 0.0, 300.0),
            &after,
            &Value::Null,
            Some(&json!({ "yaw": { "want": 25.0, "got": null, "open_loop": true } })),
            Freshness::Fresh,
            None,
        ))
    };
    let moved = run(view(25.0, 0.0, 300.0));
    assert_eq!(moved.verdict, EvVerdict::Confirmed, "{moved:?}");
    assert_eq!(run(view(5.0, 0.0, 300.0)).verdict, EvVerdict::Denied);
    let errored = run(json!({ "error": "x" }));
    assert_eq!(errored.why, Why::NoPostRead, "{errored:?}");
}

#[tokio::test]
async fn sim_camera_look_confirms() {
    let mut sim = Sim::new();
    let out = camera::run(&mut sim, yaw_req_200()).await.unwrap();
    assert_eq!(out["evidence"]["verdict"], "confirmed", "{out}");
}

fn yaw_req_200() -> CameraRequest {
    CameraRequest {
        yaw_counts: Some(200),
        ..CameraRequest::default()
    }
}
