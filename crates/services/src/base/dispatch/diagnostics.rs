//! SGWPlayer base-method diagnostic / telemetry-sink handlers.
//!
//! Extracted from `dispatch.rs` — the no-op diagnostic arms of
//! `dispatch_sgw_player_base_method`: `elementDataRequest` (in-world cache
//! miss) and `perfStats` (client perf telemetry sink). Both are DEBUG-only
//! sinks deliberately kept out of the unhandled-WARN catch-all. Pure code
//! movement; each function carries the exact arm body it replaced.

use std::net::SocketAddr;

/// `SGWPlayer.elementDataRequest(UINT16 categoryId, UINT32 key)` — in-world
/// cache-miss query. Diagnostic only (the catalog + per-key push completed in
/// `cooked_data.rs` before world entry), so this is a documented no-op.
pub(super) fn handle_element_data_request(payload: &[u8], addr: SocketAddr) {
    // Wire: UINT16 categoryId, UINT32 key. Logged as DEBUG only —
    // the in-world client uses this as a cache miss query, but
    // the catalog + per-key push completed in `cooked_data.rs`
    // before world entry, so the runtime path here is purely
    // diagnostic. Promoting back to WARN would flood operator
    // alerts on every cache miss the client decides to re-ask
    // for (1× per session observed in 2026-06-04 sessions).
    if payload.len() >= 6 {
        let category_id = u16::from_le_bytes([payload[0], payload[1]]);
        let key = u32::from_le_bytes([payload[2], payload[3], payload[4], payload[5]]);
        tracing::debug!(
            %addr,
            category_id,
            key,
            "SGWPlayer.elementDataRequest — in-world cache miss (no-op)"
        );
    } else {
        tracing::debug!(
            %addr,
            payload_len = payload.len(),
            "SGWPlayer.elementDataRequest — short payload (no-op)"
        );
    }
}

/// `SGWPlayer.perfStats(12 × FLOAT)` — client perf telemetry pushed every
/// ~15 s. Sink-only on the server; the DEBUG line confirms the client is
/// ticking without flooding WARN.
pub(super) fn handle_perf_stats(payload: &[u8], addr: SocketAddr) {
    // Wire: 12 × FLOAT (48 bytes) — client perf telemetry pushed
    // every ~15 s. No actionable response; the DEBUG line is
    // enough to confirm the client is alive without flooding
    // WARN. If/when we wire this to SigNoz metrics, parse the
    // 12 floats here and emit a `perf_stats` metric.
    //
    // A non-48-byte payload is a wire-shape drift signal: the
    // client either changed the metric set or is sending a
    // corrupted packet. Logged at DEBUG with both the actual
    // and expected length so an ops query can grep for it
    // without us promoting the everyday case to WARN.
    const EXPECTED_PERF_STATS_LEN: usize = 48;
    if payload.len() != EXPECTED_PERF_STATS_LEN {
        tracing::debug!(
            %addr,
            payload_len = payload.len(),
            expected_len = EXPECTED_PERF_STATS_LEN,
            "SGWPlayer.perfStats — unexpected payload length (wire shape drift?)"
        );
    } else {
        tracing::debug!(
            %addr,
            payload_len = payload.len(),
            "SGWPlayer.perfStats — telemetry sink (no-op)"
        );
    }
}
