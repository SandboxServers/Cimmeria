//! Answers a hostile or broken server could give. None removes a row, none
//! makes the exporter send anything but its own body, and the exporter carries
//! on afterwards.
use super::*;

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

fn two_accepted() -> CycleOutcome {
    CycleOutcome::Delivered {
        accepted: 2,
        duplicate: 0,
        rejected: 0,
    }
}

#[tokio::test]
async fn a_hostile_answer_removes_nothing_and_the_exporter_carries_on() {
    let cases = [
        ("over 4 KiB", padded(TWO_ACCEPTED, 4 * 1024 + 1)),
        ("too few results", results(&["accepted"])),
        ("too many results", results(&["accepted"; 3])),
        ("unknown result", results(&["accepted", "stored"])),
        ("results not a list", raw(r#"{"results":"accepted"}"#)),
        ("not JSON", raw("ok")),
        ("empty", raw("")),
        (
            "unknown key",
            raw(r#"{"results":["accepted","accepted"],"note":"x"}"#),
        ),
        // What a token-issuing server would answer is not a verdict either.
        (
            "a token",
            raw(r#"{"results":["accepted","accepted"],"token":"abc"}"#),
        ),
    ];
    for (case, answer) in cases {
        let rig = Rig::opted_in().await;
        // Every answer also offers a cookie and asks for credentials.
        let answer = answer
            .insert_header("set-cookie", "sid=hostile; Path=/")
            .insert_header("www-authenticate", "Bearer realm=\"hostile\"");
        mount(&rig.server, answer).await;
        rig.fail(2);
        let before = queue_bytes(&rig.owner());

        // Counted as transient: retried within the budget, then left for later.
        assert_eq!(rig.cycle().await, CycleOutcome::GaveUp, "{case}");
        assert_eq!(rig.paths().await, [INGEST; 3], "{case}");
        assert_eq!(rig.probe.sleeps(), BACKOFFS, "{case}");
        assert_eq!(queue_bytes(&rig.owner()), before, "{case}");
        // The retries answered neither the cookie nor the challenge.
        rig.assert_anonymous(case).await;

        // Still alive: the same rows leave once the server answers properly.
        rig.server.reset().await;
        mount_ok(&rig.server).await;
        assert_eq!(rig.cycle().await, two_accepted(), "{case}");
        assert_eq!(rig.paths().await, [INGEST], "{case}");
        rig.assert_anonymous(case).await;
    }
}

// Positive control for the size limit: one byte less is accepted.
#[tokio::test]
async fn an_answer_at_the_size_limit_is_accepted() {
    let rig = Rig::opted_in().await;
    mount(&rig.server, padded(TWO_ACCEPTED, 4 * 1024)).await;
    rig.fail(2);
    assert_eq!(rig.cycle().await, two_accepted());
    assert_eq!(rig.paths().await, [INGEST]);
    assert_eq!(rig.probe.sleeps(), []);
}
