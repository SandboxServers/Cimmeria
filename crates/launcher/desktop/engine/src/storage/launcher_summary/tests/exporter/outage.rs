//! A server that is down, slow, overloaded, redirecting or too old. The rows
//! stay queued, the request counts are exact, and the waits are the recorded
//! ones. Each test ends with the same rows leaving once the server answers.
use super::*;

fn all_accepted(rows: usize) -> CycleOutcome {
    CycleOutcome::Delivered {
        accepted: rows,
        duplicate: 0,
        rejected: 0,
    }
}

/// What three attempts send when `route` is the step that fails.
fn three_attempts(route: &str) -> Vec<&'static str> {
    if route == MINT {
        vec![MINT; 3]
    } else {
        [MINT, INGEST].repeat(3)
    }
}

/// `route` answers with `answer`; the other step works.
async fn mount_failing(rig: &Rig, route: &str, answer: ResponseTemplate) {
    if route == INGEST {
        mount(&rig.server, MINT, minted()).await;
    }
    mount(&rig.server, route, answer).await;
}

/// Positive control: the server recovers and the same rows are delivered.
async fn recovers(rig: &Rig, rows: usize) {
    rig.server.reset().await;
    mount_ok(&rig.server).await;
    assert_eq!(rig.cycle().await, all_accepted(rows));
    assert_eq!(rig.paths().await, [MINT, INGEST]);
    assert_eq!(stored(&rig.owner()).entries, []);
}

#[tokio::test]
async fn a_failing_step_is_tried_three_times_and_the_rows_stay() {
    let status = ResponseTemplate::new;
    let after = |code, value: &str| status(code).insert_header("retry-after", value);
    let asked = [Duration::from_secs(3); 2];
    let cases = [
        (MINT, status(500), BACKOFFS),
        (INGEST, status(500), BACKOFFS),
        // A token the server no longer honours is minted again.
        (INGEST, status(401), BACKOFFS),
        (MINT, status(403), BACKOFFS),
        (MINT, after(429, "3"), asked),
        (INGEST, after(429, "3"), asked),
        (MINT, after(503, "3"), asked),
        (INGEST, after(503, "3"), asked),
        // Capped, and the cap when the server names no usable wait.
        (INGEST, after(503, "120"), [CAP; 2]),
        (MINT, status(503), [CAP; 2]),
        (INGEST, after(429, "soon"), [CAP; 2]),
    ];
    for (case, (route, answer, sleeps)) in cases.into_iter().enumerate() {
        let rig = Rig::opted_in().await;
        mount_failing(&rig, route, answer).await;
        rig.fail(2);
        let before = queue_bytes(&rig.owner());
        assert_eq!(rig.cycle().await, CycleOutcome::GaveUp, "case {case}");
        assert_eq!(rig.paths().await, three_attempts(route), "case {case}");
        assert_eq!(rig.probe.sleeps(), sleeps, "case {case}");
        assert_eq!(queue_bytes(&rig.owner()), before, "case {case}");
        recovers(&rig, 2).await;
    }
}

#[tokio::test]
async fn a_refused_connection_is_tried_three_times_and_the_rows_stay() {
    let rig = Rig::dead().await;
    let rows = rig.fail(1);
    assert_eq!(rig.cycle().await, CycleOutcome::GaveUp);
    // Three attempts, with one wait before each of the two retries.
    assert_eq!(rig.probe.mints(), 3);
    assert_eq!(rig.probe.sleeps(), BACKOFFS);
    assert_eq!(queued(&rig.owner()), rows);
    // Positive control: with a live endpoint the same steps deliver at once.
    let live = Rig::opted_in().await;
    mount_ok(&live.server).await;
    live.fail(1);
    assert_eq!(live.cycle().await, DELIVERED_ONE);
    assert_eq!(live.probe.mints(), 1);
}

