//! The ingest's per-address quota and the knob that sizes it.

use std::time::Duration;

use super::super::dto::{SummaryError, Verdict::Accepted};
use super::super::handlers::IngestPolicy;
use super::{batch, element, refusal, Env, Harness, ENV_SUMMARY_QUOTA};

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
        SummaryError::Auth(crate::routes::dev_session::AuthError::QuotaExceeded(q)) => {
            assert_eq!(q.scope, "summary/ip");
        }
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

/// `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP` sizes the allowance: 120 when
/// unset or unreadable (an operator typo must not take the route down), the
/// value when set, 0 to disable.
///
/// The values tried are 0 or large: under `cargo test` the socket tests in
/// `routes` share the process and read this variable without the env lock,
/// and a small allowance set here could turn one of their requests into a
/// 429.
#[test]
fn the_quota_knob_defaults_to_120_and_reads_its_value() {
    let _env = Env::install();
    let prev = std::env::var(ENV_SUMMARY_QUOTA).ok();
    let mut seen = Vec::new();
    for value in [None, Some("700"), Some("0"), Some(" 300 "), Some("many")] {
        match value {
            Some(v) => std::env::set_var(ENV_SUMMARY_QUOTA, v),
            None => std::env::remove_var(ENV_SUMMARY_QUOTA),
        }
        seen.push(IngestPolicy::from_env().per_ip);
    }
    match prev {
        Some(v) => std::env::set_var(ENV_SUMMARY_QUOTA, v),
        None => std::env::remove_var(ENV_SUMMARY_QUOTA),
    }
    assert_eq!(seen, [120, 700, 0, 300, 120]);
}
