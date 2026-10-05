//! The ingest's per-address quota and the knob that sizes it.

use std::time::Duration;

use crate::routes::dev_session::QuotaPolicy;

use super::super::dto::{SummaryError, Verdict::Accepted};
use super::super::handlers::IngestPolicy;
use super::super::MAX_SUMMARY_BODY_BYTES;
use super::{
    batch, capture, content_type, element, refusal, Env, Harness, ENV_KILL_SWITCH,
    ENV_SUMMARY_QUOTA,
};

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

/// **The default is 12 requests a minute per address.** With the policy
/// an operator gets by setting nothing, twelve requests from one address
/// spread over 55 seconds are served and a thirteenth inside the same
/// minute is a 429. Its `Retry-After` is what is left of the minute, so it
/// is never more than 61 seconds. A different address is still served at
/// that moment, and 61 seconds after its first request the refused address
/// is served again.
///
/// The last request pins the window: with the mint's hour it is a 429. The
/// thirteenth pins the count: raising `DEFAULT_SUMMARY_PER_IP` serves it,
/// and lowering it refuses one of the twelve.
#[test]
fn the_default_allowance_is_twelve_requests_a_minute_per_address() {
    let _env = Env::install();
    let mut h = Harness::new();
    h.policy = with_var(ENV_SUMMARY_QUOTA, None, IngestPolicy::from_env);
    let first = h.now;
    let post = |h: &Harness, n: u32| h.post_json(&batch(vec![element(n)]));

    for n in 0..12 {
        h.now = first + Duration::from_secs(u64::from(n) * 5);
        let served = post(&h, n).unwrap_or_else(|e| panic!("request {n} was refused: {e:?}"));
        assert_eq!(served.results, [Accepted], "request {n}");
    }
    h.now = first + Duration::from_secs(59);
    let (result, rows) = capture(|| post(&h, 12));
    let r = refusal(result.expect_err("the thirteenth request"));
    assert_eq!(r.status, 429, "{r:?}");
    assert_eq!(r.retry_after.as_deref(), Some("2"));
    assert_eq!(r.body, "summary/ip quota exceeded — retry in 2s");
    assert!(
        rows.is_empty(),
        "an over-quota request wrote rows: {rows:#?}"
    );

    let own = h.peer;
    h.peer = "203.0.113.78".parse().unwrap();
    assert_eq!(post(&h, 13).unwrap().results, [Accepted], "another address");

    h.peer = own;
    h.now = first + Duration::from_secs(61);
    assert_eq!(
        post(&h, 14).expect("a minute later").results,
        [Accepted],
        "the allowance returns after a minute, not after an hour"
    );
}

/// **The allowance returns a minute after the address's first request.**
/// With the default policy and the allowance spent in a burst, a request 59
/// seconds later is still a 429 (the control: the minute has not ended) and
/// one 61 seconds after the first is served. That starts a new minute with
/// a full allowance: eleven more are served and the next is a 429.
///
/// Only statuses are asserted, so with the mint's hour as the window the
/// failure is the request at 61 seconds.
#[test]
fn the_default_allowance_returns_a_minute_after_the_first_request() {
    let _env = Env::install();
    let mut h = Harness::new();
    h.policy = with_var(ENV_SUMMARY_QUOTA, None, IngestPolicy::from_env);
    let first = h.now;
    let status = |h: &Harness, n: u32| match h.post_json(&batch(vec![element(n)])) {
        Ok(_) => 200,
        Err(e) => refusal(e).status,
    };

    let burst: Vec<_> = (0..13).map(|n| status(&h, n)).collect();
    assert_eq!(burst[..12], [200; 12]);
    assert_eq!(burst[12], 429);
    h.now = first + Duration::from_secs(59);
    assert_eq!(status(&h, 13), 429, "control: 59 s in");

    h.now = first + Duration::from_secs(61);
    assert_eq!(status(&h, 14), 200, "61 s after the first request");
    let next: Vec<_> = (15..27).map(|n| status(&h, n)).collect();
    assert_eq!(next[..11], [200; 11], "a full allowance in the new minute");
    assert_eq!(next[11], 429);
}

