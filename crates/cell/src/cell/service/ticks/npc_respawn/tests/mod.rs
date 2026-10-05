//! Tests for the `npc_respawn` tick, split by theme (issue #529):
//! - [`fixtures`]: the shared `make_mgr_with_dead_npc` / `drain` helpers.
//! - [`respawn_lifecycle`]: state reset, wire ordering, direction restore,
//!   counted-flag draining, idempotency loop, and no-op deadline cases.
//! - [`respawn_witness_loot`]: loot-UI close, zero-witness silence, the
//!   damage_apply integration loop, and the movement-type player guard.
//! - [`respawn_target_clear`]: #844, a respawn drops the corpse selection.
//! - [`respawn_recreate`]: the respawned NPC is re-created on every
//!   witness (`LeftAoI` + AoI-enter introduction) so it stands up.
//! - [`respawn_names`]: NT-25, the respawn row pairs ids with names.
//! - [`harset_respawn`]: live-DB guards on the Harset hub spawn rows
//!   (Harset packet H13 / defect H-B7) — seed row through to promotion.

mod fixtures;
mod harset_respawn;
mod respawn_lifecycle;
mod respawn_names;
mod respawn_recreate;
mod respawn_target_clear;
mod respawn_witness_loot;
