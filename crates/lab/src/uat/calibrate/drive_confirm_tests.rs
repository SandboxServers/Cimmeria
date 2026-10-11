//! D-SV6 confirmations: the re-probe threshold, falling to the next best,
//! the winner bound, and several poses in one diff.

use serde_json::json;

use super::*;

#[tokio::test(start_paused = true)]
async fn a_failed_confirmation_falls_to_the_next_best() {
    // Candidate 5 hovers once in the sweep and then poorly; candidate 2 is
    // a steady 7.
    let client = Client::new(Box::new(|v, n| {
        if v.is(4.0, 0.0) {
            hits(if n == 0 { 8 } else { 3 })
        } else if v.is(3.0, 90.0) {
            hits(7)
        } else {
            hits(5)
        }
    }));
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(o.ok, "{o:?}");
    assert_eq!(o.confirmed, 3);
    assert_eq!(o.changed["bearing_deg"], json!([322.5, 90.0]));
    assert_eq!(o.changed["radius_m"], json!([4.16, 3.0]));
    assert_eq!(confirms(&run), [(5, 1), (2, 1), (2, 2), (2, 3)]);
    // 9 sweep probes, 1 failed confirmation, 3 passed.
    assert_eq!(o.probes, 13);
    let p = written_pose(&run);
    assert_eq!((p.radius_m, p.bearing_deg), (3.0, 90.0));
}

#[tokio::test(start_paused = true)]
async fn at_most_three_winners_are_tried() {
    // Every candidate hovers 8 once and then 3: each winner fails its first
    // confirmation, and the sweep gives up after three of them.
    let client = Client::new(Box::new(|_, n| hits(if n == 0 { 8 } else { 3 })));
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(!o.ok, "{o:?}");
    assert!(
        o.reason
            .as_deref()
            .is_some_and(|r| r.starts_with("no candidate held 3 confirmations")),
        "{o:?}"
    );
    // Ties go to the earlier candidate: 0, then 1, then 2.
    assert_eq!(confirms(&run), [(0, 1), (1, 1), (2, 1)]);
    assert_eq!(o.probes, 9 + 3);
    assert!(o.diff_path.is_none());
    assert!(!run.out_dir.join(format!("{SECTION}.diff")).exists());
}

#[tokio::test(start_paused = true)]
async fn six_jitter_hits_confirm() {
    // CONFIRM_MIN_HITS is 6: a steady 6 is confirmed three times.
    let client = Client::new(Box::new(|_, _| hits(6)));
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(o.ok, "{o:?}");
    assert_eq!(o.confirmed, 3);
    assert_eq!(confirms(&run), [(0, 1), (0, 2), (0, 3)]);
}

#[tokio::test(start_paused = true)]
async fn two_changed_poses_share_one_cumulative_diff() {
    let client = Client::new(Box::new(
        |v, _| {
            if v.is(4.0, 0.0) {
                hits(8)
            } else {
                hits(5)
            }
        },
    ));
    let text = spec_with("gm", &["frost", "frost-b"], "");
    let run = run_spec(&client, text, small_grid(), DEFAULT_BUDGET).await;
    let [a, b] = &run.outcomes[..] else {
        panic!("{:?}", run.outcomes)
    };
    assert!(a.ok && b.ok, "{a:?} {b:?}");
    assert_eq!((a.pose.as_str(), b.pose.as_str()), ("frost", "frost-b"));
    assert_eq!(a.changed["bearing_deg"], json!([322.5, 0.0]));
    assert_eq!(b.changed["bearing_deg"], json!([322.5, 0.0]));
    // D-SV12: the one diff is named once, on the first outcome.
    assert!(
        a.diff_path.is_some() && b.diff_path.is_none(),
        "{a:?} {b:?}"
    );
    let diff = run.diff();
    assert_eq!(added_lines(&diff), 2, "{diff}");
    let written = run.written();
    let poses = &written.rows[0].pose;
    assert_eq!(poses.len(), 2);
    for p in poses {
        assert_eq!((p.radius_m, p.bearing_deg), (4.0, 0.0), "{}", p.id);
        assert_eq!(p.calibrated.as_deref(), Some("2026-10-12 3/3"));
    }
}
