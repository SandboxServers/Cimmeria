//! Fuzz target for `parse_incoming` — the wire-level UDP datagram parser.
//!
//! This is the most corruption-exposed surface in the codebase: every
//! UDP packet that reaches the server is fed through it before any
//! authentication or encryption check. A panic / OOM / pathological
//! allocation triggered by adversarial input is a denial-of-service
//! vector.
//!
//! Contract under test: `parse_incoming` must NEVER panic, regardless
//! of input length / contents. It must EITHER return `Ok(ParsedPacket)`
//! OR return `Err(CimmeriaError)`. Crashes here are bugs.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Drop the result — we only care about not panicking. libfuzzer
    // catches panics, OOMs, and timeouts; the harness's only job is
    // to drive the function with arbitrary bytes.
    let _ = cimmeria_mercury::packet::parse_incoming(data);
});
