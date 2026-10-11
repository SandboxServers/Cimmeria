//! Lab record: a recording is the raw log of one person playing a section,
//! written tick by tick from the client probes and the server packet tap.
//! The draft-spec transform (LR-05) turns a log into a draft spec through a
//! fixed table and infers nothing: a fact outside the table becomes a
//! `# TODO` line. A draft is unconfirmed until `lab golden record` agrees on
//! it five times. Campaign ledger and decisions:
//! `docs/analysis/lab-record/README.md`.

// cimmeria-lab is a binary crate, so these items are dead until LR-07 wires
// the record tools in.
#![cfg_attr(not(test), allow(dead_code))]

pub mod bug_spec;
pub mod clauses;
pub mod decode;
pub mod emit;
pub mod fixture;
pub mod log;
pub mod pose;
pub mod rolling;
pub mod sampler;
pub mod segment;

// Unused until LR-07 (and in the test build, where nothing reads the re-export).
#[allow(unused_imports)]
pub use log::*;
