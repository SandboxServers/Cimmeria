//! What a cycle sends, and what it does with the server's answer.
use super::*;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf};

fn fixture(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[tokio::test]
async fn a_queued_row_is_posted_once_and_removed() {
    let rig = Rig::opted_in().await;
    mount_ok(&rig.server).await;
    // Nothing queued: nothing sent.
    assert_eq!(rig.cycle().await, CycleOutcome::Empty);
    assert_eq!(rig.paths().await, [""; 0]);

    let rows = rig.fail(1);
    assert_eq!(rig.cycle().await, DELIVERED_ONE);
    assert_eq!(rig.paths().await, [INGEST]);
    let requests = rig.server.received_requests().await.unwrap();
    assert_eq!(requests[0].method.as_str(), "POST");
    assert_eq!(requests[0].headers["content-type"], "application/json");
    assert!(!requests[0].headers.contains_key("content-encoding"));
    let sent: SummaryRequest = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(sent.summaries, rows);
    assert_eq!(sent.client_dropped, DroppedCounts::default());
    assert_eq!(stored(&rig.owner()).entries, []);
    assert_eq!(rig.probe.sleeps(), []);
    // The row is gone, so the next cycle has nothing to send.
    assert_eq!(rig.cycle().await, CycleOutcome::Empty);
    assert_eq!(rig.paths().await, [INGEST]);
}

#[tokio::test]
async fn without_consent_nothing_is_queued_or_sent() {
    let rig = Rig::start().await;
    mount_ok(&rig.server).await;
    assert_eq!(rig.fail(1), []);
    assert_eq!(rig.cycle().await, CycleOutcome::SkippedNoConsent);
    assert_eq!(queue_bytes(&rig.owner()), None, "no queue file");
    assert_eq!(rig.paths().await, [""; 0]);
    // Positive control: the same steps after opting in deliver the row.
    set_consent(&mut rig.owner(), true);
    assert_eq!(rig.fail(1).len(), 1);
    assert_eq!(rig.cycle().await, DELIVERED_ONE);
    assert_eq!(rig.paths().await, [INGEST]);
}

#[tokio::test]
async fn only_the_endpoint_configured_now_is_contacted() {
    let rig = Rig::opted_in().await;
    mount_ok(&rig.server).await;
    let rows = rig.fail(1);
    // Reconfigured elsewhere: this exporter's endpoint is no longer the one.
    rig.owner().configure_summaries(config(Some(ENDPOINT)));
    assert_eq!(rig.cycle().await, CycleOutcome::SkippedNoEndpoint);
    assert_eq!(rig.paths().await, [""; 0]);
    assert_eq!(queued(&rig.owner()), rows);
    // Positive control: configured back, the same row is delivered.
    rig.owner().configure_summaries(config(Some(&rig.base)));
    assert_eq!(rig.cycle().await, DELIVERED_ONE);
    assert_eq!(rig.paths().await, [INGEST]);

    // No endpoint at all: nothing is sent and nothing is kept.
    rig.fail(1);
    rig.owner().configure_summaries(config(None));
    assert_eq!(rig.cycle().await, CycleOutcome::SkippedNoEndpoint);
    assert_eq!(queue_bytes(&rig.owner()), None);
    assert_eq!(rig.paths().await, [INGEST]);
}

#[tokio::test]
async fn every_verdict_removes_its_row_and_a_rejection_is_counted() {
    let rig = Rig::opted_in().await;
    let answer = fixture(include_str!("../../fixtures/response-mixed.json"));
    mount(
        &rig.server,
        ResponseTemplate::new(200).set_body_json(answer),
    )
    .await;
    rig.fail(3);
    assert_eq!(
        rig.cycle().await,
        CycleOutcome::Delivered {
            accepted: 1,
            duplicate: 1,
            rejected: 1,
        }
    );
    let on_disk = stored(&rig.owner());
    assert_eq!(on_disk.entries, []);
    assert_eq!(
        on_disk.dropped,
        DroppedCounts {
            overflow: 0,
            expired: 0,
            rejected: 1,
        }
    );
    assert_eq!(rig.paths().await, [INGEST]);
}

// The control is `outage.rs`: under any other failing status the rows stay.
#[tokio::test]
async fn a_body_refused_for_good_is_dropped_and_counted() {
    for status in [400, 413, 415, 422] {
        let rig = Rig::opted_in().await;
        mount(&rig.server, ResponseTemplate::new(status)).await;
        rig.fail(2);
        assert_eq!(
            rig.cycle().await,
            CycleOutcome::Delivered {
                accepted: 0,
                duplicate: 0,
                rejected: 2,
            },
            "{status}"
        );
        let on_disk = stored(&rig.owner());
        assert_eq!(on_disk.entries, [], "{status}");
        assert_eq!(on_disk.dropped.rejected, 2, "{status}");
        assert_eq!(rig.paths().await, [INGEST], "{status}: no retry");
        assert_eq!(rig.probe.sleeps(), [], "{status}");
    }
}

#[tokio::test]
async fn drop_counters_are_sent_until_a_post_is_answered_and_then_no_more() {
    let rig = Rig::opted_in().await;
    mount(&rig.server, ResponseTemplate::new(500)).await;
    rig.fail(1);
    let counted = DroppedCounts {
        overflow: 5,
        expired: 2,
        rejected: 1,
    };
    lock(&rig.owner().summaries).queue.dropped = counted;
    assert_eq!(rig.cycle().await, CycleOutcome::GaveUp);
    let posts = rig.posts().await;
    assert_eq!(posts.len(), 3);
    for post in &posts {
        assert_eq!(post["client_dropped"], json!(counted));
    }
    assert_eq!(
        lock(&rig.owner().summaries).queue.dropped,
        counted,
        "kept after a failed POST"
    );

    // Answered: the counters that were sent are cleared.
    rig.server.reset().await;
    mount_ok(&rig.server).await;
    assert_eq!(rig.cycle().await, DELIVERED_ONE);
    let posts = rig.posts().await;
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["client_dropped"], json!(counted));
    assert_eq!(stored(&rig.owner()).dropped, DroppedCounts::default());

    // The next upload carries zeros: each drop is reported once.
    rig.fail(1);
    assert_eq!(rig.cycle().await, DELIVERED_ONE);
    let posts = rig.posts().await;
    assert_eq!(posts.len(), 2);
    assert_eq!(posts[1]["client_dropped"], json!(DroppedCounts::default()));
}

