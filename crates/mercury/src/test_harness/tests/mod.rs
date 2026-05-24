//! Paired-channel test inventory for the loopback Mercury harness.
//!
//! One file per category from the issue spec (#352):
//!
//! - `smoke` — minimal "A sends, B receives" wiring proof (Phase 1).
//! - `reliable` — reliable-delivery under simulated loss.
//! - `fragment` — fragment reassembly across paired channels.
//! - `keepalive` — keepalive cadence end-to-end.
//! - `encryption` — encryption round-trip across multiple bundles.
//! - `handshake` — channel lifecycle handshake.
//! - `ack` — ack aggregation.
//! - `rto` — adaptive RTO convergence on loopback.

mod ack;
mod encryption;
mod encryption_kat;
mod fragment;
mod handshake;
mod kat_vectors;
mod keepalive;
mod reliable;
mod rto;
mod smoke;
