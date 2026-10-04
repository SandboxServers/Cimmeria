//! Replaying uploaded events through `tracing`, so every sink (SigNoz, the
//! on-disk logs, the admin WebSocket) receives them.
//!
//! # Where the rows land
//!
//! The client-side replays (`client.native`, `launcher.client_log`,
//! `launcher.debug_log`, `launcher.session_meta`) are routed to the
//! `cimmeria-client` SigNoz service by the server's log filters
//! (`cimmeria_server::otel::CLIENT_TARGETS`). The server's own account of
//! an upload (`launcher.ingest`, `launcher.bundle`) stays in
//! `cimmeria-server`.
//!
//! # Attributes every replayed row carries
//!
//! The OTLP bridge turns each event's own fields into log attributes (it
//! does not flatten span fields), so everything a SigNoz query should
//! filter on is a field on the event itself:
//!
//! | Field | Source |
//! |---|---|
//! | `session_id` | the token's `sid` claim |
//! | `install_id` | the token's `sub` claim |
//! | `cimmeria.session_kind` | `"lab"` or `"player"`, from the signed `kind` claim |
//! | `lab` | `true` for a lab session |
//! | `ts_ms`, `seq` | the uploader's clock and sequence number |
//!
//! A `client.native` row adds the DLL's event name (`client_target`, also
//! the log body), its level string (`client_level`), the DLL's IDs lifted
//! out of its `fields` bag with their names resolved here (see
//! [`super::replay_native`]), and the whole bag as JSON in `fields`.
//!
//! # Naming entity IDs
//!
//! [`replay_ndjson`] names content IDs, method indexes, message ids and
//! addresses, but not entity IDs: those need the cell
//! ([`super::entity_labels`]). The upload handler and
//! [`replay_ndjson_named`] ask it.

use std::time::SystemTime;

use tokio::sync::mpsc;

use cimmeria_services::cell::messages::BaseToCellMsg;

use crate::routes::dev_session::TokenClaims;

use super::dto::TelemetryEvent;
use super::entity_labels::{name_chunk, EntityLabels};
use super::replay_native::{replay_client_native_named, ReplayNames};

/// One NDJSON line that did not parse as a [`TelemetryEvent`]. The whole
/// chunk is refused at the first one, as before this module existed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayError {
    /// 1-based line number in the decompressed chunk.
    pub line: u64,
    pub err: String,
}

/// Counts from one [`replay_ndjson`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReplayCounts {
    /// Non-blank lines seen.
    pub parsed: u64,
    /// Lines replayed. Equal to `parsed` unless the session budget
    /// suppressed some (see `session_budget`).
    pub accepted: u64,
    /// Lines parsed but not replayed because the session was over its
    /// budget. Always 0 from [`replay_ndjson`].
    pub suppressed: u64,
}

/// Replay every event in a decompressed upload chunk (NDJSON, one
/// `TelemetryEvent` per line) under `claims`. Blank lines are skipped.
/// Entity IDs are not named (see the module docs).
///
/// Public so the server's ingest round-trip test can drive the exact path
/// `/api/telemetry/upload-chunk` takes, past the HTTP and gzip layers.
pub fn replay_ndjson(claims: &TokenClaims, ndjson: &str) -> Result<ReplayCounts, ReplayError> {
    replay_ndjson_gated(claims, ndjson, |_| true)
}

/// [`replay_ndjson`] with entity IDs named: the chunk is placed on the
/// server clock as received at `recv` and the cell is asked through `cell`,
/// as the upload handler does (without its session budget).
///
/// Public so the server's tests can drive the naming against a real
/// `SpaceManager`.
pub async fn replay_ndjson_named(
    claims: &TokenClaims,
    ndjson: &str,
    recv: SystemTime,
    cell: Option<&mpsc::Sender<BaseToCellMsg>>,
) -> Result<ReplayCounts, ReplayError> {
    let events = parse_ndjson(ndjson)?;
    let labels = name_chunk(&claims.sid, &events, recv, cell).await;
    Ok(replay_events_gated(claims, events, &labels, |_| true))
}

