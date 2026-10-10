//! `POST /api/telemetry/upload-chunk`: one gzip(NDJSON) batch of events.
//!
//! The order, each step cheaper than the next: the gate
//! ([`super::upload_gate::admit`]: kill switch, token, rate limits, a slot),
//! then the body (at most [`super::MAX_CHUNK_BYTES`]), its expansion (at
//! most [`super::MAX_CHUNK_DECOMPRESSED_BYTES`]), the rows (at most
//! [`super::MAX_CHUNK_ROWS`]), the string caps, the session budget, naming
//! and replay. A request that passes a budget is refused there with 413 and
//! nothing of it is replayed.

use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::time::{Instant, SystemTime};

use axum::extract::{ConnectInfo, Request};
use axum::Json;
use flate2::read::GzDecoder;

use super::dto::{ChunkResponse, IngestError, TelemetryEvent};
use super::entity_labels::{self, name_chunk};
use super::field_caps::cap_event;
use super::replay::replay_events;
use super::session_budget::{
    admit_budgeted, EVENTS_PER_WINDOW, PRIORITY_EVENTS_PER_WINDOW, WINDOW_SECS,
};
use super::upload_gate::{
    admit, read_body_capped, upload_state, Admitted, Route, UploadLimits, UploadPolicy,
    UploadState, Uploader,
};

/// The handler takes the request whole, body unread: a `Bytes` extractor
/// would buffer the body before the token had been checked.
pub(super) async fn upload_chunk(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
) -> Result<Json<ChunkResponse>, IngestError> {
    chunk_inner(
        upload_state(),
        &UploadPolicy::from_env(),
        peer.ip(),
        request,
        Instant::now(),
    )
    .await
    .map(Json)
}

/// The chunk ingest with its state and policy passed in, for tests. Every
/// refusal is reported through the state's throttled refusal log.
pub(super) async fn chunk_inner(
    state: &UploadState,
    policy: &UploadPolicy,
    peer: IpAddr,
    request: Request,
    now: Instant,
) -> Result<ChunkResponse, IngestError> {
    let mut who = Uploader::anonymous(peer);
    let result = chunk_flow(state, policy, &mut who, request, now).await;
    if let Err(e) = &result {
        state.refusals.report(Route::Chunk, &who, e, now);
    }
    result
}

async fn chunk_flow(
    state: &UploadState,
    policy: &UploadPolicy,
    who: &mut Uploader,
    request: Request,
    now: Instant,
) -> Result<ChunkResponse, IngestError> {
    let (parts, body) = request.into_parts();
    // `_slot` holds the upload slot until the chunk is replayed.
    let Admitted {
        claims,
        permit: _slot,
    } = admit(state, policy, Route::Chunk, &parts.headers, who, now)?;
    let limits = &state.limits;

    let body = read_body_capped(body, limits.chunk_body_bytes).await?;
    let ndjson = decompress_capped(&body, limits.chunk_decompressed_bytes)?;
    let mut events = parse_rows_capped(&ndjson, limits)?;
    drop(ndjson);
    events.iter_mut().for_each(cap_event);

    // The runaway guard first: over the session's budget only priority
    // events are replayed (the rest are counted and reported below, never
    // dropped silently), and only those are named. Then entity IDs are
    // named by asking the cell who held each slot when the row was written
    // (NT-40); the rest of a row's names need no round trip.
    let now_secs = chrono::Utc::now().timestamp();
    let (admitted_rows, totals) = admit_budgeted(&claims, &events, now_secs);
    let labels = name_chunk(
        &claims.sid,
        &claims.sub,
        &events,
        &admitted_rows,
        SystemTime::now(),
        entity_labels::link(),
    )
    .await;
    let counts = replay_events(&claims, events, &labels, &admitted_rows);
    let (accepted, parsed, suppressed) = (counts.accepted, counts.parsed, counts.suppressed);

    if suppressed > 0 {
        tracing::warn!(
            target: "launcher.ingest",
            session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
            install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
            session_kind = claims.session_kind(),
            accepted,
            parsed,
            suppressed,
            session_accepted_total = totals.accepted_total,
            session_suppressed_total = totals.suppressed_total,
            budget_per_window = EVENTS_PER_WINDOW,
            priority_allowance_per_window = PRIORITY_EVENTS_PER_WINDOW,
            window_secs = WINDOW_SECS,
            reason = "session_over_budget",
            "upload-chunk over the session event budget; only priority events replayed, up to their allowance"
        );
    } else {
        tracing::debug!(
            target: "launcher.ingest",
            session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
            install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
            session_kind = claims.session_kind(),
            accepted,
            parsed,
            session_accepted_total = totals.accepted_total,
            session_suppressed_total = totals.suppressed_total,
            body_bytes = body.len(),
            "upload-chunk accepted"
        );
    }

    Ok(ChunkResponse {
        accepted,
        parsed_lines: parsed,
        suppressed,
    })
}

/// Expand a gzip body, refusing it once the output passes `cap`. `take`
/// stops the decoder one byte past the cap, so no more than that is ever
/// allocated.
pub(super) fn decompress_capped(body: &[u8], cap: u64) -> Result<String, IngestError> {
    let mut decoder = GzDecoder::new(body).take(cap + 1);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| IngestError::Gzip(e.to_string()))?;
    if out.len() as u64 > cap {
        return Err(IngestError::OverBudget {
            what: "decompressed bytes",
            limit: cap,
        });
    }
    String::from_utf8(out).map_err(|_| IngestError::Gzip("chunk is not UTF-8".into()))
}

/// Parse a chunk's rows, refusing it at the first bad row or at the first
/// row past `limits.chunk_rows`. The whole chunk parses or none of it
/// replays, so a refused chunk spends none of the session's budget.
pub(super) fn parse_rows_capped(
    ndjson: &str,
    limits: &UploadLimits,
) -> Result<Vec<TelemetryEvent>, IngestError> {
    let mut events = Vec::new();
    for (idx, line) in ndjson.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if events.len() >= limits.chunk_rows {
            return Err(IngestError::OverBudget {
                what: "rows",
                limit: limits.chunk_rows as u64,
            });
        }
        let ev: TelemetryEvent = serde_json::from_str(line).map_err(|e| IngestError::Ndjson {
            line: idx as u64 + 1,
            err: e.to_string(),
        })?;
        events.push(ev);
    }
    Ok(events)
}
