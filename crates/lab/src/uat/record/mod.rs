//! Lab record: a recording is the raw log of one person playing a section,
//! written tick by tick from the client probes and the server packet tap.
//! The log records what was seen and inferred nothing; the draft-spec
//! transform (LR-05) is the only step that turns it into a spec, and it
//! writes drafts for review. Ledger output is in [`super::ledger`].

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
