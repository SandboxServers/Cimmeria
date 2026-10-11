//! The emitters' two-row shape. The sampler's arithmetic is tested beside
//! it in `cimmeria-observability`, the mercury firehose in
//! `cimmeria_mercury::firehose`.
//!
//! The routing half (full row reaches a file layer and no OTLP layer, sample
//! reaches OTLP) is tested against the real filters in
//! `crates/server/src/logging/parity_tests/firehose_sampling.rs`.

use std::net::SocketAddr;

use super::*;
use crate::test_support::LogCapture;

fn addr() -> SocketAddr {
    "127.0.0.1:32832".parse().unwrap()
}

/// Every call writes the full row; only the admitted ones add a sample, and
/// the sample carries the ratio. Reverting the `admit()` gate in
/// `log_decrypt_ok` makes the sample count equal the full count and fails.
#[test]
fn decrypt_ok_writes_every_full_row_and_a_counted_sample() {
    let capture = LogCapture::install();
    let sampler = FirehoseSampler::new(DECRYPT_OK_SAMPLE_EVERY);
    let n = 200;
    for _ in 0..n {
        log_decrypt_ok(&sampler, addr(), &[0xAB, 0x01]);
    }
    let all = capture.all();
    let full: Vec<_> = all
        .iter()
        .filter(|c| c.target == DECRYPT_OK_TARGET)
        .collect();
    let sampled: Vec<_> = all
        .iter()
        .filter(|c| c.target == DECRYPT_OK_SAMPLE_TARGET)
        .collect();
    assert_eq!(full.len(), n, "the files must keep every DECRYPT_OK");
    assert!(full.iter().all(|c| c.message_contains("DECRYPT_OK")
        && c.has_field("hex", "AB 01")
        && c.level == tracing::Level::TRACE));
    assert_eq!(sampled.len(), n.div_ceil(53));
    assert!(sampled
        .iter()
        .all(|c| c.has_field("sampled_1_in", "53") && c.has_field("hex", "AB 01")));
    assert!(sampled[0].has_field("suppressed", "0"));
    assert!(sampled[1].has_field("suppressed", "52"));
}

/// The `UDP_IN` sample must never carry the datagram bytes: before login the
/// datagram holds the `baseAppLogin` ticket.
#[test]
fn udp_in_sample_carries_no_hex() {
    let capture = LogCapture::install();
    let sampler = FirehoseSampler::new(UDP_IN_SAMPLE_EVERY);
    for _ in 0..60 {
        log_udp_in(&sampler, addr(), &[1, 2, 3]);
    }
    let all = capture.all();
    assert_eq!(all.iter().filter(|c| c.target == UDP_IN_TARGET).count(), 60);
    let sampled: Vec<_> = all
        .iter()
        .filter(|c| c.target == UDP_IN_SAMPLE_TARGET)
        .collect();
    assert_eq!(sampled.len(), 2);
    for c in sampled {
        assert!(!c.fields.contains_key("hex"), "ticket bytes leak: {c:?}");
        assert!(c.has_field("len", "3") && c.has_field("sampled_1_in", "53"));
    }
}

#[test]
fn entity_moved_keeps_the_avatar_update_sample_and_adds_the_ratio() {
    let capture = LogCapture::install();
    let sampler = FirehoseSampler::new(AOI_POSITION_SAMPLE_EVERY);
    let row = EntityMovedRow {
        witness_id: 7,
        entity_id: 900,
        position: [1.0, 2.0, 3.0],
        direction: [0.0, 1.5, 0.0],
        velocity: [0.5, 0.0, 0.0],
        npc_moved_since_last: Some(true),
    };
    for _ in 0..250 {
        log_entity_moved(&sampler, &row);
    }
    let all = capture.all();
    let full = all
        .iter()
        .filter(|c| c.target == AOI_POSITION_TARGET)
        .filter(|c| c.message_contains("AoI: entity position update"))
        .count();
    let sampled: Vec<_> = all
        .iter()
        .filter(|c| c.target == AOI_POSITION_SAMPLE_TARGET)
        .collect();
    assert_eq!(full, 250);
    assert_eq!(sampled.len(), 3, "ceil(250 / 101)");
    let s = sampled[2];
    assert_eq!(s.level, tracing::Level::DEBUG);
    assert!(s.message_contains("UPDATE_AVATAR sent (sampled)"));
    assert!(s.has_field("sampled_1_in", "101") && s.has_field("suppressed", "100"));
    assert!(s.has_field("entity_id", "900") && s.has_field("vx", "0.5"));
}

/// The server's parity test trusts this table; pin its shape here so a
/// renamed target or a lost row fails in this crate too.
#[test]
fn firehose_table_names_disjoint_prefixed_targets() {
    assert_eq!(FIREHOSES.len(), 4);
    assert!(
        FIREHOSES
            .iter()
            .any(|f| f.full_target == "wire.firehose.mercury_packet"
                && f.sample_target == "mercury.packet"
                && f.sample_level == tracing::Level::INFO),
        "TG-MER-09: the transport's datagram firehose is registered"
    );
    for f in FIREHOSES {
        assert!(f.full_target.starts_with(FIREHOSE_TARGET_PREFIX), "{f:?}");
        assert!(
            !f.sample_target.starts_with(FIREHOSE_TARGET_PREFIX),
            "a sample on a firehose target would be turned off with it: {f:?}"
        );
        assert!(f.every > 1, "{f:?}");
    }
}
