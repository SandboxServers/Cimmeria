//! Fuzz target for `FragmentAssembler::process_parsed`.
//!
//! Fragment reassembly is the second corruption-exposed surface:
//! after `parse_incoming` succeeds, fragmented packets feed their
//! body bytes through `process_parsed`, which reads a 6-byte header
//! (fragment_index, total_fragments, first_seq) and dispatches to
//! the assembler. A pathological header (claiming 255 fragments,
//! claiming index ≥ total, varying first_seq across fragments,
//! sending the same fragment twice) is the second class of input
//! that must not panic / OOM.
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