/// **`Retry-After` is at most 61 seconds.** The longest wait is the one a
/// burst gets: thirteen requests at one instant, with the default policy,
/// and the thirteenth is told to come back in 61 seconds (the whole
/// window, rounded up so a caller that waits exactly that long lands in
/// the next one). Twelve at that instant are the control.
#[test]
fn a_burst_past_the_default_allowance_waits_61_seconds_at_most() {
    let _env = Env::install();
    let mut h = Harness::new();
    h.policy = with_var(ENV_SUMMARY_QUOTA, None, IngestPolicy::from_env);
    let post = |h: &Harness, n: u32| h.post_json(&batch(vec![element(n)]));

    for n in 0..12 {
        assert_eq!(post(&h, n).unwrap().results, [Accepted], "request {n}");
    }
    let r = refusal(post(&h, 12).expect_err("the thirteenth request"));
    assert_eq!(r.status, 429, "{r:?}");
    assert_eq!(r.retry_after.as_deref(), Some("61"));
    assert_eq!(r.body, "summary/ip quota exceeded — retry in 61s");
}

/// **The window is this route's own.** The dev-session mint and refresh
/// quotas take their window from `CIMMERIA_TELEMETRY_QUOTA_WINDOW_SECS`
/// (an hour by default); the summary quota is a fixed minute and does not
/// read it. With that variable set to two hours and an allowance of one,
/// the second request 59 seconds in is a 429 and a third at 61 seconds is
/// served.
///
/// Control: under the same variable the mint's policy does have a two-hour
/// window, so the variable is spelled right and read by the code it
/// belongs to. Unset, the mint's window is still its hour: this route
/// changed nothing there.
#[test]
fn the_mint_quota_window_knob_does_not_move_the_summary_window() {
    let _env = Env::install();
    let mint_window = |value| with_var(ENV_WINDOW, value, || QuotaPolicy::from_env().window);
    assert_eq!(mint_window(None), Duration::from_secs(3_600), "control");
    assert_eq!(
        mint_window(Some("7200")),
        Duration::from_secs(7_200),
        "control"
    );

    with_var(ENV_WINDOW, Some("7200"), || {
        let mut h = Harness::new();
        h.policy = IngestPolicy::from_env();
        h.policy.per_ip = 1;
        let first = h.now;
        let post = |h: &Harness, n: u32| h.post_json(&batch(vec![element(n)]));

        assert_eq!(post(&h, 1).unwrap().results, [Accepted]);
        h.now = first + Duration::from_secs(59);
        let r = refusal(post(&h, 2).unwrap_err());
        assert_eq!(r.status, 429, "{r:?}");
        assert_eq!(r.retry_after.as_deref(), Some("2"));
        h.now = first + Duration::from_secs(61);
        assert_eq!(post(&h, 3).unwrap().results, [Accepted], "a new minute");
    });
}

/// **A blank variable is an unset one.** `docker/compose.yml` passes the
/// kill switch and the summary quota as `${VAR:-}`, so a deployment whose
/// `.env` names neither hands the server both variables set to the empty
/// string. That has to mean the defaults: an allowance of twelve, and a
/// request that is served.
///
/// Controls: with a value in it each variable is read. `3` is an allowance
/// of three, and `1` is a 503.
#[test]
fn blank_values_from_compose_are_the_defaults() {
    let _env = Env::install();
    let post = |quota: &str, kill_switch: &str| {
        with_var(ENV_SUMMARY_QUOTA, Some(quota), || {
            with_var(ENV_KILL_SWITCH, Some(kill_switch), || {
                let mut h = Harness::new();
                h.policy = IngestPolicy::from_env();
                let served = h
                    .post_json(&batch(vec![element(1)]))
                    .map(|response| response.results)
                    .map_err(|e| refusal(e).status);
                (h.policy.per_ip, served)
            })
        })
    };

    assert_eq!(post("", ""), (12, Ok(vec![Accepted])));
    assert_eq!(post("3", ""), (3, Ok(vec![Accepted])), "control");
    assert_eq!(post("", "1"), (12, Err(503)), "control");
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
    assert_eq!(r.retry_after.as_deref(), Some("61"));
}