#[tokio::test]
async fn a_resend_after_a_lost_answer_keeps_its_event_id_and_leaves_on_duplicate() {
    let mut rig = Rig::opted_in().await;
    // The server takes the batch, but its answer does not arrive in time.
    mount(&rig.server, |request: &Request| {
        accept_all(request).set_delay(Duration::from_secs(60))
    })
    .await;
    rig.env.tuning.post_timeout = Duration::from_millis(1500);
    rig.env.tuning.max_retries = 0;
    let rows = rig.fail(1);
    assert_eq!(rig.cycle().await, CycleOutcome::GaveUp);
    assert_eq!(rig.paths_after(1).await, [INGEST]);
    let first = rig.posts().await;
    assert_eq!(queued(&rig.owner()), rows, "kept without an answer");

    // A later process on the same state root sends the same row again.
    rig.server.reset().await;
    let rig = rig.restart();
    mount(&rig.server, results(&["duplicate"])).await;
    assert_eq!(queued(&rig.owner()), rows, "the row survived the restart");
    assert_eq!(
        rig.cycle().await,
        CycleOutcome::Delivered {
            accepted: 0,
            duplicate: 1,
            rejected: 0,
        }
    );
    let second = rig.posts().await;
    assert_eq!(second.len(), 1);
    assert_eq!(second[0]["summaries"], first[0]["summaries"]);
    assert_eq!(
        second[0]["summaries"][0]["event_id"],
        json!(rows[0].event_id)
    );
    assert_eq!(stored(&rig.owner()).entries, []);
}

