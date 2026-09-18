//! Tests for the `spawner` module, split by theme (issue #529):
//! - [`spawn_records`]: in-memory NPC id allocation, class-id mapping, and
//!   `spawn_npc_from_record` behavior against a hand-built `SpaceManager`.
//! - [`live_db_loaders`]: live-DB sanity guards for the spawner loader queries.
//! - [`live_db_content_loaders`]: the same, for the mission / objective /
//!   dialog-set / monologue loaders.

mod live_db_content_loaders;
mod live_db_loaders;
mod spawn_records;
