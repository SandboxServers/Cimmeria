//! The order of the ingest's refusals: kill switch, then quota, then the
//! token, then the body. Each test puts two faults in one request and
//! checks which one answers.

use axum::http::HeaderMap;

use super::{batch, bearer, capture, element, refusal, Env, Harness};

/// With the kill switch on, a request with no `Authorization` at all is a
/// 503 with `Retry-After`, and it is not charged to the quota: with an
/// allowance of one, the next request after the switch is cleared still
/// gets through. The request after that is the control showing the
/// allowance really was one.
#[test]
fn the_kill_switch_answers_before_the_quota_and_the_token() {
    let env = Env::install();
    let mut h = Harness::new();
    h.policy.per_ip = 1;
    let valid = h.headers.clone();

    env.set_kill_switch(true);
    h.headers = HeaderMap::new();
    let (result, rows) = capture(|| h.post_json(&batch(vec![element(1)])));
    let r = refusal(result.unwrap_err());
    assert_eq!(r.status, 503, "{r:?}");
    assert_eq!(r.retry_after.as_deref(), Some("60"));
    assert!(rows.is_empty(), "a paused ingest wrote rows: {rows:#?}");

    env.set_kill_switch(false);
    h.headers = valid;
    h.post_json(&batch(vec![element(1)]))
        .expect("the paused request must not have spent the allowance");
    let r = refusal(h.post_json(&batch(vec![element(2)])).unwrap_err());
    assert_eq!(r.status, 429, "control: the allowance is one, {r:?}");
}

/// Over quota, a request with a garbage token is a 429 with `Retry-After`,
/// not a 401: the quota is charged before the token is looked at, so
/// guessing tokens cannot be done faster than the allowance. The first
/// request, inside the allowance, is the control: the same token is a 401.
#[test]
fn the_quota_answers_before_the_token() {
    let _env = Env::install();
    let mut h = Harness::new();
    h.policy.per_ip = 1;
    h.headers = bearer("garbage.token");

    let r = refusal(h.post_json(&batch(vec![element(1)])).unwrap_err());
    assert_eq!(r.status, 401, "control: {r:?}");
    assert_eq!(r.retry_after, None);

    let r = refusal(h.post_json(&batch(vec![element(1)])).unwrap_err());
    assert_eq!(r.status, 429, "{r:?}");
    assert_eq!(r.retry_after.as_deref(), Some("3601"));
}

/// A malformed body with no token is a 401, not a 400: the body is not
/// parsed for a caller who has not shown a token. The same body with a
/// valid token is the control, a 400.
#[test]
fn the_token_answers_before_the_body() {
    let _env = Env::install();
    let mut h = Harness::new();
    let malformed = b"{ \"schema_version\": 1, ";

    let r = refusal(h.post(malformed).unwrap_err());
    assert_eq!(r.status, 400, "control: {r:?}");

    h.headers = HeaderMap::new();
    let r = refusal(h.post(malformed).unwrap_err());
    assert_eq!(r.status, 401, "{r:?}");
    assert_eq!(r.body, "Missing or malformed Authorization header");
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
