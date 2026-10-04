//! What a cycle sends, and what it does with the server's answer.
use super::*;
use serde_json::{json, Value};

fn fixture(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[tokio::test]
async fn a_queued_row_is_minted_for_posted_and_removed() {
    let mut rig = Rig::opted_in().await;
    rig.fix_mint_id();
    mount_ok(&rig.server).await;
    // Nothing queued: nothing sent.
    assert_eq!(rig.cycle().await, CycleOutcome::Empty);
    assert_eq!(rig.paths().await, [""; 0]);

    let rows = rig.fail(1);
    assert_eq!(rig.cycle().await, DELIVERED_ONE);
    assert_eq!(rig.paths().await, [MINT, INGEST]);
    let requests = rig.server.received_requests().await.unwrap();
    for request in &requests {
        assert_eq!(request.method.as_str(), "POST");
        assert_eq!(request.headers["content-type"], "application/json");
        assert!(!request.headers.contains_key("content-encoding"));
    }
    assert!(!requests[0].headers.contains_key("authorization"));
    assert_eq!(
        requests[1].headers["authorization"],
        format!("Bearer {TOKEN}").as_str()
    );
    assert_eq!(
        rig.bodies(MINT).await,
        [fixture(include_str!("../../fixtures/mint-request.json"))]
    );
    let sent: SummaryRequest = serde_json::from_slice(&requests[1].body).unwrap();
    assert_eq!(sent.summaries, rows);
    assert_eq!(sent.client_dropped, DroppedCounts::default());
    assert_eq!(stored(&rig.owner()).entries, []);
    assert_eq!(rig.probe.sleeps(), []);
    // The row is gone, so the next cycle has nothing to send.
    assert_eq!(rig.cycle().await, CycleOutcome::Empty);
    assert_eq!(rig.paths().await.len(), 2);
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
    assert_eq!(rig.paths().await, [MINT, INGEST]);
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
    assert_eq!(rig.paths().await, [MINT, INGEST]);

    // No endpoint at all: nothing is sent and nothing is kept.
    rig.fail(1);
    rig.owner().configure_summaries(config(None));
    assert_eq!(rig.cycle().await, CycleOutcome::SkippedNoEndpoint);
    assert_eq!(queue_bytes(&rig.owner()), None);
    assert_eq!(rig.paths().await, [MINT, INGEST]);
}

#[tokio::test]
async fn every_verdict_removes_its_row_and_a_rejection_is_counted() {
    let rig = Rig::opted_in().await;
    mount(&rig.server, MINT, minted()).await;
    let answer = fixture(include_str!("../../fixtures/response-mixed.json"));
    mount(
        &rig.server,
        INGEST,
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
    assert_eq!(rig.paths().await, [MINT, INGEST]);
}

#[tokio::test]
async fn a_body_refused_for_good_is_dropped_and_counted() {
    for status in [400, 413] {
        let rig = Rig::opted_in().await;
        mount(&rig.server, MINT, minted()).await;
        mount(&rig.server, INGEST, ResponseTemplate::new(status)).await;
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
        assert_eq!(on_disk.entries, []);
        assert_eq!(on_disk.dropped.rejected, 2);
        assert_eq!(rig.paths().await, [MINT, INGEST], "no retry");
    }
}

#[tokio::test]
async fn drop_counters_are_sent_until_a_post_is_answered_and_then_no_more() {
    let rig = Rig::opted_in().await;
    mount(&rig.server, MINT, minted()).await;
    mount(&rig.server, INGEST, ResponseTemplate::new(500)).await;
    rig.fail(1);
    let counted = DroppedCounts {
        overflow: 5,
        expired: 2,
        rejected: 1,
    };
    lock(&rig.owner().summaries).queue.dropped = counted;
    assert_eq!(rig.cycle().await, CycleOutcome::GaveUp);
    let posts = rig.bodies(INGEST).await;
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
    let posts = rig.bodies(INGEST).await;
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["client_dropped"], json!(counted));
    assert_eq!(stored(&rig.owner()).dropped, DroppedCounts::default());

    // The next upload carries zeros: each drop is reported once.
    rig.fail(1);
    assert_eq!(rig.cycle().await, DELIVERED_ONE);
    let posts = rig.bodies(INGEST).await;
    assert_eq!(posts.len(), 2);
    assert_eq!(posts[1]["client_dropped"], json!(DroppedCounts::default()));
}

#[tokio::test]
async fn a_resend_after_a_lost_answer_keeps_its_event_id_and_leaves_on_duplicate() {
    let mut rig = Rig::opted_in().await;
    mount(&rig.server, MINT, minted()).await;
    // The server takes the batch, but its answer does not arrive in time.
    mount(&rig.server, INGEST, |request: &Request| {
        accept_all(request).set_delay(Duration::from_secs(60))
    })
    .await;
    rig.env.tuning.post_timeout = Duration::from_millis(1500);
    rig.env.tuning.max_retries = 0;
    let rows = rig.fail(1);
    assert_eq!(rig.cycle().await, CycleOutcome::GaveUp);
    assert_eq!(rig.paths_after(2).await, [MINT, INGEST]);
    let first = rig.bodies(INGEST).await;
    assert_eq!(queued(&rig.owner()), rows, "kept without an answer");

    // A later process on the same state root sends the same row again.
    rig.server.reset().await;
    let rig = rig.restart();
    mount(&rig.server, MINT, minted()).await;
    mount(&rig.server, INGEST, results(&["duplicate"])).await;
    assert_eq!(queued(&rig.owner()), rows, "the row survived the restart");
    assert_eq!(
        rig.cycle().await,
        CycleOutcome::Delivered {
            accepted: 0,
            duplicate: 1,
            rejected: 0,
        }
    );
    let second = rig.bodies(INGEST).await;
    assert_eq!(second.len(), 1);
    assert_eq!(second[0]["summaries"], first[0]["summaries"]);
    assert_eq!(
        second[0]["summaries"][0]["event_id"],
        json!(rows[0].event_id)
    );
    assert_eq!(stored(&rig.owner()).entries, []);
}

fn files_under(directory: &Path, found: &mut Vec<Vec<u8>>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_under(&path, found);
        } else {
            // The lock file may refuse a second reader; it holds no content.
            found.push(std::fs::read(&path).unwrap_or_default());
        }
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[tokio::test]
async fn only_the_two_routes_are_called_and_no_mint_id_or_token_is_kept() {
    // The mint id is random for every mint, as in production.
    let rig = Rig::opted_in().await;
    mount_ok(&rig.server).await;
    for _ in 0..2 {
        rig.fail(1);
        assert_eq!(rig.cycle().await, DELIVERED_ONE);
    }
    assert_eq!(rig.paths().await, [MINT, INGEST, MINT, INGEST]);
    let ids: Vec<Uuid> = rig
        .bodies(MINT)
        .await
        .iter()
        .map(|body| serde_json::from_value(body["install_id"].clone()).unwrap())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
    assert!(ids
        .iter()
        .all(|id| !id.is_nil() && *id != GOLDEN_INSTALL_ID));

    // A row left waiting gives the scan something it must find.
    let waiting = rig.fail(1);
    let mut files = Vec::new();
    files_under(rig.root(), &mut files);
    // The bearer token was sent twice and is in no file either.
    let requests = rig.server.received_requests().await.unwrap();
    assert_eq!(
        requests[3].headers["authorization"],
        format!("Bearer {TOKEN}").as_str()
    );
    assert!(!files.iter().any(|bytes| contains(bytes, TOKEN.as_bytes())));
    for id in ids {
        for form in [
            id.hyphenated().to_string().into_bytes(),
            id.simple().to_string().into_bytes(),
            id.hyphenated().to_string().to_uppercase().into_bytes(),
            id.as_bytes().to_vec(),
        ] {
            assert!(!files.iter().any(|bytes| contains(bytes, &form)));
        }
    }
    let event_id = waiting[0].event_id.to_string();
    assert!(files
        .iter()
        .any(|bytes| contains(bytes, event_id.as_bytes())));
}
