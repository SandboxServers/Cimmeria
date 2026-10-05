//! The order of the ingest's refusals: kill switch, then quota, then the
//! content type, then the body. Each test puts two faults in one request
//! and checks which one answers.

use super::super::dto::Verdict::Accepted;
use super::{batch, capture, content_type, element, json_headers, refusal, Env, Harness};

const PAUSED: &str = "Kill switch active — telemetry ingest is paused";
const CONTENT_TYPE: &str = "Content-Type must be application/json";
const NOT_JSON: &str = "Body is not a JSON object";

/// With the kill switch on, a request with a wrong content type and a
/// malformed body is a 503 with `Retry-After`, and it is not charged to the
/// quota: with an allowance of one, the next request after the switch is
/// cleared still gets through. The request after that is the control
/// showing the allowance really was one.
#[test]
fn the_kill_switch_answers_before_the_quota_and_the_content_type() {
    let env = Env::install();
    let mut h = Harness::new();
    h.policy.per_ip = 1;

    env.set_kill_switch(true);
    h.headers = content_type(Some("text/plain"));
    let (result, rows) = capture(|| h.post(b"not json"));
    let r = refusal(result.unwrap_err());
    assert_eq!(r.status, 503, "{r:?}");
    assert_eq!(r.retry_after.as_deref(), Some("60"));
    assert_eq!(r.body, PAUSED);
    assert!(rows.is_empty(), "a paused ingest wrote rows: {rows:#?}");

    // A valid request is paused too.
    h.headers = json_headers();
    let r = refusal(h.post_json(&batch(vec![element(1)])).unwrap_err());
    assert_eq!(r.status, 503, "{r:?}");

    env.set_kill_switch(false);
    assert_eq!(
        h.verdicts(&batch(vec![element(1)])),
        [Accepted],
        "the paused requests must not have spent the allowance or kept the id"
    );
    let r = refusal(h.post_json(&batch(vec![element(2)])).unwrap_err());
    assert_eq!(r.status, 429, "control: the allowance is one, {r:?}");
}

/// Over quota, a request with a wrong content type is a 429 with
/// `Retry-After`, not a 415: the quota is charged before anything the
/// caller sent is looked at. The first request, inside the allowance, is
/// the control: the same request is a 415, and it spent the allowance.
#[test]
fn the_quota_answers_before_the_content_type() {
    let _env = Env::install();
    let mut h = Harness::new();
    h.policy.per_ip = 1;
    h.headers = content_type(Some("text/plain"));
    let body = batch(vec![element(1)]);

    let r = refusal(h.post_json(&body).unwrap_err());
    assert_eq!(r.status, 415, "control: {r:?}");
    assert_eq!(r.retry_after, None);

    let r = refusal(h.post_json(&body).unwrap_err());
    assert_eq!(r.status, 429, "{r:?}");
    assert_eq!(r.retry_after.as_deref(), Some("3601"));
    assert_eq!(r.body, "summary/ip quota exceeded — retry in 3601s");
}

/// A malformed body under a wrong content type is a 415, not a 400: the
/// body is not parsed for a request that does not claim to be JSON. The
/// same body as `application/json` is the control, a 400.
#[test]
fn the_content_type_answers_before_the_body() {
    let _env = Env::install();
    let mut h = Harness::new();
    let malformed = b"{ \"schema_version\": 1, ";

    let r = refusal(h.post(malformed).unwrap_err());
    assert_eq!(r.status, 400, "control: {r:?}");
    assert_eq!(r.body, NOT_JSON);

    h.headers = content_type(Some("text/plain"));
    let r = refusal(h.post(malformed).unwrap_err());
    assert_eq!(r.status, 415, "{r:?}");
    assert_eq!(r.body, CONTENT_TYPE);
}

/// The envelope answers before any element is judged: a request whose
/// envelope is wrong is a 400 even though its one element is valid, and
/// that element is neither written nor remembered, so the same element in
/// a correct envelope is then accepted as new.
#[test]
fn the_envelope_answers_before_the_elements() {
    let _env = Env::install();
    let h = Harness::new();
    let mut wrong = batch(vec![element(1)]);
    wrong["schema_version"] = serde_json::json!(2);

    let (result, rows) = capture(|| h.post_json(&wrong));
    let r = refusal(result.unwrap_err());
    assert_eq!(r.status, 400, "{r:?}");
    assert!(rows.is_empty(), "{rows:#?}");

    assert_eq!(h.verdicts(&batch(vec![element(1)])), [Accepted], "control");
}

/// A request refused for its envelope reached neither validation nor the
/// log: no batch row, no summary row. The valid request is the control.
#[test]
fn a_refused_request_writes_no_rows() {
    let _env = Env::install();
    let h = Harness::new();

    let (result, rows) = capture(|| h.post(b"not json"));
    assert_eq!(refusal(result.unwrap_err()).status, 400);
    assert!(rows.is_empty(), "{rows:#?}");

    let (result, rows) = capture(|| h.post_json(&batch(vec![element(1)])));
    result.expect("control");
    assert!(!rows.is_empty());
}