/// [`replay_ndjson`] with a gate: an event for which `admit` returns
/// `false` is parsed and counted as suppressed but not replayed. A bad
/// line refuses the whole chunk. The whole chunk is parsed before `admit`
/// sees any event, so a refused chunk replays nothing and spends none of
/// the session's budget: its retry is judged afresh.
pub(super) fn replay_ndjson_gated(
    claims: &TokenClaims,
    ndjson: &str,
    admit: impl FnMut(&TelemetryEvent) -> bool,
) -> Result<ReplayCounts, ReplayError> {
    let events = parse_ndjson(ndjson)?;
    Ok(replay_events_gated(
        claims,
        events,
        &EntityLabels::none(),
        admit,
    ))
}

/// Parse a whole chunk, refusing it at the first bad line.
pub(super) fn parse_ndjson(ndjson: &str) -> Result<Vec<TelemetryEvent>, ReplayError> {
    let mut events = Vec::new();
    for (idx, line) in ndjson.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let ev: TelemetryEvent = serde_json::from_str(line).map_err(|e| ReplayError {
            line: idx as u64 + 1,
            err: e.to_string(),
        })?;
        events.push(ev);
    }
    Ok(events)
}

/// Replay a parsed chunk through `admit`, naming each row's entity IDs
/// from `labels` (built for these `events`, in this order).
pub(super) fn replay_events_gated(
    claims: &TokenClaims,
    events: Vec<TelemetryEvent>,
    labels: &EntityLabels,
    mut admit: impl FnMut(&TelemetryEvent) -> bool,
) -> ReplayCounts {
    let mut counts = ReplayCounts {
        parsed: events.len() as u64,
        ..ReplayCounts::default()
    };
    let book = cimmeria_names::book();
    for (row, ev) in events.into_iter().enumerate() {
        if admit(&ev) {
            let names = ReplayNames {
                book: &book,
                entity_label: &|id| labels.label(row, id),
            };
            replay_event(claims, ev, &names);
            counts.accepted += 1;
        } else {
            counts.suppressed += 1;
        }
    }
    counts
}

pub(super) fn replay_event(claims: &TokenClaims, ev: TelemetryEvent, names: &ReplayNames<'_>) {
    // Every event carries the session's id, install and kind so SigNoz
    // queries can slice by session or by player without joining across
    // rows. Field naming matches the dev_session mint event so a session's
    // mint and uploads correlate naturally.
    let kind = claims.session_kind();
    let lab = claims.is_lab();
    match ev {
        TelemetryEvent::ClientLog(e) => {
            tracing::info!(
                target: "launcher.client_log",
                session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
                install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
                cimmeria.session_kind = kind,
                lab,
                ts_ms = e.ts_ms,
                seq = e.seq,
                source_file = %e.source_file,
                level = %e.level,
                category = %e.category,
                packet_no = ?e.packet_no,
                message = %e.message,
            );
        }
        TelemetryEvent::DebugLog(e) => {
            tracing::info!(
                target: "launcher.debug_log",
                session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
                install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
                cimmeria.session_kind = kind,
                lab,
                ts_ms = e.ts_ms,
                seq = e.seq,
                source_file = %e.source_file,
                level = %e.level,
                message = %e.message,
            );
        }
        TelemetryEvent::KeyDump(e) => {
            // KeyDump entries are encryption key material observed by
            // the client. Useful for offline pcap decryption — but we
            // intentionally do NOT log the key body at info; debug
            // only, so the default sinks don't carry it to disk, and the
            // target is `off` in every OTLP filter.
            tracing::debug!(
                target: "launcher.key_dump",
                session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
                install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
                ts_ms = e.ts_ms,
                seq = e.seq,
                source_file = %e.source_file,
                key_b64 = %e.key_b64,
            );
        }
        TelemetryEvent::SessionMeta(e) => {
            tracing::info!(
                target: "launcher.session_meta",
                session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
                install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
                cimmeria.session_kind = kind,
                lab,
                ts_ms = e.ts_ms,
                seq = e.seq,
                kind = %e.kind,
                fields = %serde_json::Value::Object(e.fields),
            );
        }
        TelemetryEvent::ClientNative(e) => {
            replay_client_native_named(claims, e, names.book, names.entity_label);
        }
    }
}
