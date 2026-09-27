//! Tests of the spawner's records against `SpaceManager`, cover and the
//! aggression helpers, split by theme (issue #529).
//!
//! The spawner's DB loaders are in `cimmeria-cell-catalog` (`cell::spawner`),
//! with the tests that need nothing above them. These need this crate's
//! `SpaceManager`. Until wave C6 of the services crate split
//! (docs/architecture/services-crate-split.md) they were `cimmeria-services`'
//! `cell::spawner_tests`, left there by wave W2b; they keep that path here.
//!
//! - [`spawn_records`]: in-memory NPC id allocation, class-id mapping, and
//!   `spawn_npc_from_record` behavior against a hand-built `SpaceManager`.
//! - [`spawn_behaviour_row`]: the `spawner.npc_behaviour` row carries
//!   `world`, `space_id`, the ability set, its event sets and the weapon (NA44).
//! - [`spawn_grounding`]: NA11, a seeded spawn a little off the navmesh floor
//!   is moved onto it.
//! - [`live_db_aggression`] and [`live_db_assist`]: live-DB guards for the
//!   NA13 aggro radius and aggression override and the NA14 assist radius.
//! - [`live_db_eye_heights`]: live-DB guards for the NA31 body-set eye
//!   heights line of sight casts between.
//! - [`live_db_leash_distance`]: live-DB guards that
//!   `entity_templates.leash_distance` (NA12) loads without a COALESCE,
//!   reaches the spawned NPC, and rejects `0`.
//! - [`live_db_use_cover`]: live-DB guards that `entity_templates.use_cover`
//!   (NA22) and the Cover Stance effect rows load as seeded, and that a
//!   seeded guard spawns holding its seeded cover slot.
//! - [`live_db_vault_scope`]: live-DB guards that `entity_templates.vault_scope`
//!   (bank-vault BV-02) defaults to `personal`, rejects an unknown scope, and
//!   reaches a spawned Banker.
//!
//! The Harset template guards (`harset/`) drive the NPC AI's ability
//! selector, so they are `cimmeria-cell-combat`'s `cell::spawner_tests`; the
//! template-vs-GM-spawn parity guard drives the base's GM spawn handler and
//! stays in `cimmeria-services`.

mod live_db_aggression;
mod live_db_assist;
mod live_db_eye_heights;
mod live_db_leash_distance;
mod live_db_use_cover;
mod live_db_vault_scope;
mod spawn_behaviour_row;
mod spawn_grounding;
mod spawn_records;
