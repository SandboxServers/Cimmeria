//! The transport's per-datagram firehose (TG-MER-09, owner decision D-TG7).
//!
//! `mercury.packet` used to be one INFO row per UDP datagram: about half of
//! every row SigNoz stored (63M a week), each saying only peer and length.
//! It now follows the `cimmeria_wire::firehose` pattern:
//!
//! - every datagram writes the full row on [`MERCURY_PACKET_TARGET`] at
//!   TRACE, which `protocol.log` keeps and every OTLP filter turns off;
//! - every [`MERCURY_PACKET_SAMPLE_EVERY`]th datagram per direction also
//!   writes the `mercury.packet` INFO row, with `sampled_1_in` and
//!   `suppressed` so `sum(1 + suppressed)` is the true count;
//! - a failed send writes its own `mercury.packet` row every time, unsampled;
//! - `mercury_datagrams_total{dir}` counts every datagram, so rates are
//!   exact without the sample.
//!
//! The targets and the emitter live here rather than in
//! `cimmeria_wire::firehose` because `cimmeria-wire` depends on this crate,
//! not the other way round. The wire crate re-exports them and lists this
//! firehose in its `FIREHOSES` table, which the server's parity tests walk.

use std::io;
use std::net::SocketAddr;

use cimmeria_observability::FirehoseSampler;

use crate::instrumentation::Direction;

/// Every datagram, in full. Files only.
pub const MERCURY_PACKET_TARGET: &str = "wire.firehose.mercury_packet";

/// The sampled row, exported to `cimmeria-network` at INFO. Same target the
/// per-datagram row used before TG-MER-09, so existing SigNoz queries keep
/// working; they now need `1 + suppressed` to count.
pub const MERCURY_PACKET_SAMPLE_TARGET: &str = "mercury.packet";

/// 1-in-N per direction. Same N as the `DECRYPT_OK` and `UDP_IN` firehoses,
/// which sample the same datagram stream; prime so a client's periodic packet
/// mix cannot phase-lock the sample onto one packet kind.
pub const MERCURY_PACKET_SAMPLE_EVERY: u64 = 53;

/// Counter behind the exact per-direction datagram rate.
pub const METRIC_DATAGRAMS: &str = "mercury_datagrams_total";

