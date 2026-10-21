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
//! `truncated: true` and a throttled `chunk_truncated` warn. For the same
//! reason a row that is not UTF-8 or not an event the server knows is
//! skipped and counted (`bad_rows`, and a throttled `bad_rows` warn)
//! rather than refusing the chunk; only a body that is not gzip at all is
//! refused (400 `bad_gzip`).
//!
//! A launcher chunk (its token is a launcher kind) takes its limits and its
//! row filter from [`super::launcher_rows`] and is charged to its own
//! [`super::launcher_budget`]: it is never named and never spends a player's
//! session budget.

use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::time::{Instant, SystemTime};

use axum::extract::{ConnectInfo, Request};
use axum::http::HeaderMap;
use axum::Json;
use flate2::bufread::GzDecoder;

use super::dto::{ChunkResponse, IngestError, TelemetryEvent};
use super::entity_labels::{self, name_chunk, EntityLabels};
use super::field_caps::cap_event;
use super::launcher_budget::{admit_launcher_chunk, LauncherBudget};
use super::refusal_log::Truncation;
use super::replay::replay_events_from_client;
use super::session_budget::{
    admit_budgeted, launcher_budget, SessionTotals, EVENTS_PER_WINDOW, PRIORITY_EVENTS_PER_WINDOW,
    WINDOW_SECS,
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
    chunk_inner_with(state, policy, peer, request, now, launcher_budget()).await
}

/// [`chunk_inner`] with the launcher budget passed in too, for tests.
pub(super) async fn chunk_inner_with(
    state: &UploadState,
    policy: &UploadPolicy,
    peer: IpAddr,
    request: Request,
    now: Instant,
    budget: &LauncherBudget,
) -> Result<ChunkResponse, IngestError> {
    let mut who = Uploader::anonymous(peer);
    let result = chunk_flow(state, policy, &mut who, request, now, budget).await;
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
    budget: &LauncherBudget,
) -> Result<ChunkResponse, IngestError> {
    let (parts, body) = request.into_parts();
    let Admitted { claims, slot } = admit(state, policy, Route::Chunk, &parts.headers, who, now)?;
    let launcher = claims.is_launcher();
    let client_pid = if launcher {
        None
    } else {
        client_pid_header(&parts.headers)
    };
    let client_sent_ms = client_sent_ms_header(&parts.headers);
    // Taken before the body is read, so the body transfer stays out of the
    // clock sample.
    let recv = SystemTime::now();
    let limits = if launcher {
        state.limits.launcher()
    } else {
        state.limits.clone()
    };
    let row_cap = limits.chunk_rows;

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
    let DecodedChunk {
        mut events,
        truncation,
        mut bad_rows,
    } = decoded?;

    let (admitted_rows, labels, totals) = if launcher {
        // The launcher rows only, charged to the launcher budget (AU-15).
        let admission = admit_launcher_chunk(budget, &claims, &mut events, row_cap, now)
            .map_err(IngestError::Auth)?;
        bad_rows += admission.dropped;
        (
            admission.admitted,
            EntityLabels::none(),
            SessionTotals::default(),
        )
    } else {
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
            recv,
            client_sent_ms,
            entity_labels::link(),
        )
        .await;
        (admitted_rows, labels, totals)
    };
    let counts = replay_events_from_client(&claims, events, &labels, &admitted_rows, client_pid);
    let (accepted, parsed, suppressed) = (counts.accepted, counts.parsed, counts.suppressed);

    if let Some(t) = &truncation {
        state.refusals.report_truncated(Route::Chunk, who, t, now);
    }
    if bad_rows > 0 {
        state.refusals.report_bad_rows(who, bad_rows, now);
    }
    if suppressed > 0 && launcher {
        tracing::warn!(
            target: "launcher.ingest",
            session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
            install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
            session_kind = claims.session_kind(),
            accepted,
            parsed,
            suppressed,
            reason = "launcher_over_budget",
            "launcher upload-chunk over the launcher row budget; the rows past it were not replayed"
        );
    } else if suppressed > 0 {
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
        bad_rows,
    })
}

