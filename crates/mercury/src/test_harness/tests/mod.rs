//! Paired-channel test inventory for the loopback Mercury harness.
//!
//! One file per category from the harness spec:
//!
//! - `smoke` — minimal "A sends, B receives" wiring proof (Phase 1).
//! - `reliable` — reliable-delivery under simulated loss.
//! - `fragment` — fragment reassembly across paired channels.
//! - `keepalive` — keepalive cadence end-to-end.
//! - `encryption` — encryption round-trip across multiple bundles.
//! - `handshake` — channel lifecycle handshake.
//! - `ack` — ack aggregation.
//! - `rto` — adaptive RTO convergence on loopback.
//! - `rotation` — server-initiated v2 session-key rotation across a live
//!   session, including rotate-under-load.

mod ack;
mod chaos;
mod concurrent;
mod encryption;
mod encryption_kat;
mod fragment;
mod handshake;
mod kat_vectors;
mod keepalive;
mod policy_effects;
mod reliable;
mod rotation;
mod rto;
mod smoke;
