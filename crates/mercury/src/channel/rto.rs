//! Retransmission timeout (RTO) state machine.
//!
//! Per-channel adaptive RTO computation following RFC 6298 (TCP RTO),
//! the same algorithm BigWorld's reliable-UDP layer was conceptually
//! modeled on. Tracks smoothed round-trip time and round-trip variance
//! from clean ACK samples; on retransmit, exponential backoff.
//!
//! Adaptive (not fixed) because observed real-world RTT spans 5 ms LAN
//! through 400 ms transatlantic — an 80× range that no single fixed
//! timeout can serve without either spurious retransmits on slow links
//! or painfully slow recovery on fast ones.
//!
//! ## Karn's algorithm
//!
//! ACK samples from retransmitted packets are ambiguous — the receiver
//! ACK'd *some* copy and we don't know which one. Feeding such samples
//! to the smoothing would bias `sRTT` toward whichever copy won the
//! race, producing unstable timeouts. This module does NOT detect
//! retransmits on its own; the caller MUST only invoke [`Rto::on_sample`]
//! for samples drawn from packets that were never retransmitted, and
//! invoke [`Rto::on_retransmit`] for the backoff side. The contract is
//! enforced upstream in the `Channel` send-window logic.
//!
//! ## Arithmetic
//!
//! All operations use integer nanoseconds. The RFC 6298 fractions
//! (`1/8`, `1/4`, `3/4`, `7/8`) are exact powers of two, so the
//! integer form has no floating-point drift; test expectations can be
//! hand-computed to the nanosecond. The only loss is the
//! integer-division truncation at each step (`(3*x + y) / 4` rounds
//! toward zero), which is RFC-conformant and stable.

use std::time::Duration;

/// Configuration knobs for the [`Rto`] state machine.
///
/// All four fields are session-overridable. Defaults chosen for
/// Cimmeria's observed client population (LAN through transatlantic).
#[derive(Debug, Clone, Copy)]
pub struct RtoConfig {
    /// Lower clamp on the computed RTO. Protects against absurdly tight
    /// timeouts on near-zero-latency LAN sessions where micro-jitter
    /// would otherwise fire spurious retransmits. Default: 100 ms.
    pub min: Duration,

    /// Upper clamp on the computed RTO. Caps the worst-case spiral if
    /// the link goes briefly catastrophic (mobile handoff, congestion
    /// event). Past this, connection-dead detection should be taking
    /// over via the max-retransmit count. Default: 4 s.
    pub max: Duration,

    /// Seed value used until the first ACK sample lands. Choose
    /// generously — too low produces spurious retransmits on slow
    /// links during the first round trip; too high delays first-loss
    /// recovery before any sample has arrived. Default: 500 ms.
    pub initial_srtt: Duration,
}

impl Default for RtoConfig {
    fn default() -> Self {
        Self {
            min: Duration::from_millis(100),
            max: Duration::from_secs(4),
            initial_srtt: Duration::from_millis(500),
        }
    }
}

/// Per-channel RTO state.
///
/// Holds smoothed RTT, RTT variance, and the currently-effective
/// retransmission timeout. Updated by [`on_sample`] when a clean ACK
/// arrives, and by [`on_retransmit`] when a packet's RTO fires before
/// its ACK lands.
///
/// [`on_sample`]: Rto::on_sample
/// [`on_retransmit`]: Rto::on_retransmit
#[derive(Debug, Clone)]
pub struct Rto {
    config: RtoConfig,

    /// Smoothed round-trip time. `None` until the first sample arrives;
    /// `Some(r)` thereafter, never reverting to `None`.
    srtt: Option<Duration>,

    /// Round-trip time variance. Same lifecycle as `srtt` — `None`
    /// until first sample, then `Some(_)` for the lifetime of the
    /// channel.
    rttvar: Option<Duration>,

