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
//!   a cached pet template and a ready owner (issue #570).

mod content_events;
pub mod occluder_fixtures;
mod pets;
mod space_manager;

pub use content_events::{NoContentEvents, RecordedContentEvent, RecordingContentEvents};
pub use pets::{
    add_pet_owner, pet_template_record, seed_pet_template, PET_FIXTURE_ABILITIES,
    PET_FIXTURE_TEMPLATE_ID,
};
pub use space_manager::{make_space_manager, make_space_manager_with_player, seed_ability_defs};

pub use crate::cell::arrival::{test_fixture_mesh, test_insert_navmesh_space};