/// The header the DLL sends its process id in with each chunk (TG-PIPE-02).
/// Older DLLs do not send it, so its absence is normal.
pub(super) const CLIENT_PID_HEADER: &str = "x-cimmeria-client-pid";

/// The uploading process's id from [`CLIENT_PID_HEADER`]. Absent, or not a
/// `u32`, reads as `None`: a bad header must not refuse the chunk (the
/// uploader would re-send it forever), and it never becomes pid 0.
pub(super) fn client_pid_header(headers: &HeaderMap) -> Option<u32> {
    headers
        .get(CLIENT_PID_HEADER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// The header the DLL sends its send time in, in ms on its clock, with each
/// chunk (PC-16).
pub(super) const CLIENT_SENT_MS_HEADER: &str = "x-cimmeria-client-sent-ms";

/// The upload's send stamp from [`CLIENT_SENT_MS_HEADER`]. Absent, not an
/// `i64`, or `<= 0` reads as `None`: a bad stamp must not refuse the chunk,
/// and it only ever sharpens the clock sample, never replaces a good one.
pub(super) fn client_sent_ms_header(headers: &HeaderMap) -> Option<i64> {
    headers
        .get(CLIENT_SENT_MS_HEADER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|&ms| ms > 0)
}

/// A chunk's rows, and where it was cut if it passed a budget.
#[derive(Debug)]
pub(super) struct DecodedChunk {
    pub events: Vec<TelemetryEvent>,
    pub truncation: Option<Truncation>,
    /// Rows skipped because they did not parse.
    pub bad_rows: u64,
}

/// Expand, cut and parse one chunk body, and cap its strings.
pub(super) fn decode_chunk(
    body: &[u8],
    limits: &UploadLimits,
) -> Result<DecodedChunk, IngestError> {
    let inflated = inflate_bounded(body, limits.chunk_decompressed_bytes)?;
    let ParsedRows {
        events,
        past_cap: rows_past_cap,
        bad: bad_rows,
    } = parse_rows_bounded(&inflated.bytes, limits.chunk_rows, limits.chunk_row_bytes);
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
    Ok(DecodedChunk {
        events,
        truncation,
        bad_rows,
    })
}

/// An expanded chunk, cut at its last whole line if it passed the cap.
#[derive(Debug)]
pub(super) struct Inflated {
    /// NDJSON bytes; each row is checked for UTF-8 on its own.
    pub bytes: Vec<u8>,
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
    Ok(Inflated { bytes: out, cut })
}

/// The rows of one chunk.
#[derive(Debug, Default)]
pub(super) struct ParsedRows {
    pub events: Vec<TelemetryEvent>,
    /// Rows past the row cap: counted, never parsed.
    pub past_cap: u64,
    /// Rows that are not UTF-8 or not a [`TelemetryEvent`] the server
    /// knows: counted and skipped.
    pub bad: u64,
}

/// Parse up to `max_rows` good rows; the rest are counted, not parsed. A
/// row that does not parse is counted and skipped rather than refusing the
/// chunk: the uploaders re-send a refused chunk, so one row the server's
/// types reject would otherwise block everything queued behind it.
///
/// A row longer than `max_row_bytes` is counted as bad without being
/// parsed: parsing builds the whole row in memory several times over, and
/// no uploader sends rows anywhere near that long. Each event's strings are
/// capped as soon as it parses, so the parsed chunk never holds more than
/// one uncapped row.
pub(super) fn parse_rows_bounded(
    ndjson: &[u8],
    max_rows: usize,
    max_row_bytes: usize,
) -> ParsedRows {
    let mut rows = ParsedRows::default();
    for line in ndjson.split(|&b| b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        if rows.events.len() >= max_rows {
            rows.past_cap += 1;
            continue;
        }
        if line.len() > max_row_bytes {
            rows.bad += 1;
            continue;
        }
        match std::str::from_utf8(line)
            .ok()
            .and_then(|l| serde_json::from_str::<TelemetryEvent>(l).ok())
        {
            Some(mut ev) => {
                cap_event(&mut ev);
                rows.events.push(ev);
            }
            None => rows.bad += 1,
        }
    }
    rows
}
