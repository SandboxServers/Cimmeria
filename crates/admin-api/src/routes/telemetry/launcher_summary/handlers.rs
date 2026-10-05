//! The launcher-summary ingest handler and its injectable core.

use std::future::poll_fn;
use std::net::{IpAddr, SocketAddr};
use std::pin::pin;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use axum::body::{Body, HttpBody};
use axum::extract::{ConnectInfo, Request};
use axum::http::header::CONTENT_TYPE;
use axum::Json;

use crate::routes::dev_session::quota::{ip_key, WindowTable};
use crate::routes::dev_session::{env_u32, kill_switch_active, QuotaPolicy};

use super::dedup::Dedup;
use super::dto::{is_json_content_type, validate, SummaryError, SummaryResponse, Verdict};
use super::envelope::parse_envelope;
use super::rows::{emit_batch, emit_summary};
use super::MAX_SUMMARY_BODY_BYTES;

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

/// `POST /api/telemetry/launcher-summary`. The handler takes the request
/// whole, body unread. A `Bytes` or `Json<T>` extractor would buffer the
/// body, and answer an oversized or malformed one itself, before the kill
/// switch or the quota had been checked.
pub(super) async fn ingest(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
) -> Result<Json<SummaryResponse>, SummaryError> {
    ingest_inner(
        state(),
        &IngestPolicy::from_env(),
        peer.ip(),
        request,
        Instant::now(),
    )
    .await
    .map(Json)
}

/// The quota key for a peer. An IPv4 peer that reached a dual-stack
/// listener arrives as the IPv4-mapped IPv6 address `::ffff:a.b.c.d`.
/// [`ip_key`] folds IPv6 to its /64, and every mapped address has the same
/// one (`::`), so uncanonicalised they would all share a single bucket,
/// and none of them would share with the same host seen over plain IPv4.
fn peer_key(peer_ip: IpAddr) -> u64 {
    ip_key(peer_ip.to_canonical())
}

/// Read the body, refusing it once it passes [`MAX_SUMMARY_BODY_BYTES`]:
/// the frame that would cross the cap is never copied in. There is no
/// shortcut on `Content-Length`, so a length-prefixed body and a chunked
/// one take the same path.
async fn read_body(body: Body) -> Result<Vec<u8>, SummaryError> {
    let mut body = pin!(body);
    let mut bytes = Vec::new();
    while let Some(frame) = poll_fn(|cx| body.as_mut().poll_frame(cx)).await {
        // A body that fails part-way (broken chunking, a peer that left)
        // is not the payload either.
        let frame = frame.map_err(|_| SummaryError::BadRequest("Body could not be read"))?;
        // Trailers are not body bytes.
        let Ok(data) = frame.into_data() else {
            continue;
        };
        if bytes.len() + data.len() > MAX_SUMMARY_BODY_BYTES {
            return Err(SummaryError::TooLarge);
        }
        bytes.extend_from_slice(&data);
    }
    Ok(bytes)
}

/// The ingest, in the order the tests pin: kill switch (503), per-address
/// quota (429), `Content-Type` (415), query string (400), body size (413),
/// envelope (400), then per-element validation, deduplication, the rows
/// and the response.
///
/// The route is anonymous. There is no token, and nothing here reads the
/// `Authorization` header: the headers are consulted for `Content-Type`
/// and for nothing else, so a caller who sends a token, a real one
/// included, is treated exactly like one who sends none.
///
/// The quota is charged before anything the caller sent is looked at and
/// before a byte of the body is read. Every request shape therefore spends
/// the address's allowance like an accepted one (an oversized body, a
/// wrong media type, a query string, a malformed envelope), and a caller
/// over the allowance, or any caller while the kill switch is on, is
/// answered with nothing buffered and nothing parsed. The other side of
/// that: anyone behind the same address as real launchers (a NAT, a
/// tunnel) can use the allowance up and turn their summary posts into 429s
/// until the window ends.
///
/// A query string is refused because of where it would be logged, not
/// because of anything it could do here: no code reads it, but the
/// listener's request span may record the whole URI (the admin listener's
/// does), and every row this function writes sits inside that span.
/// Refusing the request before the first row keeps caller-chosen URI text
/// from ever standing beside one. The media type and the query are checked
/// before the body is read, so a request refused for either costs no
/// buffering.
pub(super) async fn ingest_inner(
    state: &IngestState,
    policy: &IngestPolicy,
    peer_ip: IpAddr,
    request: Request,
    now: Instant,
) -> Result<SummaryResponse, SummaryError> {
    if kill_switch_active() {
        return Err(SummaryError::Paused);
    }
    state.quota.check_and_record(
        peer_key(peer_ip),
        policy.per_ip,
        policy.window,
        "summary/ip",
        now,
    )?;
    let (parts, body) = request.into_parts();
    // Exactly one `Content-Type`: with two, which one a proxy or a later
    // reader would honour is not ours to guess.
    let mut content_types = parts.headers.get_all(CONTENT_TYPE).iter();
    match (content_types.next(), content_types.next()) {
        (Some(value), None) if is_json_content_type(value) => {}
        _ => return Err(SummaryError::UnsupportedMediaType),
    }
    // `?` with nothing after it is a query string too.
    if parts.uri.query().is_some() {
        return Err(SummaryError::BadRequest(SummaryError::QUERY));
    }
    let body = read_body(body).await?;

    let envelope = parse_envelope(&body)?;
    let summaries: Vec<_> = envelope
        .elements
        .into_iter()
        .map(|element| element.and_then(validate))
        .collect();
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
