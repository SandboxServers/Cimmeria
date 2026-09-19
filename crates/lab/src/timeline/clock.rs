//! Client↔server clock-offset estimation for the merged timeline
//! (ADR §5).
//!
//! Client events carry a client-generated `ts_ms`; packet-tap rows carry
//! the server's clock. To interleave them on one axis we need the offset
//! between the two clocks. The estimator is NTP-shaped: each sample is a
//! round trip whose midpoint on the *local* clock is paired with a
//! *server* reading, and the offset is the difference.
//!
//! On this branch there is no dedicated server `lab` ping tool yet, so
//! the live caller builds its sample from a packet-tap read: the newest
//! tap-row server timestamp stands in for "server now". That is coarse
//! (it lags by the row's own age plus read latency), which is why the
//! estimator keeps the RTT alongside the offset and picks the sample with
//! the *smallest* RTT — the least-contaminated reading. When a real
//! server ping lands, feed its samples through the same function; nothing
//! downstream changes.

use serde::Serialize;

/// A single round-trip clock reading.
///
/// `local_send_ms` / `local_recv_ms` are the dev-box wall clock (ms since
/// epoch) bracketing the request; `server_ms` is the server's clock as
/// read from the reply (or, today, from the newest tap row).
#[derive(Debug, Clone, Copy)]
pub struct PingSample {
    pub local_send_ms: i64,
    pub server_ms: i64,
    pub local_recv_ms: i64,
}

impl PingSample {
    /// Round-trip time in ms. Clamped at 0 in case the two local reads
    /// come back non-monotonic (coarse timers, NTP step mid-call).
    pub fn rtt_ms(&self) -> i64 {
        (self.local_recv_ms - self.local_send_ms).max(0)
    }

    /// Offset to add to a *local* (client) timestamp to land it on the
    /// *server* clock: `server ≈ local + offset`.
    ///
    /// NTP form: assume the request and reply legs are symmetric, so the
    /// server reading corresponds to the local-clock midpoint.
    pub fn offset_ms(&self) -> i64 {
        let local_mid = self.local_send_ms + (self.local_recv_ms - self.local_send_ms) / 2;
        self.server_ms - local_mid
    }
}

/// The estimate handed to the merge step and reported to the caller.
#[derive(Debug, Clone, Serialize)]
pub struct ClockOffset {
    /// Add to a client timestamp to reach the server clock.
    pub offset_ms: i64,
    /// RTT of the sample this offset came from (lower ⇒ more trustworthy).
    pub rtt_ms: i64,
    /// How many samples were considered.
    pub sample_count: usize,
    /// How the estimate was produced, for the finding record.
    pub method: String,
}

impl ClockOffset {
    /// The identity offset — used when nothing could be measured, so the
    /// timeline still renders (client and server rows just sit on their
    /// own raw clocks, flagged by `method`).
    pub fn unestimated() -> Self {
        Self {
            offset_ms: 0,
            rtt_ms: 0,
            sample_count: 0,
            method: "unestimated".to_string(),
        }
    }
}

/// Estimate the offset from a set of round-trip samples, NTP best-sample
/// style: pick the sample with the lowest RTT and take its offset.
///
/// Returns `None` when there are no samples (the caller then falls back
/// to [`ClockOffset::unestimated`]).
pub fn estimate_offset(samples: &[PingSample], method: &str) -> Option<ClockOffset> {
    let best = samples.iter().min_by_key(|s| s.rtt_ms())?;
    Some(ClockOffset {
        offset_ms: best.offset_ms(),
        rtt_ms: best.rtt_ms(),
        sample_count: samples.len(),
        method: method.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_is_server_minus_local_midpoint() {
        // Symmetric 100ms round trip; server clock is 5000ms ahead of
        // the local midpoint (1050).
        let s = PingSample {
            local_send_ms: 1000,
            server_ms: 6050,
            local_recv_ms: 1100,
        };
        assert_eq!(s.rtt_ms(), 100);
        assert_eq!(s.offset_ms(), 5000);
    }

    #[test]
    fn negative_offset_when_server_behind() {
        let s = PingSample {
            local_send_ms: 10_000,
            server_ms: 7_000,
            local_recv_ms: 10_020,
        };
        // local midpoint 10_010; 7_000 - 10_010 = -3_010.
        assert_eq!(s.offset_ms(), -3_010);
    }

    #[test]
    fn non_monotonic_local_clock_clamps_rtt() {
        let s = PingSample {
            local_send_ms: 500,
            server_ms: 900,
            local_recv_ms: 400, // clock stepped backwards mid-call
        };
        assert_eq!(s.rtt_ms(), 0);
    }

    #[test]
    fn best_sample_has_lowest_rtt() {
        let samples = vec![
            // Noisy: 400ms RTT, offset would be 200.
            PingSample {
                local_send_ms: 0,
                server_ms: 1200,
                local_recv_ms: 400,
            },
            // Clean: 20ms RTT, offset 1000.
            PingSample {
                local_send_ms: 1000,
                server_ms: 2010,
                local_recv_ms: 1020,
            },
        ];
        let est = estimate_offset(&samples, "packet_tap").unwrap();
        assert_eq!(est.rtt_ms, 20);
        assert_eq!(est.offset_ms, 1000);
        assert_eq!(est.sample_count, 2);
        assert_eq!(est.method, "packet_tap");
    }

    #[test]
    fn no_samples_returns_none() {
        assert!(estimate_offset(&[], "packet_tap").is_none());
        assert_eq!(ClockOffset::unestimated().method, "unestimated");
    }
}
