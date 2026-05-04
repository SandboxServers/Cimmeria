//! Fuzz target for `FragmentAssembler::process_parsed`.
//!
//! Fragment reassembly is the second corruption-exposed surface:
//! after `parse_incoming` succeeds on a `FLAG_FRAGMENTED` packet,
//! `process_parsed` reads the parsed FOOTER fields (`seq_id`,
//! `frag_begin`, `frag_end`) — NOT a body header — to derive the
//! reassembly key, this fragment's index, and the total fragment
//! count. The body bytes are the fragment's payload. Adversarial
//! inputs (frag_begin > frag_end, range exceeding MAX_FRAGMENTS,
//! seq outside [frag_begin, frag_end], repeated indices, conflicting
//! frag_begin / frag_end across fragments) must not panic, OOM, or
//! unboundedly allocate.
//!
//! The harness chains parse_incoming → process_parsed so every
//! input is exercised through the production decode path. We accept
//! that some inputs will fail parse_incoming and skip — the goal is
//! that NEITHER call panics on any input.

#![no_main]

use cimmeria_mercury::packet::parse_incoming;
use cimmeria_mercury::unpacker::FragmentAssembler;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Build a fresh assembler per input so state doesn't accumulate
    // across cases — keeps the corpus minimisation work focused on
    // the parser/assembler logic rather than on state-machine paths
    // that depend on prior inputs.
    let mut asm = FragmentAssembler::new();
    if let Ok(pkt) = parse_incoming(data) {
        let _ = asm.process_parsed(&pkt);
    }
});
