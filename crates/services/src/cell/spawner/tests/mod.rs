//! Tests for the `spawner` module, split by theme (issue #529):
//! - [`spawn_records`]: in-memory NPC id allocation, class-id mapping, and
//!   `spawn_npc_from_record` behavior against a hand-built `SpaceManager`.
//! - [`live_db_loaders`]: live-DB sanity guards for the spawner loader queries.
//! - [`harset`]: live-DB guards for the Harset entity templates seeded by
//!   rebuild packet H11 — template rows, faction design rules, and the two new
//!   ability sets.
//! - [`live_db_content_loaders`]: the same, for the mission / objective /
//!   dialog-set / monologue loaders.
//! - [`template_prototype_parity`]: live-DB guard that the cell's startup
//!   template cache and the base-side GM spawn handler map an
//!   `entity_templates` row identically (PR #662 review, finding 3).

mod harset;
mod live_db_content_loaders;
mod live_db_loaders;
mod spawn_records;
mod template_prototype_parity;
