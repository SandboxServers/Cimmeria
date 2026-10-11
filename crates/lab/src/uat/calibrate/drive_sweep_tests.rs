//! The sweep itself: candidate order, what the camera and the probe are
//! sent, misses, and what one pose proposes.

use std::path::Path;

use serde_json::json;

use super::*;

#[tokio::test(start_paused = true)]
async fn the_best_hovering_candidate_wins_and_is_confirmed() {
    // The winner's avatar is off-screen: `avatar_px` null, as the real
    // probe sends it, must still score.
    let client = Client::new(Box::new(
        |v, _| {
            if v.is(4.0, 0.0) {
                far(8)
            } else {
                hits(5)
            }
        },
    ));
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(o.ok, "{o:?}");
    assert_eq!(o.confirmed, 3);
    // 8 * 1000 + 400 (null avatar_px is the cap) + 120 / 2.
    assert_eq!(o.score, Some(8460));
    assert_eq!(o.changed["bearing_deg"], json!([322.5, 0.0]));
    assert_eq!(o.changed["radius_m"], json!([4.16, 4.0]));
    assert_eq!(o.changed["pitch_deg"], json!([-28.8, -30.0]));
    assert_eq!(o.changed["yaw_offset_deg"], json!([25.3, 0.0]));
    assert!(o.changed.get("zoom").is_none(), "zoom did not change");
    // 9 sweep probes, then 3 confirmations.
    assert_eq!(o.probes, 12);
    let order: Vec<(f64, f64)> = run
        .sweep()
        .iter()
        .map(|l| (l["r"].as_f64().unwrap(), l["b"].as_f64().unwrap()))
        .collect();
    assert_eq!(
        order,
        [
            (4.16, 322.5),
            (3.0, 0.0),
            (3.0, 90.0),
            (3.0, 180.0),
            (3.0, 270.0),
            (4.0, 0.0),
            (4.0, 90.0),
            (4.0, 180.0),
            (4.0, 270.0)
        ],
        "the grid's order"
    );
    assert!(run.jsonl().iter().all(|l| !l.to_string().contains("null")));
    // The first candidate is the current pose: the camera gets exactly its
    // face, yaw, zoom and pitch, and the probe the entity alone.
    assert_eq!(
        client.calls("client_camera")[0],
        json!({ "face_entity_id": FROST_ID, "yaw_offset_deg": 25.3, "zoom": 250.0,
                "pitch_deg": -28.8 })
    );
    assert_eq!(
        client.calls("client_hover_probe")[0],
        json!({ "entity_id": FROST_ID })
    );
    // Each confirmation re-stands: away to the opposite bearing, then back.
    let stands = client.stands();
    assert_eq!(
        stands[9..],
        [
            (4.0, 180.0),
            (4.0, 0.0),
            (4.0, 180.0),
            (4.0, 0.0),
            (4.0, 180.0),
            (4.0, 0.0)
        ]
    );
    // The copy in the run folder loads with the winner; the spec is untouched.
    let p = written_pose(&run);
    assert_eq!((p.radius_m, p.bearing_deg), (4.0, 0.0));
    assert_eq!(
        (p.pitch_deg, p.yaw_offset_deg, p.zoom),
        (Some(-30.0), Some(0.0), Some(250.0))
    );
    assert_eq!(p.calibrated.as_deref(), Some("2026-10-12 3/3"));
    assert_eq!(
        std::fs::read_to_string(&run.spec_path).unwrap(),
        run.original
    );
    let diff_path = o.diff_path.as_deref().unwrap();
    assert!(
        Path::new(diff_path).starts_with(&run.out_dir),
        "{diff_path}"
    );
    let diff = std::fs::read_to_string(diff_path).unwrap();
    assert!(diff.starts_with(
        "--- a/docs/guides/uat-specs/first-session.toml\n+++ b/docs/guides/uat-specs/first-session.toml\n"
    ));
    assert_eq!(added_lines(&diff), 1);
    // D-SV12: one pose's outcome, with a real-length run folder path and
    // a real section id, stays under 400 characters, with no nulls.
    let text = serde_json::to_string(o).unwrap();
    assert!(text.len() < 400, "{} chars: {text}", text.len());
    assert!(!text.contains("null"), "{text}");
    assert!(!text.contains("spec_path"), "{text}");
}

#[tokio::test(start_paused = true)]
async fn nothing_is_clicked() {
    let client = Client::new(Box::new(
        |v, _| {
            if v.is(4.0, 0.0) {
                hits(8)
            } else {
                hits(5)
            }
        },
    ));
    let run = run(&client).await;
    assert!(run.outcomes[0].ok);
    // The row's own step is a click and the click tools are routed, so
    // their absence is the sweep's choice. (`run_spec` has already checked
    // that every call was one of SWEEP_TOOLS; this pins it by name.)
    assert!(run.original.contains("@world_click"));
    let names = client.names();
    for never in ["client_world_click", "client_target", "client_input_mouse"] {
        assert!(client.has_tool(never));
        assert!(!names.iter().any(|n| n == never), "{never} was called");
    }
}

