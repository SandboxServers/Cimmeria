//! Tests for the `spawner` module, split by theme (issue #529):
//! - [`spawn_records`]: in-memory NPC id allocation, class-id mapping, and
//!   `spawn_npc_from_record` behavior against a hand-built `SpaceManager`.
//! - [`live_db_loaders`]: live-DB sanity guards for the spawner loader queries.
//! - [`harset`]: live-DB guards for the Harset entity templates seeded by
//!   rebuild packet H11 — template rows, faction design rules, and the two new
//!   ability sets.

mod harset;
mod live_db_loaders;
mod spawn_records;