    /// Currently-effective retransmission timeout. Always within
    /// `[config.min, config.max]`. Seeded from `config.initial_srtt`
    /// via the same RFC 6298 formula used after the first sample
    /// (`SRTT + 4 × RTTVAR`, with `RTTVAR = SRTT / 2`), so the initial
    /// RTO is 3 × `initial_srtt` clamped.
    rto: Duration,
}

impl Rto {
    /// Construct a new RTO state machine seeded from `config`.
    ///
    /// The initial `rto` is `3 × config.initial_srtt`, clamped to
    /// `[config.min, config.max]` — this matches the RFC 6298
    /// "first-sample" formula (`SRTT + 4 × RTTVAR` with
    /// `RTTVAR = SRTT / 2`) applied to the seed value, so the seed
    /// represents *what the first sample would have been*, not the
    /// RTO value directly.
    pub fn new(config: RtoConfig) -> Self {
        let initial_rto = clamp(config.initial_srtt * 3, config.min, config.max);
        Self {
            config,
            srtt: None,
            rttvar: None,
            rto: initial_rto,
        }
    }

    /// Currently-effective RTO. Always within `[config.min, config.max]`.
    pub fn current(&self) -> Duration {
        self.rto
    }

    /// Currently-smoothed RTT. `None` until the first sample arrives.
    /// Exposed for diagnostics / logging — not used by the algorithm
    /// itself outside this struct.
    pub fn srtt(&self) -> Option<Duration> {
        self.srtt
    }

    /// Currently-smoothed RTT variance. `None` until the first sample
    /// arrives. Exposed for diagnostics / logging.
    pub fn rttvar(&self) -> Option<Duration> {
        self.rttvar
    }

    /// Record a clean ACK sample.
    ///
    /// The caller MUST ensure `rtt` was measured against a packet that
    /// was never retransmitted (Karn's algorithm — see module docs).
    /// Feeding ambiguous samples here biases the smoothing and produces
    /// unstable timeouts.
    ///
    /// On the first call: `SRTT = R`, `RTTVAR = R / 2`, per RFC 6298 §2.2.
    /// On subsequent calls: standard smoothing with α = 1/8, β = 1/4.
    pub fn on_sample(&mut self, rtt: Duration) {
        let rtt_ns: u128 = rtt.as_nanos();

        match (self.srtt, self.rttvar) {
            (None, _) => {
                // First sample: SRTT = R, RTTVAR = R/2 (RFC 6298 §2.2)
                self.srtt = Some(rtt);
                self.rttvar = Some(Duration::from_nanos((rtt_ns / 2) as u64));
            }
            (Some(srtt), Some(rttvar)) => {
                let srtt_ns = srtt.as_nanos();
                let rttvar_ns = rttvar.as_nanos();
                let abs_diff_ns = srtt_ns.max(rtt_ns) - srtt_ns.min(rtt_ns);

                // RTTVAR = (3 × RTTVAR + |SRTT − R|) / 4
                let new_rttvar_ns = (3 * rttvar_ns + abs_diff_ns) / 4;
                // SRTT   = (7 × SRTT + R) / 8
                let new_srtt_ns = (7 * srtt_ns + rtt_ns) / 8;

                self.srtt = Some(Duration::from_nanos(new_srtt_ns as u64));
                self.rttvar = Some(Duration::from_nanos(new_rttvar_ns as u64));
            }
            (Some(_), None) => {
                // Invariant violation — srtt and rttvar move together.
                unreachable!("rttvar must be Some whenever srtt is Some");
            }
        }
        self.recompute_rto_from_smoothing();
    }

    /// Record a retransmission of the oldest in-flight packet.
    ///
    /// Doubles the current RTO (Karn's exponential backoff), clamped
    /// to `config.max`. Does NOT touch `srtt` or `rttvar` — those keep
    /// their pre-retransmit values so the next clean ACK sample can
    /// resume smoothing from a known-good state.
    pub fn on_retransmit(&mut self) {
        let doubled = self.rto.saturating_mul(2);
        self.rto = clamp(doubled, self.config.min, self.config.max);
    }

