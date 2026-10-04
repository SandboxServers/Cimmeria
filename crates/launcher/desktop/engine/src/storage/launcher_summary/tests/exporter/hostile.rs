//! Answers a hostile or broken server could give. None removes a row, none
//! reaches a request header, and the exporter carries on afterwards.
use super::*;
use serde_json::json;

const TWO_ACCEPTED: &str = r#"{"results":["accepted","accepted"]}"#;

/// `json` followed by spaces up to `total` bytes: still valid JSON if read whole.
fn padded(json: &str, total: usize) -> ResponseTemplate {
    let mut body = json.as_bytes().to_vec();
    body.resize(total, b' ');
    raw(body)
}

fn raw(body: impl Into<Vec<u8>>) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body, "application/json")
}

fn token(value: &str) -> ResponseTemplate {
    raw(json!({ "token": value }).to_string())
}

#[tokio::test]
async fn a_hostile_answer_removes_nothing_and_the_exporter_carries_on() {
    let mint_json = json!({ "token": TOKEN }).to_string();
    let cases = [
        ("mint over 8 KiB", MINT, padded(&mint_json, 8 * 1024 + 1)),
        ("mint not JSON", MINT, raw("token=abc")),
        ("mint without a token", MINT, raw(r#"{"session_id":"x"}"#)),
        ("empty token", MINT, token("")),
        ("token with CR LF", MINT, token("abc\r\nx-injected: 1")),
        ("token with a control byte", MINT, token("abc\u{0}def")),
        ("token over 4096 bytes", MINT, token(&"a".repeat(4097))),
        (
            "ingest over 4 KiB",
            INGEST,
            padded(TWO_ACCEPTED, 4 * 1024 + 1),
        ),
        ("too few results", INGEST, results(&["accepted"])),
        ("too many results", INGEST, results(&["accepted"; 3])),
        ("unknown result", INGEST, results(&["accepted", "stored"])),
        ("ingest not JSON", INGEST, raw("ok")),
        (
            "unknown key",
            INGEST,
            raw(r#"{"results":["accepted","accepted"],"note":"x"}"#),
        ),
    ];
    for (case, route, answer) in cases {
        let rig = Rig::opted_in().await;
        if route == INGEST {
            mount(&rig.server, MINT, minted()).await;
        }
        mount(&rig.server, route, answer).await;
        rig.fail(2);
        let before = queue_bytes(&rig.owner());

        // Counted as transient: retried within the budget, then left for later.
        assert_eq!(rig.cycle().await, CycleOutcome::GaveUp, "{case}");
        let sent = if route == MINT {
            vec![MINT; 3]
        } else {
            [MINT, INGEST].repeat(3)
        };
        assert_eq!(rig.paths().await, sent, "{case}");
        assert_eq!(rig.probe.sleeps(), BACKOFFS, "{case}");
        assert_eq!(queue_bytes(&rig.owner()), before, "{case}");
        for request in rig.server.received_requests().await.unwrap() {
            assert!(!request.headers.contains_key("x-injected"), "{case}");
        }

        // Still alive: the same rows leave once the server answers properly.
        rig.server.reset().await;
        mount_ok(&rig.server).await;
        assert_eq!(
            rig.cycle().await,
            CycleOutcome::Delivered {
                accepted: 2,
                duplicate: 0,
                rejected: 0,
            },
            "{case}"
        );
    }
}

// Positive controls for the three size limits: one byte less is accepted.
#[tokio::test]
async fn answers_at_the_size_limits_are_accepted() {
    let longest = "a".repeat(4096);
    let mint_json = json!({ "token": TOKEN }).to_string();
    let cases = [
        (padded(&mint_json, 8 * 1024), accept_all(&two_rows()), TOKEN),
        (minted(), padded(TWO_ACCEPTED, 4 * 1024), TOKEN),
        (token(&longest), accept_all(&two_rows()), longest.as_str()),
    ];
    for (case, (mint, ingest, bearer)) in cases.into_iter().enumerate() {
        let rig = Rig::opted_in().await;
        mount(&rig.server, MINT, mint).await;
        mount(&rig.server, INGEST, ingest).await;
        rig.fail(2);
        assert_eq!(
            rig.cycle().await,
            CycleOutcome::Delivered {
                accepted: 2,
                duplicate: 0,
                rejected: 0,
            },
            "case {case}"
        );
        let requests = rig.server.received_requests().await.unwrap();
        assert_eq!(rig.paths().await, [MINT, INGEST], "case {case}");
        assert_eq!(
            requests[1].headers["authorization"],
            format!("Bearer {bearer}").as_str()
        );
    }
}

/// A stand-in request with two summaries, for the canned `accept_all` answer.
fn two_rows() -> Request {
    Request {
        url: "http://localhost/".parse().unwrap(),
        method: "POST".parse().unwrap(),
        headers: Default::default(),
        body: br#"{"summaries":[0,0]}"#.to_vec(),
    }
}
