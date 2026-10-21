//! The log row for a refused summary request: one per reason per minute on
//! the ingest target, with the closed reason and the status. It never names
//! the peer; the quota key stays in memory.

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::routes::telemetry::refusal_log::REFUSED_EVENT;

/// A reason writes at most one row per this window.
const WINDOW: Duration = Duration::from_secs(60);

/// Per reason: the last row's time and the refusals held back since. The
/// reasons are a closed set, so this stays a handful of entries.
static THROTTLE: Mutex<Vec<(&'static str, Instant, u64)>> = Mutex::new(Vec::new());

/// Log one refused summary request, unless the throttle holds it back. WARN:
/// the player's summary was not recorded.
pub(super) fn note(reason: &'static str, status: u16) {
    note_at(&THROTTLE, reason, status, Instant::now());
}

fn note_at(
    table: &Mutex<Vec<(&'static str, Instant, u64)>>,
    reason: &'static str,
    status: u16,
    now: Instant,
) {
    // Released before the row is written: emitting runs the log pipeline.
    let suppressed = {
        let mut table = table.lock().unwrap_or_else(PoisonError::into_inner);
        match table.iter_mut().find(|slot| slot.0 == reason) {
            None => {
                table.push((reason, now, 0));
                0
            }
            Some(slot) if now.duration_since(slot.1) < WINDOW => {
                slot.2 += 1;
                return;
            }
            Some(slot) => {
                slot.1 = now;
                std::mem::take(&mut slot.2)
            }
        }
    };
    tracing::warn!(
        target: "launcher.ingest", event = REFUSED_EVENT, route = "launcher_summary",
        reason, status, suppressed, "launcher summary refused, not recorded: {reason}"
    );
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::body::Body;
    use axum::extract::{ConnectInfo, Request};
    use axum::http::{header::CONTENT_TYPE, HeaderValue};

    use super::super::handlers::ingest;
    use super::super::tests::{capture, Env};
    use super::*;

    const PEER: &str = "203.0.113.78";

    #[test]
    fn a_reason_writes_one_row_a_minute_and_counts_the_rest() {
        let table = Mutex::new(Vec::new());
        let t0 = Instant::now();
        let ((), rows) = capture(|| {
            note_at(&table, "over_quota", 429, t0);
            note_at(&table, "over_quota", 429, t0 + Duration::from_secs(1));
            note_at(&table, "bad_request", 400, t0 + Duration::from_secs(2));
            note_at(&table, "over_quota", 429, t0 + Duration::from_secs(61));
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
            .all(|r| r.target == "launcher.ingest" && r.event() == "upload_refused"));
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
        assert_eq!(rows[0].fields["reason"], "over_quota");
        assert_eq!(rows[0].fields["status"], "429");
        assert!(!format!("{rows:?}").contains(PEER), "{rows:#?}");
    }
}
