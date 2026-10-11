//! What stops or refuses a sweep: the budget, a revoked lease, a non-gm
//! section and a client out of the world.

use std::time::Duration;

use super::*;

#[tokio::test(start_paused = true)]
async fn the_budget_stops_the_sweep_with_the_best_so_far() {
    // A zero budget: the first candidate (the current pose) runs, then the
    // sweep stops; the current pose hovers and is confirmed.
    let client = Client::new(Box::new(
        |v, _| {
            if v.is(4.16, 322.5) {
                hits(7)
            } else {
                hits(8)
            }
        },
    ));
    let run = run_with(&client, "gm", small_grid(), Duration::ZERO).await;
    let o = &run.outcomes[0];
    assert!(o.ok, "{o:?}");
    assert_eq!(o.score, Some(7210));
    assert_eq!(o.confirmed, 3);
    assert!(
        o.reason.as_deref().is_some_and(|r| r.starts_with("budget")),
        "{o:?}"
    );
    assert_eq!(run.sweep().len(), 1, "only the first candidate was probed");
    // The winner is the pose as it stands: only its stamp is proposed.
    assert!(o.changed.is_empty(), "{o:?}");
    assert_eq!(added_lines(&run.diff()), 1);
}

#[tokio::test(start_paused = true)]
async fn the_budget_is_checked_between_candidates_not_only_between_stands() {
    // Two pitches share each stand point: 0 is the current pose, 1 and 2
    // stand at (4, 0). Each stand costs one 250 ms poll, so 300 ms is spent
    // after candidate 1; candidate 2 needs no stand but must not run.
    let grid = Grid {
        radii_m: vec![4.0],
        bearing_step_deg: 360.0,
        yaw_offsets_deg: vec![0.0],
        pitches_deg: vec![Some(-30.0), Some(-20.0)],
    };
    let client = Client::new(Box::new(|_, _| hits(8)));
    let run = run_with(&client, "gm", grid, Duration::from_millis(300)).await;
    let o = &run.outcomes[0];
    let sweep: Vec<u64> = run.sweep().iter().filter_map(|l| l["i"].as_u64()).collect();
    assert_eq!(sweep, [0, 1], "{o:?}");
    assert_eq!(o.reason.as_deref(), Some("budget: swept 2 of 3 candidates"));
    // Candidates 0 and 1 tie at 8210: the earlier, the current pose, wins.
    assert!(o.ok, "{o:?}");
    assert!(o.changed.is_empty(), "{o:?}");
}

#[tokio::test(start_paused = true)]
async fn a_spent_budget_starts_no_further_pose() {
    let client = Client::new(Box::new(|_, _| hits(8)));
    let text = spec_with("gm", &["frost", "frost-b"], "");
    let run = run_spec(&client, text, small_grid(), Duration::ZERO).await;
    let [a, b] = &run.outcomes[..] else {
        panic!("{:?}", run.outcomes)
    };
    assert!(a.ok, "{a:?}");
    assert!(!b.ok, "{b:?}");
    assert_eq!(b.reason.as_deref(), Some("budget: not started"));
    assert_eq!(b.probes, 0);
    assert!(run.jsonl().iter().all(|l| l["pose"] == "frost"));
}

#[tokio::test(start_paused = true)]
async fn a_lease_revoked_mid_sweep_stops_it() {
    // The lease goes after the third probe; candidate 3 must not run.
    let mut client = Client::new(Box::new(|_, _| hits(8)));
    client.revoke_after = Some(3);
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(!o.ok, "{o:?}");
    assert!(
        o.reason
            .as_deref()
            .is_some_and(|r| r.starts_with("lease revoked")),
        "{o:?}"
    );
    assert_eq!(o.confirmed, 0);
    assert!(o.diff_path.is_none());
    assert!(!run.out_dir.join(format!("{SECTION}.diff")).exists());
    assert_eq!(client.calls("client_hover_probe").len(), 3);
}

#[tokio::test(start_paused = true)]
async fn a_lease_revoked_while_confirming_stops_every_pose() {
    // The lease goes with the first confirmation probe of the first pose:
    // the second confirmation never stands, and the second pose never runs.
    let mut client = Client::new(Box::new(|_, _| hits(8)));
    client.revoke_after = Some(10);
    let text = spec_with("gm", &["frost", "frost-b"], "");
    let run = run_spec(&client, text, small_grid(), DEFAULT_BUDGET).await;
    for o in &run.outcomes {
        assert!(!o.ok, "{o:?}");
        assert!(
            o.reason
                .as_deref()
                .is_some_and(|r| r.starts_with("lease revoked")),
            "{o:?}"
        );
        assert!(o.diff_path.is_none(), "{o:?}");
    }
    assert_eq!(run.outcomes[1].probes, 0);
    assert_eq!(client.calls("client_hover_probe").len(), 10);
    assert_eq!(confirms(&run), [(0, 1)]);
}

#[tokio::test(start_paused = true)]
async fn a_non_gm_section_is_refused_before_anything_moves() {
    let client = Client::new(Box::new(|_, _| hits(8)));
    let run = run_with(&client, "non-gm", small_grid(), DEFAULT_BUDGET).await;
    let o = &run.outcomes[0];
    assert!(!o.ok);
    assert_eq!(o.pose, "frost");
    assert!(
        o.reason
            .as_deref()
            .is_some_and(|r| r.contains("gm section")),
        "{o:?}"
    );
    assert!(client.names().is_empty(), "{:?}", client.names());
    assert!(!run.out_dir.join(JSONL).exists());
}

#[tokio::test(start_paused = true)]
async fn a_client_out_of_world_is_refused() {
    let mut client = Client::new(Box::new(|_, _| hits(8)));
    client.login = "character_select";
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(
        o.reason
            .as_deref()
            .is_some_and(|r| r.contains("not in world")),
        "{o:?}"
    );
    assert_eq!(client.names(), ["lab_client_status"]);
}
