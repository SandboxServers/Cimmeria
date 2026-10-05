//! The launcher-summary ingest handler and its synchronous core.

use std::net::{IpAddr, SocketAddr};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use axum::extract::ConnectInfo;
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::Json;
use bytes::Bytes;

use crate::routes::dev_session::quota::{ip_key, WindowTable};
use crate::routes::dev_session::{env_u32, kill_switch_active, QuotaPolicy};

use super::dedup::Dedup;
use super::dto::{
    is_json_content_type, parse_envelope, validate, SummaryError, SummaryResponse, Verdict,
};
use super::rows::{emit_batch, emit_summary};

/// Requests per peer address per quota window. The route is anonymous, so
/// this is the only thing between it and anyone who can reach the port,
/// and it is deliberately low: a launcher sends one request per export
/// cycle, a few an hour at most. Every request counts, the refused ones
/// too.
///
/// The allowance belongs to the address, not to a machine: everyone behind
/// one NAT, reverse proxy or tunnel shares it, and an operator behind such
/// an address raises `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP`.
const DEFAULT_SUMMARY_PER_IP: u32 = 12;

/// Operator-tunable limits, read per request like the mint's
/// [`QuotaPolicy`]: change the env and restart.
pub(super) struct IngestPolicy {
    /// Shared with the mint quotas (`CIMMERIA_TELEMETRY_QUOTA_WINDOW_SECS`).
    pub window: Duration,
    /// `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP`; 0 disables the quota.
    pub per_ip: u32,
}

impl IngestPolicy {
    pub(super) fn from_env() -> Self {
        Self {
            window: QuotaPolicy::from_env().window,
            per_ip: env_u32(
                "CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP",
                DEFAULT_SUMMARY_PER_IP,
            ),
        }
    }
}

/// What the ingest remembers between requests. It lives for the process (a
/// per-request copy would count nothing and forget every id) and is passed
/// in, so each test gets a fresh one.
pub(super) struct IngestState {
    pub quota: WindowTable,
    pub dedup: Dedup,
}

impl IngestState {
    pub(super) fn new() -> Self {
        Self {
            quota: WindowTable::new(),
            dedup: Dedup::new(),
        }
    }
}

fn state() -> &'static IngestState {
    static STATE: OnceLock<IngestState> = OnceLock::new();
    STATE.get_or_init(IngestState::new)
}

/// `POST /api/telemetry/launcher-summary`. The body is taken as bytes, not
/// through `Json<T>`: the extractor would answer a malformed body with
/// serde's error text, and a wrong `Content-Type` with its own 415, before
/// the kill switch or the quota had been checked.
pub(super) async fn ingest(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<SummaryResponse>, SummaryError> {
    ingest_inner(
        state(),
        &IngestPolicy::from_env(),
        peer.ip(),
        &headers,
        &body,
        Instant::now(),
    )
    .map(Json)
}

/// The ingest, in the order the tests pin: kill switch (503), per-address
/// quota (429), `Content-Type` (415), envelope (400), then per-element
/// validation, deduplication, the rows and the response.
///
/// The route is anonymous. There is no token, and nothing here reads the
/// `Authorization` header: `headers` is consulted for `Content-Type` and
/// for nothing else, so a caller who sends a token, a real one included,
/// is treated exactly like one who sends none.
///
/// The quota is charged before anything the caller sent is looked at, so a
/// request refused for its media type or its body spends the address's
/// allowance like an accepted one, and a caller over the allowance costs
/// no JSON parse. The other side of that: anyone behind the same address
/// as real launchers (a NAT, a tunnel) can use the allowance up and turn
/// their summary posts into 429s until the window ends.
pub(super) fn ingest_inner(
    state: &IngestState,
    policy: &IngestPolicy,
    peer_ip: IpAddr,
    headers: &HeaderMap,
    body: &[u8],
    now: Instant,
) -> Result<SummaryResponse, SummaryError> {
    if kill_switch_active() {
        return Err(SummaryError::Paused);
    }
    state.quota.check_and_record(
        ip_key(peer_ip),
        policy.per_ip,
        policy.window,
        "summary/ip",
        now,
    )?;
    // Exactly one `Content-Type`: with two, which one a proxy or a later
    // reader would honour is not ours to guess.
    let mut content_types = headers.get_all(CONTENT_TYPE).iter();
    match (content_types.next(), content_types.next()) {
        (Some(value), None) if is_json_content_type(value) => {}
        _ => return Err(SummaryError::UnsupportedMediaType),
    }

    let envelope = parse_envelope(body)?;
    let summaries: Vec<_> = envelope.elements.into_iter().map(validate).collect();
    let ids: Vec<_> = summaries
        .iter()
        .map(|summary| summary.as_ref().map(|s| s.event_id.as_u128()))
        .collect();
    // Every verdict is decided before the first row is written, and the
    // dedup lock is released by then: emitting runs the whole log pipeline.
    let results = state.dedup.judge(&ids);

    let (mut accepted, mut duplicate, mut rejected) = (0u32, 0u32, 0u32);
    for (summary, verdict) in summaries.iter().zip(&results) {
        match (verdict, summary) {
            (Verdict::Accepted, Some(summary)) => {
                accepted += 1;
                emit_summary(summary);
            }
            (Verdict::Duplicate, _) => duplicate += 1,
            _ => rejected += 1,
        }
    }
    emit_batch(accepted, duplicate, rejected, envelope.client_dropped);
    Ok(SummaryResponse { results })
}
