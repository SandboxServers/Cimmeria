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
//! - [`normalise`] — the variance model applied to a tap and window samples.
//! - [`build`] — a run directory as a fingerprint.
//! - [`agree`] — whether five runs may become a golden; [`diff`] — the
//!   first place a run leaves it; [`file`] — the golden on disk.
//!
//! The tools are `lab_golden_record` and `lab_golden_diff`
//! (`crate::server::golden`).

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
