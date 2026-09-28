//! Test fixtures for the world types (docs/architecture/services-crate-split.md
//! §3).
//!
//! Compiled for this crate's own tests and, behind the `test-support` feature,
//! for the tests of every crate above it. Those crates re-export this module
//! from their `test_support` shim, so a test keeps writing
//! `crate::test_support::make_space_manager()` wherever it lives.
//!
//! - [`make_space_manager`], [`make_space_manager_with_player`] and
//!   [`seed_ability_defs`]: the standard one-space `SpaceManager`.
//! - [`occluder_fixtures`]: box-built occluders for line-of-sight tests.
//! - [`test_fixture_mesh`] and [`test_insert_navmesh_space`]: a space with the
//!   shipped Castle Cellblock navmesh, for arrival and containment tests.
//! - [`NoContentEvents`] and [`RecordingContentEvents`]: the
//!   `ContentEvents` fakes.
//! - [`pet_template_record`], [`seed_pet_template`] and [`add_pet_owner`]:
//!   a cached pet template and a ready owner (issue #570); [`make_pet_world`]
//!   and [`watched_pet_world`] build the pet lifecycle world, and the
//!   `drain_*_for` / [`assert_pet_fully_gone`] helpers read its results.
//! - [`seed_deployable`] and the `DEPLOYABLE_*` constants: the seeded 1012
//!   Microwave Emitter (ability, effects, template 400, deployables row).
//! - [`npc_spawn_record`]: a template-shaped `SpawnRecord` for spawning an
//!   NPC through the real spawn-time derivations.

mod content_events;
mod deployables;
pub mod occluder_fixtures;
mod pets;
mod space_manager;
mod spawn_record;

pub use content_events::{NoContentEvents, RecordedContentEvent, RecordingContentEvents};
pub use deployables::{
    deployable_ability_def, deployable_effect_defs, deployable_template_record, seed_deployable,
    DEPLOYABLE_ABILITY, DEPLOYABLE_FLAGS, DEPLOYABLE_LIFETIME_EFFECT, DEPLOYABLE_MAX_RANGE,
    DEPLOYABLE_PULSE_EFFECT, DEPLOYABLE_SPEC, DEPLOYABLE_TEMPLATE, DEPLOYABLE_WARMUP,
};
pub use pets::{
    add_pet_owner, assert_pet_fully_gone, drain_entity_moved_for, drain_left_aoi_for,
    make_pet_world, pet_template_record, seed_pet_template, watched_pet_world,
    PET_FIXTURE_ABILITIES, PET_FIXTURE_OTHER, PET_FIXTURE_OWNER, PET_FIXTURE_TEMPLATE_ID,
};
pub use space_manager::{make_space_manager, make_space_manager_with_player, seed_ability_defs};
pub use spawn_record::npc_spawn_record;

pub use crate::cell::arrival::{test_fixture_mesh, test_insert_navmesh_space};