    /// Recompute `rto` from the current `srtt` / `rttvar` and clamp.
    /// Called after every [`on_sample`].
    fn recompute_rto_from_smoothing(&mut self) {
        let srtt = self.srtt.expect("on_sample sets srtt before this runs");
        let rttvar = self.rttvar.expect("on_sample sets rttvar before this runs");
        let raw = srtt.saturating_add(rttvar.saturating_mul(4));
        self.rto = clamp(raw, self.config.min, self.config.max);
    }
}

/// `Duration::clamp` is unstable; provide our own.
fn clamp(value: Duration, min: Duration, max: Duration) -> Duration {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-computed deterministic sequence pinning the RFC 6298
    /// smoothing formula. The expected values are computed in integer
    /// nanoseconds to match the implementation's truncation behavior.
    ///
    /// Sequence: `[50ms, 60ms, 55ms, 200ms]` — chosen to exercise:
    /// 1. First-sample bootstrap (`SRTT = R`, `RTTVAR = R/2`)
    /// 2. Smoothing convergence on a stable link (samples 2 + 3)
    /// 3. RTO spike response to a single outlier (sample 4)
    ///
    /// Hand math (all in nanoseconds, integer-divided per impl):
    ///
    /// Step 1 (rtt=50ms, first sample):
    ///   srtt   = 50_000_000
    ///   rttvar = 25_000_000
    ///   rto    = 50_000_000 + 4*25_000_000 = 150_000_000 (150 ms)
    ///
    /// Step 2 (rtt=60ms):
    ///   |srtt − rtt| = 10_000_000
    ///   rttvar = (3*25_000_000 + 10_000_000) / 4 = 21_250_000
    ///   srtt   = (7*50_000_000 + 60_000_000) / 8 = 51_250_000
    ///   rto    = 51_250_000 + 4*21_250_000 = 136_250_000 (136.25 ms)
    ///
    /// Step 3 (rtt=55ms):
    ///   |srtt − rtt| = 3_750_000
    ///   rttvar = (3*21_250_000 + 3_750_000) / 4 = 16_875_000
    ///   srtt   = (7*51_250_000 + 55_000_000) / 8 = 51_718_750
    ///   rto    = 51_718_750 + 4*16_875_000 = 119_218_750 (119.218750 ms)
    ///
    /// Step 4 (rtt=200ms, spike):
    ///   |srtt − rtt| = 148_281_250
    ///   rttvar = (3*16_875_000 + 148_281_250) / 4 = 49_726_562 (truncated from .5)
    ///   srtt   = (7*51_718_750 + 200_000_000) / 8 = 70_253_906 (truncated from .25)
    ///   rto    = 70_253_906 + 4*49_726_562 = 269_160_154 (~269.16 ms)
    #[test]
    fn rto_smoothing_matches_rfc6298_hand_computed_sequence() {
        let mut rto = Rto::new(RtoConfig::default());

        rto.on_sample(Duration::from_millis(50));
        assert_eq!(rto.srtt(), Some(Duration::from_nanos(50_000_000)));
        assert_eq!(rto.rttvar(), Some(Duration::from_nanos(25_000_000)));
        assert_eq!(rto.current(), Duration::from_nanos(150_000_000));

        rto.on_sample(Duration::from_millis(60));
        assert_eq!(rto.srtt(), Some(Duration::from_nanos(51_250_000)));
        assert_eq!(rto.rttvar(), Some(Duration::from_nanos(21_250_000)));
        assert_eq!(rto.current(), Duration::from_nanos(136_250_000));

        rto.on_sample(Duration::from_millis(55));
        assert_eq!(rto.srtt(), Some(Duration::from_nanos(51_718_750)));
        assert_eq!(rto.rttvar(), Some(Duration::from_nanos(16_875_000)));
        assert_eq!(rto.current(), Duration::from_nanos(119_218_750));

        rto.on_sample(Duration::from_millis(200));
        assert_eq!(rto.srtt(), Some(Duration::from_nanos(70_253_906)));
        assert_eq!(rto.rttvar(), Some(Duration::from_nanos(49_726_562)));
        assert_eq!(rto.current(), Duration::from_nanos(269_160_154));
    }

    /// Initial RTO before any sample arrives is `3 × initial_srtt`
    /// clamped to `[min, max]`. This represents "what would the RTO
    /// be if the very first sample equaled `initial_srtt`" — keeps
    /// the bootstrap value consistent with the first-sample formula.
    #[test]
    fn initial_rto_is_three_times_initial_srtt_clamped() {
        let rto = Rto::new(RtoConfig {
            min: Duration::from_millis(100),
            max: Duration::from_secs(4),
            initial_srtt: Duration::from_millis(500),
        });
        assert_eq!(rto.current(), Duration::from_millis(1500));
        assert_eq!(rto.srtt(), None, "srtt is None before the first sample");
        assert_eq!(rto.rttvar(), None, "rttvar is None before the first sample");
    }

    /// `RTO_MIN` floor catches sub-floor values produced by very low
    /// RTT samples on LAN links. Without the floor, micro-jitter (a
    /// 0 ms RTT measurement on a loopback link, for example) would
    /// drive RTO to zero and fire spurious retransmits every tick.
    #[test]
    fn rto_clamps_to_min_floor_on_low_rtt_samples() {
        let mut rto = Rto::new(RtoConfig {
            min: Duration::from_millis(100),
            max: Duration::from_secs(4),
            initial_srtt: Duration::from_millis(500),
        });
        // First sample at 1 ms produces srtt=1ms, rttvar=500us,
        // raw rto = 1ms + 4*500us = 3ms — below the 100ms floor.
        rto.on_sample(Duration::from_millis(1));
        assert_eq!(rto.current(), Duration::from_millis(100));
        assert_eq!(rto.srtt(), Some(Duration::from_millis(1)));
    }

    /// `RTO_MAX` ceiling catches the post-spike spiral. After a single
    /// large sample on an otherwise quiet link, `srtt + 4*rttvar` can
    /// far exceed any reasonable upper bound; without the clamp,
    /// subsequent retransmit backoffs would push it further still.
    #[test]
    fn rto_clamps_to_max_ceiling_on_high_rtt_samples() {
        let mut rto = Rto::new(RtoConfig {
            min: Duration::from_millis(100),
            max: Duration::from_secs(4),
            initial_srtt: Duration::from_millis(500),
        });
        // First sample of 10 seconds → raw rto = 10s + 4*5s = 30s,
        // clamps to RTO_MAX = 4s.
        rto.on_sample(Duration::from_secs(10));
        assert_eq!(rto.current(), Duration::from_secs(4));
        assert_eq!(rto.srtt(), Some(Duration::from_secs(10)));
    }

    /// Karn's backoff: `on_retransmit` doubles RTO and clamps to MAX.
    /// `srtt` / `rttvar` are NOT touched — they keep their last-known
    /// good values so the next clean sample resumes smoothing from
    /// a known state instead of from the backed-off RTO.
    #[test]
    fn on_retransmit_doubles_rto_and_preserves_smoothing_state() {
        let mut rto = Rto::new(RtoConfig::default());
        rto.on_sample(Duration::from_millis(100));
        // Post-first-sample: srtt=100ms, rttvar=50ms, rto=300ms.
        let srtt_before = rto.srtt();
        let rttvar_before = rto.rttvar();
        assert_eq!(rto.current(), Duration::from_millis(300));

        rto.on_retransmit();
        assert_eq!(rto.current(), Duration::from_millis(600));
        assert_eq!(
            rto.srtt(),
            srtt_before,
            "srtt must not change on retransmit"
        );
        assert_eq!(
            rto.rttvar(),
            rttvar_before,
            "rttvar must not change on retransmit"
        );

        rto.on_retransmit();
        assert_eq!(rto.current(), Duration::from_millis(1200));

        rto.on_retransmit();
        assert_eq!(rto.current(), Duration::from_millis(2400));

        // Next doubling = 4800 ms, clamps to RTO_MAX = 4 s.
        rto.on_retransmit();
        assert_eq!(rto.current(), Duration::from_secs(4));

        // Further retransmits stay pinned at MAX (saturating).
        rto.on_retransmit();
        assert_eq!(rto.current(), Duration::from_secs(4));
    }

    /// After a backoff, the next clean sample resumes smoothing using
    /// the preserved `srtt` / `rttvar` (not the backed-off RTO). This
    /// is the Karn-algorithm payoff: a single retransmit doesn't
    /// permanently poison the smoothing.
    #[test]
    fn clean_sample_after_retransmit_resumes_smoothing_from_preserved_state() {
        let mut rto = Rto::new(RtoConfig::default());
        rto.on_sample(Duration::from_millis(100));
        // srtt=100ms, rttvar=50ms, rto=300ms

        rto.on_retransmit();
        assert_eq!(rto.current(), Duration::from_millis(600));

        // Next clean sample uses srtt=100ms, rttvar=50ms as the base —
        // NOT the 600ms backed-off rto.
        rto.on_sample(Duration::from_millis(110));
        // |srtt − rtt| = 10ms
        // rttvar = (3*50ms + 10ms) / 4 = 40ms
        // srtt   = (7*100ms + 110ms) / 8 = 101.25ms
        // rto    = 101.25ms + 4*40ms = 261.25ms
        assert_eq!(rto.srtt(), Some(Duration::from_nanos(101_250_000)));
        assert_eq!(rto.rttvar(), Some(Duration::from_millis(40)));
        assert_eq!(rto.current(), Duration::from_nanos(261_250_000));
    }

    /// Negative-drift sample (rtt < srtt) takes the absolute value of
    /// the difference for the RTTVAR update. Pins the `|sRTT − R|`
    /// computation against a regression that would naively subtract
    /// in one direction and wrap on the other.
    #[test]
    fn negative_drift_sample_uses_absolute_difference() {
        let mut rto = Rto::new(RtoConfig::default());
        rto.on_sample(Duration::from_millis(100));
        // srtt=100ms, rttvar=50ms

        // rtt=80ms is BELOW srtt — abs_diff = 20ms.
        rto.on_sample(Duration::from_millis(80));
        // rttvar = (3*50ms + 20ms) / 4 = 42.5ms
        // srtt   = (7*100ms + 80ms) / 8 = 97.5ms
        // rto    = 97.5ms + 4*42.5ms = 267.5ms
        assert_eq!(rto.rttvar(), Some(Duration::from_nanos(42_500_000)));
        assert_eq!(rto.srtt(), Some(Duration::from_nanos(97_500_000)));
        assert_eq!(rto.current(), Duration::from_nanos(267_500_000));
    }

    /// Pathological zero-RTT sample — possible on a single-process
    /// loopback test scenario. First-sample formula produces
    /// `srtt = 0`, `rttvar = 0`, `rto = 0`, which the floor lifts to
    /// `min`. Pin so future regressions can't silently produce a
    /// zero-RTO that spurious-retransmits every tick.
    #[test]
    fn zero_rtt_sample_clamps_to_min() {
        let mut rto = Rto::new(RtoConfig::default());
        rto.on_sample(Duration::ZERO);
        assert_eq!(rto.srtt(), Some(Duration::ZERO));
        assert_eq!(rto.rttvar(), Some(Duration::ZERO));
        assert_eq!(rto.current(), RtoConfig::default().min);
    }
}
