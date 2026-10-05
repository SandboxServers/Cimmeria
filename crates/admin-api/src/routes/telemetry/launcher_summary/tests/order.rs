//! The order of the ingest's refusals: kill switch, then quota, then the
//! content type, then the query string, then the body's size, then the
//! body. Each test puts two faults in one request and checks which one
//! answers.

use super::super::dto::Verdict::Accepted;
use super::super::MAX_SUMMARY_BODY_BYTES;
use super::{batch, capture, content_type, element, json_headers, refusal, Env, Harness, ROUTE};

const PAUSED: &str = "Kill switch active — telemetry ingest is paused";
const CONTENT_TYPE: &str = "Content-Type must be application/json";
const QUERY: &str = "Query string not allowed";
const TOO_LARGE: &str = "Body is over 64 KiB";
const NOT_JSON: &str = "Body is not a JSON object";

/// A body one byte over the cap.
fn oversized() -> Vec<u8> {
    vec![b' '; MAX_SUMMARY_BODY_BYTES + 1]
}

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

/// **The kill switch and the quota answer before the body is read.** An
/// oversized request is a 503 while the switch is on and is not charged;
/// with the switch off and an allowance of one it is a 413 (the control:
/// admitted, it is refused for its size) and is charged, so the next one is
/// a 429, not a 413.
///
/// A handler that read the body first would answer 413 all three times.
#[test]
fn the_kill_switch_and_the_quota_answer_before_the_body_size() {
    let env = Env::install();
    let mut h = Harness::new();
    h.policy.per_ip = 1;

    env.set_kill_switch(true);
    let r = refusal(h.post(&oversized()).unwrap_err());
    assert_eq!((r.status, r.body.as_str()), (503, PAUSED), "{r:?}");

    env.set_kill_switch(false);
    let r = refusal(h.post(&oversized()).unwrap_err());
    assert_eq!((r.status, r.body.as_str()), (413, TOO_LARGE), "control");

    let r = refusal(h.post(&oversized()).unwrap_err());
    assert_eq!(r.status, 429, "{r:?}");
    assert_eq!(r.retry_after.as_deref(), Some("3601"));
}

/// Over quota, a request with a query string is a 429, not a 400. The first
/// request, inside the allowance of one, is the control: the same request
/// is the query's 400, and it spent the allowance.
#[test]
fn the_quota_answers_before_the_query() {
    let _env = Env::install();
    let mut h = Harness::new();
    h.policy.per_ip = 1;
    h.uri = format!("{ROUTE}?a=1");
    let body = batch(vec![element(1)]);

    let r = refusal(h.post_json(&body).unwrap_err());
    assert_eq!((r.status, r.body.as_str()), (400, QUERY), "control");
    assert_eq!(r.retry_after, None);

    let r = refusal(h.post_json(&body).unwrap_err());
    assert_eq!(r.status, 429, "{r:?}");
}

/// A wrong content type answers before a query string, and both answer
/// before the body's size: neither needs the body, so neither reads it.
/// Each line removes the fault that answered the line before.
#[test]
fn the_content_type_and_the_query_answer_before_the_body_size() {
    let _env = Env::install();
    let mut h = Harness::new();
    h.uri = format!("{ROUTE}?a=1");
    h.headers = content_type(Some("text/plain"));

    let r = refusal(h.post(&oversized()).unwrap_err());
    assert_eq!((r.status, r.body.as_str()), (415, CONTENT_TYPE));

    h.headers = json_headers();
    let r = refusal(h.post(&oversized()).unwrap_err());
    assert_eq!((r.status, r.body.as_str()), (400, QUERY));

    h.uri = ROUTE.to_string();
    let r = refusal(h.post(&oversized()).unwrap_err());
    assert_eq!((r.status, r.body.as_str()), (413, TOO_LARGE));
}

/// **A query string is a 400 before any row is written.** The valid
/// request posted with a query string (a marker, an empty one, a pair, an
/// encoded newline) is refused whole with a static body that repeats none
/// of it, writes no row and remembers no id. The control is the same body
/// to the bare path on the same ingest: accepted as new, and written.
#[test]
fn a_query_string_is_a_400_and_writes_no_row() {
    let _env = Env::install();
    let body = batch(vec![element(1)]);
    for query in ["?ZZMARKER", "?", "?a=1&b=2", "?%0Aforged"] {
        let mut h = Harness::new();
        h.uri = format!("{ROUTE}{query}");
        let (result, rows) = capture(|| h.post_json(&body));
        let r = refusal(result.expect_err(query));
        assert_eq!((r.status, r.body.as_str()), (400, QUERY), "{query}");
        assert_eq!(r.retry_after, None, "{query}");
        assert!(rows.is_empty(), "{query}: {rows:#?}");

        h.uri = ROUTE.to_string();
        let (result, rows) = capture(|| h.post_json(&body));
        assert_eq!(result.expect(query).results, [Accepted], "{query}: control");
        assert!(!rows.is_empty(), "{query}: control");
    }
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
