//! Crafting subsystem — BaseApp side.
//!
//! Phase 1 (this module) provides persistence helpers for `CraftingState`:
//! the load/save round-trip between `sgw_player` + `sgw_player_discipline_expertise`
//! and the in-memory `cimmeria_entity::crafting::CraftingState`.
//!
//! Phases 2-5 (craft/research/alloy/reveng activities, full wire emission,
//! respec) live in the cell layer and will land in follow-up PRs.
//!
//! See issue #53 deep dive for the full subsystem design.

pub mod persistence;

// Re-exports are deferred until Phase 2 wires up the world-entry load + the
// cell-side activity handlers. The persistence functions are pub on their
// module path; callers can reach them as `base::crafting::persistence::*`
// either way. Eliding the re-export here avoids an unused-import warning
// under `-D warnings` during Phase 1 when nothing outside this module's
// tests calls into the persistence path yet.
