//! The launcher-summary ingest handler and its synchronous core.

use std::net::{IpAddr, SocketAddr};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use axum::Json;
use bytes::Bytes;

use crate::routes::dev_session::quota::{ip_key, WindowTable};
use crate::routes::dev_session::{
    env_u32, kill_switch_active, AuthError, QuotaPolicy, SCOPE_LAUNCHER_SUMMARY_WRITE,
};

use super::super::handlers::verify_bearer_scoped;
use super::dedup::Dedup;
use super::dto::{parse_envelope, validate, SummaryError, SummaryResponse, Verdict};
use super::rows::{emit_batch, emit_summary};

/// Requests per peer address per quota window, with or without a valid
/// token. A launcher sends one request per export cycle, a few an hour at
/// most; like the mint quota this is sized for a shared egress address,
/// not for one machine.
const DEFAULT_SUMMARY_PER_IP: u32 = 120;

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
/// serde's error text before the kill switch, the quota or the token had
/// been checked.
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
/// quota (429), bearer token with the summary scope (401), envelope (400),
/// then per-element validation, deduplication, the rows and the response.
///
/// The cheap refusals come first so that a caller without a token, or over
/// its allowance, costs neither an HMAC nor a JSON parse.
///
/// The quota is therefore charged before the token is looked at, and a
/// request with no token or a bad one spends the address's allowance like
/// any other. Anyone behind the same address as real launchers (a NAT, a
/// tunnel) can use it up without a token and turn their summary posts into
/// 429s until the window ends. The refresh route closes the same hole by
/// verifying first and counting failures on a table of their own; doing
/// that here would change the pinned order.
pub(super) fn ingest_inner(
    state: &IngestState,
    policy: &IngestPolicy,
    peer_ip: IpAddr,
    headers: &HeaderMap,
    body: &[u8],
    now: Instant,
) -> Result<SummaryResponse, SummaryError> {
    if kill_switch_active() {
        return Err(AuthError::KillSwitchActive.into());
    }
    state
        .quota
        .check_and_record(
            ip_key(peer_ip),
            policy.per_ip,
            policy.window,
            "summary/ip",
            now,
        )
        .map_err(AuthError::from)?;
    // The claims are deliberately unused: the token proves the caller went
    // through the summary mint and nothing more. Its session and subject
    // must not reach a row.
    verify_bearer_scoped(headers, SCOPE_LAUNCHER_SUMMARY_WRITE)?;

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
