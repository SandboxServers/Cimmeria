//! Property test: `parse_incoming` must never panic, regardless of
//! input length or contents.
//!
//! The stable-Rust complement to the cargo-fuzz target at
//! `fuzz/fuzz_targets/parse_incoming.rs` (run via
//! `cargo +nightly fuzz run parse_incoming` from `fuzz/`).
//! Fuzzing requires nightly Rust (libfuzzer-sys); this proptest runs
//! in every per-PR CI invocation under stable. It catches the
//! obvious panic sources (unchecked indexing, integer overflow,
//! pathological allocations driven by user-supplied counts) without
//! the coverage-guided exploration cargo-fuzz brings.
//!
//! Contract: for ANY byte slice, `parse_incoming` returns either
//! `Ok(ParsedPacket)` or `Err(CimmeriaError)`. A panic is a bug.

use super::parse_incoming;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        // 4096 random byte sequences per run. Much higher than the
        // 256-case round-trip property because the input space here
        // is wider (all u8 sequences in 0..=1500 bytes long) and the
        // assertion is just "didn't panic" — fast to evaluate.
        cases: 4096,
        ..ProptestConfig::default()
    })]

    /// Arbitrary byte input must never crash the parser. Length range
    /// 0..=1500 covers: empty buffer (BufferUnderflow path), every
    /// length up through Ethernet MTU-sized UDP datagrams (Mercury's
    /// PACKET_MAX_SIZE is 1472), and inputs longer than any valid
    /// frame so the bound-strip arithmetic in `pop_u32_le!` etc.
    /// gets exercised on payloads where every footer would still
    /// have room. parse_incoming has no upper-length cap of its own,
    /// so any byte budget here is fine — capping at 1500 keeps the
    /// proptest runtime bounded.
    #[test]
    fn parse_incoming_never_panics_on_arbitrary_bytes(
        raw in proptest::collection::vec(any::<u8>(), 0..=1500),
    ) {
        // The result is intentionally discarded — we only care that
        // the call returns rather than panicking. Wrapping in
        // catch_unwind would convert a panic into a test failure,
        // but the test runner already does that for us — a panic in
        // this body fails the property and proptest shrinks the
        // input to a minimal repro.
        let _ = parse_incoming(&raw);
    }

    /// Specifically exercise the FLAG_HAS_REQUESTS / FLAG_HAS_ACKS /
    /// FLAG_HAS_SEQUENCE / FLAG_FRAGMENTED bits in the leading flags
    /// byte. The general property above also hits these by chance,
    /// but seeding the first byte with a flag combination
    /// systematically makes the footer-strip paths get exercised
    /// even when the body is too short to hold them — the failure
    /// mode that triggers BufferUnderflow.
    #[test]
    fn parse_incoming_handles_truncated_footer_input_without_panic(
        flags in any::<u8>(),
        body in proptest::collection::vec(any::<u8>(), 0..=64),
    ) {
        let mut raw = Vec::with_capacity(1 + body.len());
        raw.push(flags);
        raw.extend_from_slice(&body);
        let _ = parse_incoming(&raw);
    }
}