/// Every file under `directory`, by path.
fn files_under(directory: &Path, found: &mut BTreeMap<PathBuf, Vec<u8>>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_under(&path, found);
        } else {
            // The lock file may refuse a second reader; it holds no content.
            let bytes = std::fs::read(&path).unwrap_or_default();
            found.insert(path, bytes);
        }
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// What the mock hands out with every answer for a client to keep or send back.
const ISSUED: &str = "MARKER-ISSUED-BY-SERVER";

#[tokio::test]
async fn every_request_is_anonymous_and_nothing_a_server_issues_is_kept() {
    let rig = Rig::opted_in().await;
    // The server offers a cookie, a session token and a challenge every time.
    mount(&rig.server, |request: &Request| {
        accept_all(request)
            .insert_header("set-cookie", format!("sid={ISSUED}; Path=/").as_str())
            .insert_header("x-session-token", ISSUED)
            .insert_header(
                "www-authenticate",
                format!("Bearer realm=\"{ISSUED}\"").as_str(),
            )
    })
    .await;
    let mut sent = Vec::new();
    sent.push(rig.fail(1));
    let mut before = BTreeMap::new();
    files_under(rig.root(), &mut before);
    assert_eq!(rig.cycle().await, DELIVERED_ONE);
    sent.push(rig.fail(1));
    assert_eq!(rig.cycle().await, DELIVERED_ONE);

    // The full list of what was requested: the one route, twice, and each body
    // is exactly its batch. The second request answers nothing the first
    // response offered.
    assert_eq!(rig.paths().await, [INGEST; 2]);
    rig.assert_anonymous("delivery").await;
    let expected: Vec<Value> = sent
        .iter()
        .map(|rows| {
            json!(SummaryRequest {
                schema_version: SCHEMA_VERSION,
                client_dropped: DroppedCounts::default(),
                summaries: rows.clone(),
            })
        })
        .collect();
    assert_eq!(rig.posts().await, expected);

    // A row left waiting gives the scan something it must find.
    let waiting = rig.fail(1);
    let mut after = BTreeMap::new();
    files_under(rig.root(), &mut after);
    // The exporter created no file of its own: no token, id or cookie store.
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>()
    );
    assert!(!after
        .values()
        .any(|bytes| contains(bytes, ISSUED.as_bytes())));
    // Whatever id is in the queue file belongs to the row that is waiting.
    let queue = stored(&rig.owner());
    assert_eq!(queue.tracking, None);
    let waiting_rows: Vec<Summary> = queue
        .entries
        .into_iter()
        .map(|entry| entry.summary)
        .collect();
    assert_eq!(waiting_rows, waiting);
    let event_id = waiting[0].event_id.to_string();
    assert!(after
        .values()
        .any(|bytes| contains(bytes, event_id.as_bytes())));

    // Positive control for the header checks: the exporter's own client sends
    // the same request with one more header, and the mock records exactly that
    // header besides the allowed ones. So a credential, a `user-agent` or an
    // identifier on a real request would have failed the allowlist above.
    let extras = ["user-agent", "x-installation-id"];
    for extra in CREDENTIALS.into_iter().chain(extras) {
        rig.env
            .http
            .post(rig.env.endpoint.ingest_url())
            .header("content-type", "application/json")
            .header(extra, "control")
            .body("{}")
            .send()
            .await
            .unwrap();
        let requests = rig.server.received_requests().await.unwrap();
        let mut expected = HEADERS.to_vec();
        expected.push(extra);
        expected.sort_unstable();
        assert_eq!(header_names(requests.last().unwrap()), expected);
        assert!(requests.last().unwrap().headers.contains_key(extra));
    }
}