/// **Oversized requests spend the allowance.** The quota is charged before
/// the body is read, so two requests refused for their size (413) use up
/// an allowance of two and the valid request after them is a 429. The
/// control is the same valid request on a fresh ingest with the same
/// allowance: accepted.
///
/// A handler that read the body before the quota would answer the two with
/// a 413 it never charged, and then accept the valid request.
#[test]
fn oversized_requests_spend_the_allowance() {
    let _env = Env::install();
    let valid = batch(vec![element(1)]);
    let oversized = vec![b' '; MAX_SUMMARY_BODY_BYTES + 1];
    let mut h = Harness::new();
    h.policy.per_ip = 2;
    assert_eq!(h.verdicts(&valid), [Accepted], "control");

    let mut h = Harness::new();
    h.policy.per_ip = 2;
    for attempt in 1..=2 {
        let r = refusal(h.post(&oversized).unwrap_err());
        assert_eq!(r.status, 413, "oversized request {attempt}: {r:?}");
    }
    let r = refusal(h.post_json(&valid).unwrap_err());
    assert_eq!(r.status, 429, "{r:?}");
    assert_eq!(r.retry_after.as_deref(), Some("61"));
}

/// **An IPv4-mapped peer is counted as its IPv4 address.** On a dual-stack
/// listener an IPv4 peer arrives as `::ffff:a.b.c.d`. With an allowance of
/// one:
///
/// - the mapped form and the plain form of one address share a bucket, so
///   the second of them is a 429;
/// - two different mapped addresses do not, although as IPv6 they lie in
///   one /64 (`::`), which is what the quota folds IPv6 to;
/// - control: two native IPv6 addresses in one /64 still share a bucket.
///
/// The canonical form is taken in `ip_key` (`dev_session/quota.rs`), for
/// every quota that keys on the peer. Without it the second request is
/// served (the two forms are different keys) and the third is a 429 (every
/// mapped address is the same key).
#[test]
fn an_ipv4_mapped_peer_is_counted_as_its_ipv4_address() {
    let _env = Env::install();
    let mut h = Harness::new();
    h.policy.per_ip = 1;
    let mut post = |peer: &str, n: u32| {
        h.peer = peer.parse().unwrap();
        h.post_json(&batch(vec![element(n)]))
            .map(|response| response.results)
            .map_err(|e| refusal(e).status)
    };

    assert_eq!(post("::ffff:203.0.113.77", 1), Ok(vec![Accepted]));
    assert_eq!(post("203.0.113.77", 2), Err(429), "the same host over IPv4");
    assert_eq!(
        post("::ffff:203.0.113.78", 3),
        Ok(vec![Accepted]),
        "another mapped address"
    );
    assert_eq!(post("203.0.113.78", 4), Err(429), "that host over IPv4");

    assert_eq!(post("2001:db8:1:2::1", 5), Ok(vec![Accepted]), "control");
    assert_eq!(post("2001:db8:1:2::2", 6), Err(429), "control: one /64");
}

/// With an allowance of two, the third request from an address inside the
/// minute is a 429 naming the summary quota, another address is unaffected,
/// and the allowance returns when the minute ends: still a 429 at 59
/// seconds, served at 60.
#[test]
fn the_third_request_past_an_allowance_of_two_is_a_429() {
    let _env = Env::install();
    let mut h = Harness::new();
    h.policy.per_ip = 2;
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
    h.now += Duration::from_secs(59);
    assert_eq!(refusal(post(&h, 4).unwrap_err()).status, 429, "59 s in");
    h.now += Duration::from_secs(1);
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

/// `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP` sizes the allowance per
/// minute: 12 when unset or unreadable (an operator typo must neither take the route down
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
