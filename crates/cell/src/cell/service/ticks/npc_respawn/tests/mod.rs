//! Tests for the `npc_respawn` tick, split by theme (issue #529):
//! - [`fixtures`]: the shared `make_mgr_with_dead_npc` / `drain` helpers.
//! - [`respawn_lifecycle`]: state reset, wire ordering, direction restore,
//!   counted-flag draining, idempotency loop, and no-op deadline cases.
//! - [`respawn_witness_loot`]: loot-UI close, zero-witness silence, the
//!   damage_apply integration loop, and the movement-type player guard.
//! - [`harset_respawn`]: live-DB guards on the Harset hub spawn rows
//!   (Harset packet H13 / defect H-B7) — seed row through to promotion.

mod fixtures;
mod harset_respawn;
mod respawn_lifecycle;
mod respawn_witness_loot;
