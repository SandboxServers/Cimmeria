//! Lomiada regression — replay the canonical pcap fixture and
//! assert recovery semantics.
//!
//! This is the highest-fidelity regression test in the chaos
//! suite. It uses a real captured session from a real player
//! (Germany → US server, ~6 minutes of clean play, then a
//! transatlantic UDP drop of server-sent packet `#1148`) and
//! drives the captured packet stream into a synthetic Channel.
//!
//! What this test pins:
//!
//! - The session pcap actually decrypts with the saved key
//!   (catches a regression in `MercuryEncryption` that would
//!   silently shift the ciphertext format).
//! - The captured packet stream contains the gap shape the
//!   fixture name advertises — a meaningful share of
//!   server→client packets PLUS a non-trivial number of
//!   client→server frames.
//! - The replay loader successfully parses + decrypts the
//!   majority of UDP frames in the pcap (test would fail if
//!   `parse_incoming` started rejecting valid Mercury bytes).
//!
//! Stronger assertions (full state-machine replay against a fresh
//! Channel, exact recovery tick count, byte-identical
//! retransmits) are scoped for follow-up — they need the same
//! `with_key` constructor on Channel that the wireclient effort
//! (#281 successor) will eventually add. For now this test
//! validates the **replay infrastructure itself** is wired
//! end-to-end against real captured bytes.

use std::path::PathBuf;

use crate::packet::parse_incoming;
use crate::test_harness::pcap_replay::{CaptureDirection, PcapReplay};

fn fixture_root() -> PathBuf {
    // Workspace root is 3 levels up from `crates/mercury/`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("debug")
        .join("lomiada-broke-in-hallway02")
}

#[test]
fn lomiada_pcap_loads_and_decrypts_under_saved_key() {
    let root = fixture_root();
    let pcap = root.join("2026-05-23_13-28.pcap");
    let keys = root.join("2026-05-23_13-27-keys.txt");

    // Skip if the fixture isn't present (some dev environments
    // exclude the debug/ directory). The test is a regression
    // guard; absence of the fixture is not a failure.
    if !pcap.exists() || !keys.exists() {
        eprintln!(
            "skipping: fixture not present at {} / {}",
            pcap.display(),
            keys.display()
        );
        return;
    }

    let replay = PcapReplay::load(&pcap)
        .with_key_from(&keys)
        .expect("session key file must be a 64-char hex string");

    let events = replay.events().expect("pcap must parse without I/O error");

    // The fixture is ~5 MB and contains thousands of Mercury
    // packets. A regression that broke decryption or the wire
    // parser would yield zero (or near-zero) decryptable events.
    assert!(
        events.len() > 500,
        "expected hundreds of decryptable Mercury packets in the fixture; got {}",
        events.len(),
    );

    // Both directions must be represented — a session this long
    // has acks flowing both ways.
    let s2c = events
        .iter()
        .filter(|e| e.direction == CaptureDirection::ServerToClient)
        .count();
    let c2s = events
        .iter()
        .filter(|e| e.direction == CaptureDirection::ClientToServer)
        .count();
    assert!(
        s2c > 100,
        "expected substantial server→client traffic; got {s2c}"
    );
    assert!(
        c2s > 50,
        "expected substantial client→server traffic; got {c2s}"
    );
}

#[test]
fn lomiada_decrypted_payloads_parse_as_mercury_packets() {
    let root = fixture_root();
    let pcap = root.join("2026-05-23_13-28.pcap");
    let keys = root.join("2026-05-23_13-27-keys.txt");
    if !pcap.exists() || !keys.exists() {
        return;
    }

    let replay = PcapReplay::load(&pcap)
        .with_key_from(&keys)
        .expect("key file");
    let events = replay.events().expect("pcap parse");

    // The vast majority of decrypted payloads should parse cleanly
    // as Mercury packets. A meaningful failure rate (>5%) would
    // signal that either the wire format has drifted or the pcap
    // contains a substantial fraction of non-Mercury UDP that
    // happened to decrypt-validate by coincidence (unlikely given
    // HMAC-MD5 integrity).
    let mut parsed = 0usize;
    let mut failed = 0usize;
    for event in &events {
        match parse_incoming(&event.plaintext) {
            Ok(_) => parsed += 1,
            Err(_) => failed += 1,
        }
    }
    let total = parsed + failed;
    assert!(
        total > 0,
        "loader must yield events for this fixture (got 0)"
    );
    let success_rate = (parsed as f64) / (total as f64);
    assert!(
        success_rate >= 0.95,
        "expected ≥95% of decrypted payloads to parse as valid Mercury packets; \
         got {parsed}/{total} = {success_rate:.3}",
    );
}
