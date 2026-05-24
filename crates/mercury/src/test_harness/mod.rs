//! Loopback Mercury session harness for end-to-end protocol tests.
//!
//! Pairs two [`Channel`](crate::channel::Channel) instances on real
//! `127.0.0.1:0` loopback `UdpSocket`s and runs a driver loop that pumps
//! recv on both sides, dispatches inbound bytes to the right channel,
//! drives `tick()` at a controllable cadence, and exposes hooks for
//! simulated loss / reorder / latency / duplication.
//!
//! # When to reach for this vs. its siblings
//!
//! - [`crate::test_transport::TestTransport`] (Tier 1) records outbound
//!   `(SocketAddr, bytes)` for byte-exact fan-out assertions. Right when
//!   the question is "did handler H emit these bytes?".
//! - **This harness** (Tier 2) drives two endpoints exchanging real
//!   packets over real loopback sockets. Right when the question is
//!   "does the Mercury protocol deliver these bytes under conditions X?"
//!   — retransmit, fragment reassembly, keepalive, encryption.
//! - The content-aware wireclient is the end-to-end driver one floor
//!   up; it walks scripts with auth, entity mirrors, and dialog
//!   state. That layer is for "does the game work?", not "does the
//!   wire work?".
//!
//! # Ordering, isolation, and runtime guarantees
//!
//! These are the contract bits tests can rely on:
//!
//! - **Per-direction ordering is preserved.** If A sends packets 1, 2, 3
//!   to B, B observes them in that order (subject to any active
//!   `reorder_pairs` or `drop_next` policy in the A→B direction). Same
//!   for B→A.
//! - **Cross-direction interleaving is implementation-defined unless
//!   serialized via `tick()`.** If A sends 1, 2, 3 and B sends 4, 5, 6
//!   simultaneously, the relative ordering across directions is NOT
//!   guaranteed. Tests that need a precise interleaving must drive the
//!   harness with explicit `session.tick().await` calls between phases.
//! - **Per-session [`TestClock`] isolation.** Each [`LoopbackPeer`] owns
//!   its own `Arc<TestClock>` — no process-global state. `cargo nextest`
//!   parallel runs cannot perturb each other's clock.
//! - **Per-session port isolation.** Each [`LoopbackPeer`] binds
//!   `127.0.0.1:0` so the OS assigns ephemeral ports. Parallel runs
//!   cannot collide.
//! - **Tokio runtime defaults to `#[tokio::test]` (current-thread).**
//!   Recv pumps are driven cooperatively via `tokio::select!`. Tests
//!   that want to assert "neither direction stalls the other" must opt
//!   into `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`
//!   and document why in the test header.
//!
//! # Known harness simplifications vs production
//!
//! - **Non-fragmented packets bypass the RX window.** Production's
//!   recv path routes reliable non-fragmented packets through
//!   `Channel::receive_packet`, which de-duplicates by sequence and
//!   delivers in-order. The harness recv pump pushes every arrival
//!   straight to the inbox in wire-arrival order — so duplicates and
//!   reorderings are observable from the test side, not silently
//!   smoothed. This makes wire-level effect tests (the
//!   `policy_effects` module) straightforward to assert on. Fragmented
//!   packets DO go through the per-channel `FragmentAssembler`, which
//!   handles its own duplicate-fragment dedup.

mod clock;
pub mod invariants;
mod peer;
mod policy;
mod session;

// Pcap replay depends on `pcap-file` + `etherparse` which live in
// `[dev-dependencies]` (not available to feature-gated lib code).
// Gate the module to `cfg(test)` so the lomiada replay tests under
// mercury's own test suite can use it.
//
// **Consumer-visibility limitation**: because of the `cfg(test)`
// gate, downstream crates' integration tests CANNOT use this module
// even with `features = ["test-harness"]`. The pcap-replay path is
// mercury-internal-only today. If a consumer crate needs it, the
// path is to lift `pcap-file` + `etherparse` to optional
// `[dependencies]` tied to a new `test-harness-pcap` feature; do
// that work in the same PR that adds the consumer use case so the
// scope is justified.
#[cfg(test)]
pub mod pcap_replay;

#[cfg(test)]
mod tests;

pub use clock::TestClock;
pub use peer::{LoopbackPeer, TickActions};
pub use policy::{Direction, DropProbability, NetworkDirection, NetworkPolicy};
pub use session::LoopbackSession;
