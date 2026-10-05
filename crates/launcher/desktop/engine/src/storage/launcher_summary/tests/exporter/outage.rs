//! A server that is down, slow, overloaded, rate limiting, redirecting or too
//! old. The rows stay queued, the request counts are exact, and the waits are
//! the recorded ones. Each test ends with the same rows leaving once the server
//! answers.
use super::*;

fn all_accepted(rows: usize) -> CycleOutcome {
    CycleOutcome::Delivered {
        accepted: rows,
        duplicate: 0,
        rejected: 0,
    }
}

/// Positive control: the server recovers and the same rows are delivered.
async fn recovers(rig: &Rig, rows: usize) {
    rig.server.reset().await;
    mount_ok(&rig.server).await;
    assert_eq!(rig.cycle().await, all_accepted(rows));
    assert_eq!(rig.paths().await, [INGEST]);
    assert_eq!(stored(&rig.owner()).entries, []);
}

#[tokio::test]
async fn a_failing_post_is_tried_three_times_and_the_rows_stay() {
    let status = ResponseTemplate::new;
    let after = |code, value: &str| status(code).insert_header("retry-after", value);
    let asked = [Duration::from_secs(3); 2];
    let cases = [
        (status(500), BACKOFFS),
        (status(502), BACKOFFS),
        // Only a `503` names the wait.
        (after(500, "3"), BACKOFFS),
        // The route asks for no credentials, so there is none to send next time.
        (status(401), BACKOFFS),
        (status(403), BACKOFFS),
        (after(503, "3"), asked),
        // Capped, and the cap when the server names no usable wait.
        (after(503, "120"), [CAP; 2]),
        (status(503), [CAP; 2]),
        (after(503, "soon"), [CAP; 2]),
    ];
    for (case, (answer, sleeps)) in cases.into_iter().enumerate() {
        let rig = Rig::opted_in().await;
        mount(&rig.server, answer).await;
        rig.fail(2);
        let before = queue_bytes(&rig.owner());
        assert_eq!(rig.cycle().await, CycleOutcome::GaveUp, "case {case}");
        // One POST and one more for each retry: `1 + max_retries`.
        assert_eq!(rig.paths().await, [INGEST; 3], "case {case}");
        assert_eq!(rig.probe.sleeps(), sleeps, "case {case}");
        assert_eq!(queue_bytes(&rig.owner()), before, "case {case}");
        rig.assert_anonymous("a retry").await;
        recovers(&rig, 2).await;
    }
}

// The server's limit is a fixed number of requests a minute for each address
// (12 by default), and a retry before the minute ends would only spend it. The
// `503` case is the control: the same answer under the
// other status is retried, with the waits it names.
#[tokio::test]
async fn a_rate_limited_post_is_not_retried_and_the_rows_wait_for_the_next_trigger() {
    let status = ResponseTemplate::new;
    let cases = [
        (503, status(503).insert_header("retry-after", "3"), true),
        (429, status(429).insert_header("retry-after", "3"), false),
        (429, status(429), false),
    ];
    for (code, answer, retried) in cases {
        let rig = Rig::opted_in().await;
        mount(&rig.server, answer).await;
        let rows = rig.fail(2);
        let before = queue_bytes(&rig.owner());
        assert_eq!(rig.cycle().await, CycleOutcome::GaveUp, "{code}");
        if retried {
            assert_eq!(rig.paths().await, [INGEST; 3], "{code}");
            assert_eq!(rig.probe.sleeps(), [Duration::from_secs(3); 2], "{code}");
        } else {
            assert_eq!(rig.paths().await, [INGEST], "{code}: exactly one request");
            assert_eq!(
                rig.probe.sleeps(),
                Vec::<Duration>::new(),
                "{code}: no wait"
            );
        }
        assert_eq!(queued(&rig.owner()), rows, "{code}");
        assert_eq!(queue_bytes(&rig.owner()), before, "{code}");

        // A later trigger with the limit lifted delivers the same rows: the
        // exporter was not stopped for the run.
        recovers(&rig, 2).await;
        assert_eq!(rig.posts().await[0]["summaries"], serde_json::json!(rows));
    }
}

#[tokio::test]
async fn a_refused_connection_is_tried_three_times_and_the_rows_stay() {
    let rig = Rig::dead().await;
    let rows = rig.fail(1);
    assert_eq!(rig.cycle().await, CycleOutcome::GaveUp);
    // One wait before each of the two retries, so three attempts.
    assert_eq!(rig.probe.sleeps(), BACKOFFS);
    assert_eq!(queued(&rig.owner()), rows);
    // Positive control: with a live endpoint the same steps deliver at once.
    let live = Rig::opted_in().await;
    mount_ok(&live.server).await;
    live.fail(1);
    assert_eq!(live.cycle().await, DELIVERED_ONE);
    assert_eq!(live.probe.sleeps(), Vec::<Duration>::new());
}

#[tokio::test]
async fn a_server_that_does_not_answer_costs_three_deadlines_and_no_row() {
    let mut rig = Rig::opted_in().await;
    // The answer is held far longer than the deadline.
    let held = Duration::from_secs(120);
    mount(&rig.server, results(&["accepted"]).set_delay(held)).await;
    rig.env.tuning.post_timeout = Duration::from_millis(1500);
    let rows = rig.fail(1);
    let started = std::time::Instant::now();
    assert_eq!(rig.cycle().await, CycleOutcome::GaveUp);
    assert!(started.elapsed() < held, "bounded by the deadline");
    assert_eq!(rig.paths_after(3).await, [INGEST; 3]);
    assert_eq!(rig.probe.sleeps(), BACKOFFS);
    assert_eq!(queued(&rig.owner()), rows);
    rig.env.tuning.post_timeout = Duration::from_secs(30);
    recovers(&rig, 1).await;
}

#[tokio::test]
async fn a_redirect_is_never_followed() {
    const ELSEWHERE: &str = "/elsewhere";
    for status in [301, 302, 303, 307, 308] {
        let rig = Rig::opted_in().await;
        let target = format!("{}{ELSEWHERE}", rig.server.uri());
        let redirect = ResponseTemplate::new(status).insert_header("location", target.as_str());
        mount(&rig.server, redirect).await;
        // A followed redirect would be answered here, and so be recorded.
        Mock::given(path(ELSEWHERE))
            .respond_with(results(&["accepted"]))
            .mount(&rig.server)
            .await;
        let rows = rig.fail(1);
        assert_eq!(rig.cycle().await, CycleOutcome::GaveUp, "{status}");
        assert_eq!(rig.paths().await, [INGEST; 3], "{status}");
        assert_eq!(queued(&rig.owner()), rows);

        // Positive control: a client that follows redirects does reach it.
        let following = reqwest::Client::new();
        let response = following
            .post(format!("{}{INGEST}", rig.server.uri()))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(rig.paths().await.last().unwrap(), ELSEWHERE);
    }
}

#[tokio::test]
async fn a_server_without_the_summary_route_stops_the_exporter_for_the_run() {
    for status in [404, 405] {
        let rig = Rig::opted_in().await;
        mount(&rig.server, ResponseTemplate::new(status)).await;
        let rows = rig.fail(1);
        assert_eq!(rig.cycle().await, CycleOutcome::StoppedForRun, "{status}");
        assert_eq!(rig.paths().await, [INGEST], "{status}: no retry");
        assert_eq!(rig.probe.sleeps(), Vec::<Duration>::new());
        assert_eq!(queued(&rig.owner()), rows);

        // A second trigger makes no request, even though the route now exists.
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
        assert_eq!(rig.paths().await, [INGEST]);
    }
}
