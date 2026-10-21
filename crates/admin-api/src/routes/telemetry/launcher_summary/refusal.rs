//! The log row for a refused summary request: one per reason per minute on
//! the ingest target, with the closed reason and the status. It never names
//! the peer; the quota key stays in memory.

use std::sync::LazyLock;
use std::time::Instant;

use crate::routes::dev_session::Throttle;

/// The row's event. Not the ingest route's `upload_refused`: an anonymous
/// caller drives this row, so the Discord layer keeps it SigNoz-only.
const SUMMARY_REFUSED_EVENT: &str = "launcher_summary_refused";

/// The route in the throttle key and in the row.
const ROUTE: &str = "launcher_summary";

/// The dev-session throttle: one row per (route, reason) per minute, with the
/// rows held back carried in `suppressed`.
static THROTTLE: LazyLock<Throttle> = LazyLock::new(Throttle::default);

/// Log one refused summary request, unless the throttle holds it back. WARN:
/// the player's summary was not recorded.
pub(super) fn note(reason: &'static str, status: u16) {
    note_in(&THROTTLE, reason, status, Instant::now());
}

/// The log reason for a summary refusal code. The ingest route logs
/// `kill_switch`, `rate_limited` and `body_too_large` for the same refusals,
/// so one SigNoz filter counts both routes. The HTTP body keeps the code.
pub(super) fn log_reason(code: &'static str) -> &'static str {
    match code {
        "paused" => "kill_switch",
        "over_quota" => "rate_limited",
        "too_large" => "body_too_large",
        other => other,
    }
}

/// [`note`] against a given table and clock, so the tests start from an
/// empty table.
fn note_in(throttle: &Throttle, reason: &'static str, status: u16, now: Instant) {
    let Some(suppressed) = throttle.admit(ROUTE, reason, now) else {
        return;
    };
    tracing::warn!(
        target: "launcher.ingest", event = SUMMARY_REFUSED_EVENT, route = ROUTE,
        reason, status, suppressed, "launcher summary refused, not recorded: {reason}"
    );
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use axum::body::Body;
    use axum::extract::{ConnectInfo, Request};
    use axum::http::{header::CONTENT_TYPE, HeaderValue};

    use super::super::handlers::ingest;
    use super::super::tests::{capture, Env};
    use super::*;

    const PEER: &str = "203.0.113.78";

    #[test]
    fn a_reason_writes_one_row_a_minute_and_counts_the_rest() {
        let table = Throttle::default();
        let t0 = Instant::now();
        let ((), rows) = capture(|| {
            note_in(&table, "over_quota", 429, t0);
            note_in(&table, "over_quota", 429, t0 + Duration::from_secs(1));
            note_in(&table, "bad_request", 400, t0 + Duration::from_secs(2));
            note_in(&table, "over_quota", 429, t0 + Duration::from_secs(61));
        });
        let reasons: Vec<&str> = rows.iter().map(|r| r.fields["reason"].as_str()).collect();
        assert_eq!(
            reasons,
            ["over_quota", "bad_request", "over_quota"],
            "{rows:#?}"
        );
        assert_eq!(rows[2].fields["suppressed"], "1", "{rows:#?}");
        assert_eq!(rows[0].fields["status"], "429");
        assert!(rows
            .iter()
            .all(|r| r.target == "launcher.ingest" && r.event() == "launcher_summary_refused"));
    }

    /// The summary codes that the ingest route also logs map onto its
    /// reasons, so one SigNoz filter counts kill-switch and rate-limit
    /// refusals on both routes. Reverted to the code, this fails.
    #[test]
    fn the_summary_codes_log_the_ingest_reason_names() {
        assert_eq!(log_reason("paused"), "kill_switch");
        assert_eq!(log_reason("over_quota"), "rate_limited");
        assert_eq!(log_reason("too_large"), "body_too_large");
        assert_eq!(log_reason("bad_request"), "bad_request");
    }

    /// The real handler with a quota of one: the second request is refused
    /// and writes one row naming the reason, not the address. Reverted, no
    /// row is written and this fails.
    #[test]
    fn a_refused_summary_writes_one_row_that_names_no_peer() {
        let _env = Env::install();
        std::env::set_var("CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP", "1");
        let peer = SocketAddr::new(PEER.parse().unwrap(), 40000);
        let send = || {
            let mut request = Request::new(Body::from("{}"));
            request
                .headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();
            rt.block_on(ingest(ConnectInfo(peer), request))
        };
        let _ = send();
        let (result, rows) = capture(send);
        std::env::remove_var("CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP");
        assert_eq!(result.err().map(|e| e.status()), Some(429));
        assert_eq!(rows.len(), 1, "{rows:#?}");
        assert_eq!(rows[0].fields["reason"], "rate_limited");
        assert_eq!(rows[0].fields["status"], "429");
        assert!(!format!("{rows:?}").contains(PEER), "{rows:#?}");
    }
}
