//! Fixtures for this crate's tests and, behind the `test-support` feature,
//! the tests of the crates above it (docs/architecture/services-crate-split.md
//! §3).
//!
//! - [`npc_detectors`]: the one-NPC spaces the NA02 detector tests share.
//!   `ground`, `stale_velocity` and one `state` test drive the service loop's
//!   movement tick, so they are `cimmeria-cell`'s (`cimmeria-services`' until
//!   wave C6), which imports the same fixtures.
//! - [`npc_surrender`]: the H08 surrender guards' Castle space, players,
//!   hostile NPCs and auto-fire loop. `auto_cycle` and `health_crossing` drive
//!   the service loop's auto-cycle tick, so they are `cimmeria-cell`'s too.

pub mod npc_detectors;
pub mod npc_surrender;
