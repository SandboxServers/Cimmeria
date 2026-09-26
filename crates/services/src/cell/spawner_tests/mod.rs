//! The spawner test that drives the base-side GM spawn handler.
//!
//! - [`template_prototype_parity`]: live-DB guard that the cell's startup
//!   template cache and the base-side GM spawn handler map an
//!   `entity_templates` row identically (PR #662 review, finding 3). It drives
//!   base code, so it waits here for wave F with the other cross-track tests.
//!
//! The spawner's DB loaders moved to `cimmeria-cell-catalog` (wave W2b of
//! docs/architecture/services-crate-split.md) with the tests that need nothing
//! above them. The rest of this module's tests went to the crate of their
//! highest dependency in wave C6, under the same `cell::spawner_tests` path:
//! the Harset suite (`harset/`, which drives the NPC AI's ability selector) to
//! `cimmeria-cell-combat`, and the `SpaceManager`, cover and aggression tests
//! to `cimmeria-cell-world`.

mod template_prototype_parity;