/// One datagram through the UDP transport: count it, write its full row, and
/// write the sampled row when `sampler` admits it. `error` is a failed
/// send's error; that row is written whatever the sampler says.
pub fn log_mercury_packet(
    sampler: &FirehoseSampler,
    dir: Direction,
    len: usize,
    peer: SocketAddr,
    error: Option<&io::Error>,
) {
    let dir = dir.as_str();
    let ok = error.is_none();
    cimmeria_observability::counter!(METRIC_DATAGRAMS, "dir" => dir);
    tracing::trace!(
        target: MERCURY_PACKET_TARGET,
        dir,
        transport = "udp",
        len,
        peer = %peer,
        ok,
        "mercury_packet"
    );
    // A failure is an anomaly, never sampled (D-TG7). The literal target is
    // the same string as `MERCURY_PACKET_SAMPLE_TARGET`; it stays a literal
    // so the server's source-target scan sees `mercury.packet` at INFO.
    if let Some(e) = error {
        tracing::info!(
            target: "mercury.packet",
            dir,
            transport = "udp",
            len,
            peer = %peer,
            ok = false,
            error = %e,
            "mercury_packet_failed"
        );
    }
    if let Some(suppressed) = sampler.admit() {
        tracing::info!(
            target: MERCURY_PACKET_SAMPLE_TARGET,
            dir,
            transport = "udp",
            len,
            peer = %peer,
            ok,
            sampled_1_in = sampler.every(),
            suppressed,
            "mercury_packet"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing::{Event, Subscriber};

    use super::*;

    /// One captured event: its target, message and the sampling field.
    #[derive(Debug, Default, Clone)]
    struct Row {
        target: String,
        message: String,
        suppressed: Option<u64>,
    }

    struct Visitor<'a>(&'a mut Row);
    impl Visit for Visitor<'_> {
        fn record_u64(&mut self, field: &Field, value: u64) {
            if field.name() == "suppressed" {
                self.0.suppressed = Some(value);
            }
        }
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.0.message = format!("{value:?}").trim_matches('"').to_string();
            }
        }
    }

    /// Keeps every event, TRACE included. Mercury has no `LogCapture`
    /// (`cimmeria-test-support` sits above it), so this is the crate's usual
    /// hand-rolled subscriber (see `unified.rs`).
    struct Sink(Arc<Mutex<Vec<Row>>>);
    impl Subscriber for Sink {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &Event<'_>) {
            let mut row = Row {
                target: event.metadata().target().to_string(),
                ..Row::default()
            };
            event.record(&mut Visitor(&mut row));
            self.0.lock().unwrap().push(row);
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    fn capture(f: impl FnOnce()) -> Vec<Row> {
        let rows = Arc::new(Mutex::new(Vec::new()));
        tracing::subscriber::with_default(Sink(Arc::clone(&rows)), f);
        let captured = rows.lock().unwrap().clone();
        captured
    }

    fn peer() -> SocketAddr {
        "127.0.0.1:9".parse().unwrap()
    }

    /// 106 datagrams: 106 full rows and 2 counted samples. Dropping the
    /// `admit()` gate writes 106 sampled rows and fails.
    #[test]
    fn mercury_packet_sample_accounts_for_every_datagram() {
        let sampler = FirehoseSampler::new(MERCURY_PACKET_SAMPLE_EVERY);
        let rows = capture(|| {
            for _ in 0..106 {
                log_mercury_packet(&sampler, Direction::Out, 8, peer(), None);
            }
        });

        let full = rows
            .iter()
            .filter(|r| r.target == MERCURY_PACKET_TARGET)
            .count();
        assert_eq!(full, 106, "protocol.log must keep every datagram");

        let sampled: Vec<&Row> = rows
            .iter()
            .filter(|r| r.target == MERCURY_PACKET_SAMPLE_TARGET)
            .collect();
        assert_eq!(sampled.len(), 2, "one sampled row per 53 datagrams");
        // The framework samples datagrams 1 and 54; `suppressed` counts the
        // drops BEFORE each sample, so the sum reaches the last sample (54),
        // and the 52 after it are counted by the next one.
        assert_eq!(sampled[0].suppressed, Some(0));
        assert_eq!(sampled[1].suppressed, Some(52));
        let covered: u64 = sampled.iter().map(|r| 1 + r.suppressed.unwrap()).sum();
        assert_eq!(covered, 54, "sum(1 + suppressed) up to the last sample");
    }

    /// Every failed send keeps its own row, whatever the sampler decides.
    /// Gating the failure row behind `admit()` writes one row and fails.
    #[test]
    fn failed_send_is_logged_on_mercury_packet_unsampled() {
        let sampler = FirehoseSampler::new(MERCURY_PACKET_SAMPLE_EVERY);
        let err = io::Error::other("no route");
        let rows = capture(|| {
            for _ in 0..3 {
                log_mercury_packet(&sampler, Direction::Out, 8, peer(), Some(&err));
            }
        });
        let failed: Vec<&Row> = rows
            .iter()
            .filter(|r| r.message == "mercury_packet_failed")
            .collect();
        assert_eq!(failed.len(), 3, "every failed send keeps its row");
        assert!(failed.iter().all(|r| r.target == "mercury.packet"));
        assert_eq!("mercury.packet", MERCURY_PACKET_SAMPLE_TARGET);
    }

    /// The counter sees every datagram, per direction, sampled or not.
    #[test]
    fn datagram_counter_counts_every_datagram() {
        use cimmeria_observability::testing::{counter_total, install};
        install();
        let sampler = FirehoseSampler::new(MERCURY_PACKET_SAMPLE_EVERY);
        let (out0, in0) = (
            counter_total(METRIC_DATAGRAMS, &[("dir", "out")]),
            counter_total(METRIC_DATAGRAMS, &[("dir", "in")]),
        );
        for _ in 0..60 {
            log_mercury_packet(&sampler, Direction::Out, 8, peer(), None);
        }
        for _ in 0..7 {
            log_mercury_packet(&sampler, Direction::In, 8, peer(), None);
        }
        assert_eq!(
            counter_total(METRIC_DATAGRAMS, &[("dir", "out")]) - out0,
            60
        );
        assert_eq!(counter_total(METRIC_DATAGRAMS, &[("dir", "in")]) - in0, 7);
    }
}
