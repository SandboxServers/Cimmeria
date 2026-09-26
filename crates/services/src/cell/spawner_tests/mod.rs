//! Tests of the spawner's records against `SpaceManager`, combat and the
//! base-side GM spawn handler, split by theme (issue #529).
//!
//! The spawner's DB loaders moved to `cimmeria-cell-catalog`
//! (`cell::spawner`), and the tests that need nothing above them moved with
//! them. These stay here because each needs a module still in this crate.
//!
//! - [`spawn_records`]: in-memory NPC id allocation, class-id mapping, and
//!   `spawn_npc_from_record` behavior against a hand-built `SpaceManager`.
//! - [`harset`]: live-DB guards for the Harset entity templates seeded by
//!   rebuild packet H11 — template rows, faction design rules, and the two new
//!   ability sets.
//! - [`live_db_leash_distance`]: live-DB guards that
//!   `entity_templates.leash_distance` (NA12) loads without a COALESCE,
//!   reaches the spawned NPC, and rejects `0`.
//! - [`live_db_use_cover`]: live-DB guards that `entity_templates.use_cover`
//!   (NA22) and the Cover Stance effect rows load as seeded, and that a
//!   seeded guard spawns holding its seeded cover slot.
//! - [`template_prototype_parity`]: live-DB guard that the cell's startup
//!   template cache and the base-side GM spawn handler map an
//!   `entity_templates` row identically (PR #662 review, finding 3).

mod harset;
mod live_db_aggression;
mod live_db_assist;
mod live_db_eye_heights;
mod live_db_leash_distance;
mod live_db_use_cover;
mod spawn_grounding;
mod spawn_records;
mod template_prototype_parity;
