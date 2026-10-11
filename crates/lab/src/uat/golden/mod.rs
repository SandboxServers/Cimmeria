//! Golden runs: a fingerprint is the normalised, comparable record of one
//! UAT run (its rows' events, transitions, windows and clause verdicts).
//! A golden is only written once five fresh runs agree on it, because no
//! single run is golden (owner rule, docs/analysis/lab-golden/README.md).
//!
//! The entity file in `fixtures/` is synthetic. Its tags are the seed's
//! (`db/resources/Worlds/Seed/spawnlist.sql`), not a capture.
//!
//! - [`event`] — the normalised event, the variance and the fingerprints.
//! - [`labels`] — entity ids as stable names (`self`, `tag:…`, `?1`).
//! - [`key_args`] — the outbound and inbound key-argument tables.
//! - [`normalise`], [`build`], [`agree`], [`diff`], [`file`] — filled by
//!   the later GD- packets.

// `cimmeria-lab` is a binary crate, so its items are dead until the golden
// tools are wired in (GD-07 removes this allow). It applies to test builds
// too: the tests do not build every type, and the re-exports are unused
// until then.
#![allow(dead_code, unused_imports)]

pub mod agree;
pub mod build;
pub mod diff;
pub mod event;
pub mod file;
pub mod key_args;
pub mod labels;
pub mod normalise;
#[cfg(test)]
pub(crate) mod testkit;

pub use event::{Dir, Event, RowFingerprint, RunFingerprint, Variance, FINGERPRINT_SCHEMA};
