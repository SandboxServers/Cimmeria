//! The ingest's per-address quota and the knob that sizes it.

use std::time::Duration;

use super::super::dto::{SummaryError, Verdict::Accepted};
use super::super::handlers::IngestPolicy;
use super::{batch, capture, content_type, element, refusal, Env, Harness, ENV_SUMMARY_QUOTA};

const ENV_WINDOW: &str = "CIMMERIA_TELEMETRY_QUOTA_WINDOW_SECS";

/// Run `f` with `name` set to `value` (or unset), then put back what was
/// there. The caller holds an [`Env`], and so the crate's env lock.
fn with_var<T>(name: &str, value: Option<&str>, f: impl FnOnce() -> T) -> T {
    let prev = std::env::var(name).ok();
    match value {
        Some(v) => std::env::set_var(name, v),
        None => std::env::remove_var(name),
    }
    let out = f();
    match prev {
        Some(v) => std::env::set_var(name, v),
        None => std::env::remove_var(name),
    }
    out
}

/// **The default is low: 12 requests an hour per address.** With the policy
/// an operator gets by setting nothing, twelve requests from one address
/// are served and the thirteenth is a 429 that says to come back when the
/// hour is up. A different address is still served.
///
/// Raising `DEFAULT_SUMMARY_PER_IP` fails the thirteenth request's
/// assertion.
#[test]
fn the_default_allowance_is_twelve_requests_an_hour_per_address() {
    let _env = Env::install();
    let policy = with_var(ENV_SUMMARY_QUOTA, None, || {
        with_var(ENV_WINDOW, None, IngestPolicy::from_env)
    });
    assert_eq!(policy.window, Duration::from_secs(3_600));
    let mut h = Harness::new();
    h.policy = policy;
    let post = |h: &Harness, n: u32| h.post_json(&batch(vec![element(n)]));

    for n in 1..=12 {
        let served = post(&h, n).unwrap_or_else(|e| panic!("request {n} was refused: {e:?}"));
        assert_eq!(served.results, [Accepted], "request {n}");
    }
    let (result, rows) = capture(|| post(&h, 13));
    let r = refusal(result.expect_err("the thirteenth request"));
    assert_eq!(r.status, 429, "{r:?}");
    assert_eq!(r.retry_after.as_deref(), Some("3601"));
    assert_eq!(r.body, "summary/ip quota exceeded — retry in 3601s");
    assert!(
        rows.is_empty(),
        "an over-quota request wrote rows: {rows:#?}"
    );

    h.peer = "203.0.113.78".parse().unwrap();
    assert_eq!(post(&h, 13).unwrap().results, [Accepted], "another address");
}

/// **Refused requests spend the allowance.** The quota is charged before
/// anything is parsed, so three requests refused for three different
/// reasons use up an allowance of three, and the valid request after them
/// is a 429. The control is the same valid request on a fresh ingest with
/// the same allowance: accepted.
#[test]
fn malformed_requests_spend_the_allowance() {
    let _env = Env::install();
    let valid = batch(vec![element(1)]);
    let mut h = Harness::new();
    h.policy.per_ip = 3;
    assert_eq!(h.verdicts(&valid), [Accepted], "control");

    let mut h = Harness::new();
    h.policy.per_ip = 3;
    let json = h.headers.clone();
    assert_eq!(refusal(h.post(b"not json").unwrap_err()).status, 400);
    h.headers = content_type(Some("text/plain"));
    assert_eq!(refusal(h.post_json(&valid).unwrap_err()).status, 415);
    h.headers = content_type(None);
    assert_eq!(refusal(h.post(b"").unwrap_err()).status, 415);

    h.headers = json;
    let r = refusal(h.post_json(&valid).unwrap_err());
    assert_eq!(r.status, 429, "{r:?}");
    assert_eq!(r.retry_after.as_deref(), Some("3601"));
}

/// With an allowance of two, the third request from an address inside the
/// window is a 429 naming the summary quota, another address is unaffected,
/// and the allowance returns when the window ends.
#[test]
fn the_third_request_past_an_allowance_of_two_is_a_429() {
    let _env = Env::install();
    let mut h = Harness::new();
    h.policy.per_ip = 2;
    h.policy.window = Duration::from_secs(60);
    let post = |h: &Harness, n: u32| h.post_json(&batch(vec![element(n)]));

    assert_eq!(post(&h, 1).unwrap().results, [Accepted]);
    assert_eq!(post(&h, 2).unwrap().results, [Accepted]);
    let err = post(&h, 3).unwrap_err();
    match &err {
        SummaryError::OverQuota(q) => assert_eq!(q.scope, "summary/ip"),
        other => panic!("expected the summary quota, got {other:?}"),
    }
    let r = refusal(err);
    assert_eq!(r.status, 429);
    assert_eq!(r.retry_after.as_deref(), Some("61"));

    let own = h.peer;
    h.peer = "203.0.113.78".parse().unwrap();
    assert_eq!(post(&h, 3).unwrap().results, [Accepted], "another address");

    h.peer = own;
    h.now += Duration::from_secs(60);
    assert_eq!(post(&h, 4).unwrap().results, [Accepted], "a new window");
}

/// An allowance of 0 is the documented "disabled", not "refuse everything".
#[test]
fn an_allowance_of_zero_disables_the_quota() {
    let _env = Env::install();
    let h = Harness::new();
    assert_eq!(h.policy.per_ip, 0);
    for n in 1..=200 {
        h.post_json(&batch(vec![element(n)]))
            .unwrap_or_else(|e| panic!("request {n} was refused: {e:?}"));
    }
}

/// `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP` sizes the allowance: 12 when
/// unset or unreadable (an operator typo must neither take the route down
/// nor open it up), the value when set, 0 to disable.
///
/// The values tried are 0, the default or large: under `cargo test` the
/// socket tests in `routes` share the process and read this variable
/// without the env lock, and an allowance set here below the default could
/// turn one of their requests into a 429.
#[test]
fn the_quota_knob_defaults_to_12_and_reads_its_value() {
    let _env = Env::install();
    let seen = [None, Some("700"), Some("0"), Some(" 300 "), Some("many")]
        .map(|value| with_var(ENV_SUMMARY_QUOTA, value, || IngestPolicy::from_env().per_ip));
    assert_eq!(seen, [12, 700, 0, 300, 12]);
}
