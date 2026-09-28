//! SGWPlayer base-method diagnostic / telemetry-sink handlers.
//!
//! Extracted from `dispatch.rs` — the no-op diagnostic arm of
//! `dispatch_sgw_player_base_method` for `perfStats` (client perf telemetry
//! sink), a DEBUG-only sink deliberately kept out of the unhandled-WARN
//! catch-all. (`elementDataRequest`, `0xD5`, is served since #840: the
//! connect loop routes it to `cooked_data::handle_element_data_request`.)

use std::net::SocketAddr;

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