#[tokio::test]
async fn a_server_that_does_not_answer_costs_three_deadlines_and_no_row() {
    for route in [MINT, INGEST] {
        let mut rig = Rig::opted_in().await;
        // The answer is held far longer than the deadline.
        let held = Duration::from_secs(120);
        let answer = if route == MINT {
            minted()
        } else {
            results(&["accepted"])
        };
        mount_failing(&rig, route, answer.set_delay(held)).await;
        let deadline = Duration::from_millis(1500);
        if route == MINT {
            rig.env.tuning.mint_timeout = deadline;
        } else {
            rig.env.tuning.post_timeout = deadline;
        }
        let rows = rig.fail(1);
        let started = std::time::Instant::now();
        assert_eq!(rig.cycle().await, CycleOutcome::GaveUp, "{route}");
        assert!(started.elapsed() < held, "bounded by the deadline");
        let expected = three_attempts(route);
        assert_eq!(rig.paths_after(expected.len()).await, expected, "{route}");
        assert_eq!(rig.probe.sleeps(), BACKOFFS, "{route}");
        assert_eq!(queued(&rig.owner()), rows, "{route}");
        rig.env.tuning.mint_timeout = Duration::from_secs(30);
        rig.env.tuning.post_timeout = Duration::from_secs(30);
        recovers(&rig, 1).await;
    }
}

#[tokio::test]
async fn a_redirect_is_never_followed() {
    const ELSEWHERE: &str = "/elsewhere";
    for (route, status) in [(MINT, 302), (INGEST, 302), (MINT, 307), (INGEST, 307)] {
        let rig = Rig::opted_in().await;
        let target = format!("{}{ELSEWHERE}", rig.base);
        let redirect = ResponseTemplate::new(status).insert_header("location", target.as_str());
        mount_failing(&rig, route, redirect).await;
        // A followed redirect would be answered here, and so be recorded.
        Mock::given(path(ELSEWHERE))
            .respond_with(minted())
            .mount(&rig.server)
            .await;
        let rows = rig.fail(1);
        assert_eq!(rig.cycle().await, CycleOutcome::GaveUp, "{route} {status}");
        assert_eq!(rig.paths().await, three_attempts(route), "{route} {status}");
        assert_eq!(queued(&rig.owner()), rows);

        // Positive control: a client that follows redirects does reach it.
        let following = reqwest::Client::new();
        let response = following
            .post(format!("{}{route}", rig.base))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(rig.paths().await.last().unwrap(), ELSEWHERE);
    }
}

#[tokio::test]
async fn a_server_without_the_summary_routes_stops_the_exporter_for_the_run() {
    let cases = [
        (MINT, 400),
        (MINT, 404),
        (MINT, 405),
        (MINT, 415),
        (MINT, 422),
        (INGEST, 404),
        (INGEST, 405),
    ];
    for (route, status) in cases {
        let rig = Rig::opted_in().await;
        mount_failing(&rig, route, ResponseTemplate::new(status)).await;
        let rows = rig.fail(1);
        assert_eq!(
            rig.cycle().await,
            CycleOutcome::StoppedForRun,
            "{route} {status}"
        );
        let sent: &[&str] = if route == MINT {
            &[MINT]
        } else {
            &[MINT, INGEST]
        };
        assert_eq!(rig.paths().await, sent, "{route} {status}: no retry");
        assert_eq!(rig.probe.sleeps(), []);
        assert_eq!(queued(&rig.owner()), rows);

        // A second trigger makes no request, even though the routes now exist.
        rig.server.reset().await;
        mount_ok(&rig.server).await;
        let rows = rig.fail(1);
        assert_eq!(rig.cycle().await, CycleOutcome::StoppedForRun);
        assert_eq!(rig.paths().await, [""; 0]);
        assert_eq!(queued(&rig.owner()), rows);

        // Positive control: the next run's exporter delivers both rows.
        let (next_run, _) = test_env(&rig.owner());
        assert_eq!(
            run_cycle(&Arc::downgrade(&rig.state), &next_run).await,
            all_accepted(2)
        );
        assert_eq!(rig.paths().await, [MINT, INGEST]);
    }
}
