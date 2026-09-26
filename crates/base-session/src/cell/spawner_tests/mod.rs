//! The spawner test that drives the base-side GM spawn handler.
//!
//! - [`template_prototype_parity`]: live-DB guard that the cell's startup
//!   template cache (the spawner loaders in `cimmeria-cell-catalog`) and this
//!   crate's GM spawn handler map an `entity_templates` row identically (PR
//!   #662 review, finding 3).
//!
//! It was `cell::spawner_tests::template_prototype_parity` in
//! `cimmeria-services` until wave F of the services crate split
//! (docs/architecture/services-crate-split.md): it needs only the catalog
//! loaders and `base::gm_spawn`, and this crate is the lowest one above
//! both, so it moved here under the same module path. The rest of the
//! spawner's tests are with the loaders (catalog) or at the same path in
//! `cimmeria-cell-world` and `cimmeria-cell-combat`.

mod template_prototype_parity;
