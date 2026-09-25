//! Tests for the `spawner` module, split by theme (issue #529):
//! - [`spawn_records`]: in-memory NPC id allocation, class-id mapping, and
//!   `spawn_npc_from_record` behavior against a hand-built `SpaceManager`.
//! - [`live_db_loaders`]: live-DB sanity guards for the spawner loader queries
//!   themselves — column renames, type drift, JOIN breakage.
//! - [`harset`]: live-DB guards for the Harset entity templates seeded by
//!   rebuild packet H11 — template rows, faction design rules, and the two new
//!   ability sets.
//! - [`live_db_content_loaders`]: the same, for the mission / objective /
//!   dialog-set / monologue loaders.
//! - [`live_db_castle_seed`]: live-DB guards for Castle (World 8) *seed content*
//!   that loads fine and is nonetheless wrong — actors outside the box that is
//!   meant to contain them, missing display names, missing respawn timers.
//! - [`template_prototype_parity`]: live-DB guard that the cell's startup
//!   template cache and the base-side GM spawn handler map an
//!   `entity_templates` row identically (PR #662 review, finding 3).

mod harset;
mod live_db_castle_seed;
mod live_db_content_loaders;
mod live_db_loaders;
mod spawn_grounding;
mod spawn_records;
mod template_prototype_parity;
