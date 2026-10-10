//! `POST /api/telemetry/upload-chunk`: one gzip(NDJSON) batch of events.
//!
//! The order, each step cheaper than the next: the gate
//! ([`super::upload_gate::admit`]: kill switch, token, rate limits, a slot),
//! then the body (at most [`super::MAX_CHUNK_BYTES`], refused with 413 past
//! it, and within the body deadline), then on a blocking worker the
//! expansion (at most [`super::MAX_CHUNK_DECOMPRESSED_BYTES`]) and the parse
//! (at most [`super::MAX_CHUNK_ROWS`] rows), then the string caps, the
//! session budget, naming and replay.
//!
//! The two expansion budgets truncate rather than refuse. The deployed
//! launcher posts its whole on-disk queue as one chunk and re-queues it on
//! any error, so a backlog over a budget would otherwise be refused on
//! every retry, forever. Instead the chunk is cut at the last whole line
//! within the budget, the rows before the cut are replayed, the rest are
//! counted (or estimated) and dropped, and the answer is a 200 with
//! `truncated: true` and a throttled `chunk_truncated` warn.

use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::time::{Instant, SystemTime};

use axum::extract::{ConnectInfo, Request};
use axum::Json;
use flate2::bufread::GzDecoder;

use super::dto::{ChunkResponse, IngestError, TelemetryEvent};
use super::entity_labels::{self, name_chunk};
use super::field_caps::cap_event;
use super::refusal_log::Truncation;
use super::replay::replay_events;
use super::session_budget::{
    admit_budgeted, EVENTS_PER_WINDOW, PRIORITY_EVENTS_PER_WINDOW, WINDOW_SECS,
};
use super::upload_gate::{
    admit, by_deadline, read_body_capped, upload_state, Admitted, Route, UploadLimits,
    UploadPolicy, UploadState, Uploader,
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
    let Admitted { claims, slot } = admit(state, policy, Route::Chunk, &parts.headers, who, now)?;
    let limits = state.limits.clone();

    let deadline = tokio::time::Instant::now() + limits.body_timeout;
    let body = by_deadline(deadline, read_body_capped(body, limits.chunk_body_bytes)).await?;
    let body_bytes = body.len();

    // Expansion and parsing are CPU-bound: a blocking worker, which takes
    // the slot along so it stays held while the worker runs, and hands it
    // back for the replay.
    let (slot, decoded) = tokio::task::spawn_blocking(move || {
        let decoded = decode_chunk(&body, &limits);
        (slot, decoded)
    })
    .await
    .map_err(|e| IngestError::Gzip(format!("chunk decode join failed: {e}")))?;
    let _slot = slot;
    let DecodedChunk { events, truncation } = decoded?;

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

    if let Some(t) = &truncation {
        state.refusals.report_truncated(Route::Chunk, who, t, now);
    }
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
            body_bytes,
            truncated = truncation.is_some(),
            "upload-chunk accepted"
        );
    }

    Ok(ChunkResponse {
        accepted,
        parsed_lines: parsed,
        suppressed,
        truncated: truncation.is_some(),
    })
}

/// A chunk's rows, and where it was cut if it passed a budget.
#[derive(Debug)]
pub(super) struct DecodedChunk {
    pub events: Vec<TelemetryEvent>,
    pub truncation: Option<Truncation>,
}

/// Expand, cut and parse one chunk body, and cap its strings.
pub(super) fn decode_chunk(
    body: &[u8],
    limits: &UploadLimits,
) -> Result<DecodedChunk, IngestError> {
    let inflated = inflate_bounded(body, limits.chunk_decompressed_bytes)?;
    let (mut events, rows_past_cap) = parse_rows_bounded(&inflated.text, limits.chunk_rows)?;
    events.iter_mut().for_each(cap_event);
    let kept = events.len() as u64;
    let truncation = if let Some(ratio_left) = inflated.cut {
        // Rows past the cut were never expanded: estimate them from the
        // rows per compressed byte of what was.
        let seen = kept + rows_past_cap;
        Some(Truncation {
            budget: "decompressed bytes",
            limit: limits.chunk_decompressed_bytes,
            kept,
            dropped_estimate: rows_past_cap + (seen as f64 * ratio_left) as u64,
        })
    } else if rows_past_cap > 0 {
        Some(Truncation {
            budget: "rows",
            limit: limits.chunk_rows as u64,
            kept,
            dropped_estimate: rows_past_cap,
        })
    } else {
        None
    };
    Ok(DecodedChunk { events, truncation })
}

/// An expanded chunk, cut at its last whole line if it passed the cap.
#[derive(Debug)]
pub(super) struct Inflated {
    pub text: String,
    /// `Some(r)` when the output was cut at the cap: `r` is the compressed
    /// input left unread per byte read, for estimating what was dropped.
    pub cut: Option<f64>,
}

/// Expand a gzip body to at most `cap` bytes. `take` stops the decoder one
/// byte past the cap, so no more than that is ever allocated; output past
/// the cap is dropped, and so is the partial line before it.
pub(super) fn inflate_bounded(body: &[u8], cap: u64) -> Result<Inflated, IngestError> {
    let mut decoder = GzDecoder::new(body).take(cap + 1);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| IngestError::Gzip(e.to_string()))?;
    let cut = if out.len() as u64 > cap {
        let unread = decoder.get_ref().get_ref().len();
        let read = body.len().saturating_sub(unread).max(1);
        out.truncate(cap as usize);
        let whole = out.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
        out.truncate(whole);
        Some(unread as f64 / read as f64)
    } else {
        None
    };
    let text =
        String::from_utf8(out).map_err(|_| IngestError::Gzip("chunk is not UTF-8".into()))?;
    Ok(Inflated { text, cut })
}

/// Parse up to `max_rows` rows; the rest are counted, not parsed. A bad
/// row among the parsed ones refuses the whole chunk, so a refused chunk
/// spends none of the session's budget.
pub(super) fn parse_rows_bounded(
    ndjson: &str,
    max_rows: usize,
) -> Result<(Vec<TelemetryEvent>, u64), IngestError> {
    let mut events = Vec::new();
    let mut past_cap = 0u64;
    for (idx, line) in ndjson.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if events.len() >= max_rows {
            past_cap += 1;
            continue;
        }
        let ev: TelemetryEvent = serde_json::from_str(line).map_err(|e| IngestError::Ndjson {
            line: idx as u64 + 1,
            err: e.to_string(),
        })?;
        events.push(ev);
    }
    Ok((events, past_cap))
}