#[tokio::test(start_paused = true)]
async fn an_aim_point_pose_faces_and_probes_the_point() {
    let client = Client::new(Box::new(|_, _| hits(8)));
    let text = spec_with("gm", &["frost"], r#", aim = "point", aim_dy_m = 1.20"#);
    let run = run_spec(&client, text, small_grid(), DEFAULT_BUDGET).await;
    assert!(run.outcomes[0].ok, "{:?}", run.outcomes[0]);
    let aim = json!({ "x": FROST[0], "y": FROST[1] + 1.2, "z": FROST[2] });
    let cam = &client.calls("client_camera")[0];
    assert_eq!(cam["face_point"], aim, "{cam}");
    assert!(cam.get("face_entity_id").is_none(), "{cam}");
    // Point aim: the point, and the entity it must not be hidden by.
    for probe in client.calls("client_hover_probe") {
        assert_eq!(probe, json!({ "point": aim, "expect_entity": FROST_ID }));
    }
}

#[tokio::test(start_paused = true)]
async fn a_failed_stand_misses_its_candidates_without_a_probe() {
    // Candidate 2 (3 m at 90) cannot be reached; the winner is 4 m at 0.
    let mut client = Client::new(Box::new(
        |v, _| {
            if v.is(4.0, 0.0) {
                hits(8)
            } else {
                hits(5)
            }
        },
    ));
    client.blocked = vec![(3.0, 90.0)];
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(o.ok, "{o:?}");
    let sweep = run.sweep();
    assert_eq!(sweep.len(), 9);
    assert_eq!(sweep[2]["miss"], "stand", "{}", sweep[2]);
    assert!(sweep[2].get("score").is_none(), "{}", sweep[2]);
    assert!(sweep.iter().filter(|l| l.get("miss").is_some()).count() == 1);
    // 8 sweep probes and 3 confirmations: the missed stand cost none, and
    // neither the camera nor the probe ran for it.
    assert_eq!(o.probes, 11);
    assert_eq!(client.calls("client_hover_probe").len(), 11);
    assert_eq!(client.calls("client_camera").len(), 11);
}

#[tokio::test(start_paused = true)]
async fn the_unchanged_pose_proposes_only_its_stamp() {
    // The current pose is the only one that hovers well. A re-measurement
    // still leaves provenance: the diff changes `calibrated` and nothing
    // else.
    let client = Client::new(Box::new(
        |v, _| {
            if v.is(4.16, 322.5) {
                hits(8)
            } else {
                hits(4)
            }
        },
    ));
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(o.ok, "{o:?}");
    assert_eq!(o.confirmed, 3);
    assert!(o.changed.is_empty(), "{o:?}");
    assert!(o.diff_path.is_some(), "{o:?}");
    let diff = run.diff();
    assert_eq!(added_lines(&diff), 1, "{diff}");
    assert!(diff.contains(r#"calibrated = "2026-10-12 3/3""#), "{diff}");
    let before = spec::parse(&run.original).unwrap().rows[0].pose[0].clone();
    let after = written_pose(&run);
    assert_eq!(after.calibrated.as_deref(), Some("2026-10-12 3/3"));
    assert_eq!(
        PoseSpec {
            calibrated: None,
            ..after
        },
        before
    );
}

#[tokio::test(start_paused = true)]
async fn nothing_hovering_says_so() {
    let client = Client::new(Box::new(
        |_, _| json!({ "verdict": "nothing", "pixel": [1, 1] }),
    ));
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(!o.ok);
    assert_eq!(
        o.reason.as_deref(),
        Some("no candidate hovered the target (9 probes)")
    );
    assert_eq!(o.confirmed, 0);
    assert!(o.diff_path.is_none());
}

#[test]
fn the_diff_names_the_repo_relative_spec() {
    let win = r"C:\repo\docs\guides\uat-specs\first-session.toml";
    assert_eq!(
        repo_relative(win),
        "docs/guides/uat-specs/first-session.toml"
    );
    assert_eq!(
        repo_relative("docs/guides/uat-specs/a.toml"),
        "docs/guides/uat-specs/a.toml"
    );
    assert_eq!(repo_relative("/elsewhere/specs/a.toml"), "a.toml");
    assert_eq!(repo_relative("/x/mydocs/guides/uat-specs/a.toml"), "a.toml");
}
