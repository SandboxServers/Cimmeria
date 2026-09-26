//! The spawner tests that drive the NPC AI, which is this crate's.
//!
//! - [`harset`]: live-DB guards for the Harset entity templates seeded by
//!   rebuild packet H11 — template rows, faction design rules, the two new
//!   ability sets and the placements built on them. Its ability-set guards
//!   drive the NPC AI's ability selector (`service::npc_ai`), so the suite
//!   sits here rather than in `cimmeria-cell-world` with the other spawner
//!   tests.
//!
//! Until wave C6 of the services crate split
//! (docs/architecture/services-crate-split.md) this was `cimmeria-services`'
//! `cell::spawner_tests::harset`; it keeps that path here.

mod harset;
