//! Network-chaos scenario tests.
//!
//! Each scenario reproduces a real-or-suspected failure mode that
//! pure unit tests on the Channel state machine cannot exercise.
//! All scenarios use the `LoopbackSession` paired-channel harness
//! with the chaos primitives in `NetworkPolicy` (probabilistic
//! drop, multi-packet reorder, one-shot duplicate, etc.).
//!
//! Each test ends with an [`super::super::invariants::all_safety_invariants`]
//! check so silent state corruption fails the test alongside any
//! scenario-specific assertion.
//!
//! Naming: each file is one scenario. File name matches the
//! function name for easy `cargo nextest --filter` lookups.

mod asymmetric_ack_loss;
mod burst_loss_mid_stream;
mod defeat_burst_overflow;
mod duplicate_flood;
mod lomiada_single_packet_gap;
mod reorder_within_rx_window;
mod replay_lomiada;
mod sustained_5pct_loss_60s;
mod tx_window_overflow_with_recovery;
