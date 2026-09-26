//! The per-packet firehoses through the real filters: every row reaches its
//! file, and SigNoz gets a counted 1-in-N sample on another target.

use cimmeria_services::firehose;

use super::harness;
use crate::logging::filters::FILE_LAYERS;

// ── Firehose sampling through the real filters ───────────────────────────

/// Run `emit` `n` times through the harness; return
/// (rows in `file`, sampled rows exported, sum of `1 + suppressed`).
fn run_firehose(n: u64, file: &str, full_target: &str, emit: impl Fn()) -> (u64, u64, u64) {
    let (dispatch, hits) = harness(FILE_LAYERS);
    tracing::dispatcher::with_default(&dispatch, || {
        for _ in 0..n {
            emit();
        }
    });
    let hits = hits.lock().unwrap().clone();
    let in_file = hits
        .iter()
        .filter(|h| h.sink == format!("file:{file}") && h.target == full_target)
        .count() as u64;
    assert!(
        !hits
            .iter()
            .any(|h| h.sink.starts_with("otlp:") && h.target == full_target),
        "the full firehose {full_target} must not be exported"
    );
    let sampled: Vec<_> = hits
        .iter()
        .filter(|h| h.sink.starts_with("otlp:") && h.target != full_target)
        .collect();
    let reconstructed = sampled.iter().map(|h| 1 + h.suppressed.unwrap_or(0)).sum();
    (in_file, sampled.len() as u64, reconstructed)
}

fn addr() -> std::net::SocketAddr {
    "127.0.0.1:32832".parse().unwrap()
}

/// `n` DECRYPT_OK rows: all `n` in base.log, `ceil(n / 53)` in
/// `cimmeria-trace`, and the samples' counts add back up. Dropping the
/// `admit()` gate fails the sample count (1000 vs 19, verified); a firehose
/// target missing from `wire.firehose=off`'s reach fails the "not exported"
/// assertion.
#[test]
fn decrypt_ok_is_complete_on_disk_and_sampled_in_signoz() {
    let s = firehose::FirehoseSampler::new(firehose::DECRYPT_OK_SAMPLE_EVERY);
    let n = 1_000;
    let (file, sampled, sum) = run_firehose(n, "base.log", firehose::DECRYPT_OK_TARGET, || {
        firehose::log_decrypt_ok(&s, addr(), &[0xAB]);
    });
    assert_eq!(file, n);
    assert_eq!(sampled, n.div_ceil(53));
    assert_eq!(sum, (sampled - 1) * 53 + 1);
}

#[test]
fn udp_in_is_complete_on_disk_and_sampled_in_signoz() {
    let s = firehose::FirehoseSampler::new(firehose::UDP_IN_SAMPLE_EVERY);
    let n = 500;
    let (file, sampled, _) = run_firehose(n, "base.log", firehose::UDP_IN_TARGET, || {
        firehose::log_udp_in(&s, addr(), &[1, 2]);
    });
    assert_eq!((file, sampled), (n, n.div_ceil(53)));
}

/// The AoI relay: every row in world_entry.log, a 1-in-101
/// `wire.out.avatar_update` DEBUG sample in `cimmeria-server`.
#[test]
fn aoi_position_is_complete_on_disk_and_sampled_in_signoz() {
    let s = firehose::FirehoseSampler::new(firehose::AOI_POSITION_SAMPLE_EVERY);
    let row = firehose::EntityMovedRow {
        witness_id: 1,
        entity_id: 2,
        position: [0.0; 3],
        direction: [0.0; 3],
        velocity: [0.0; 3],
        npc_moved_since_last: None,
    };
    let n = 1_010;
    let (file, sampled, sum) =
        run_firehose(n, "world_entry.log", firehose::AOI_POSITION_TARGET, || {
            firehose::log_entity_moved(&s, &row)
        });
    assert_eq!(file, n);
    assert_eq!(sampled, n.div_ceil(101));
    assert_eq!(sum, (sampled - 1) * 101 + 1);
}
