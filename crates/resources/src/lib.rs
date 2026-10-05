//! # cimmeria-resources
//!
//! The server's copy of the client's cooked data, and the edits Cimmeria makes
//! to it:
//!
//! - [`base::resources`]: `ResourceCache`, which loads every `data/cache/*.pak`
//!   at startup, applies the overrides below in memory and bumps each patched
//!   category's metadata, so the client's `versionInfoRequest` handshake
//!   re-fetches exactly the patched entries. Also the inventory bag tables
//!   (`BAG_FILL_ORDER`, `bag_max_slots`, `bag_min_slot`).
//! - [`base::mission_overrides`], [`base::item_overrides`],
//!   [`base::dialog_overrides`] and [`base::sequence_overrides`]: the
//!   per-category override tables and the XML patchers that apply them.
//! - [`base::chardef`]: the CharDefId identity table character creation reads.
//! - [`base::start_profiles`]: where each char_def starts and what it starts
//!   with (Class Start v6 CS-02), read from `resources.char_creation`.
//!
//! The wire side (answering `versionInfoRequest`, streaming
//! `RESOURCE_FRAGMENT`s) stays in `cimmeria-services` (`base::cooked_data`).
//!
//! Split out of `cimmeria-services` (wave W1b of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its
//! old nesting under `base`, so `crate::base::…` and `super::…` paths inside
//! it are unchanged, and `cimmeria-services` re-exports each module at its
//! old path (`cimmeria_services::base::resources`, …).

#![warn(unreachable_pub)]

pub mod base {
    pub mod attribute_patches;
    pub mod chardef;
    pub mod dialog_overrides;
    pub mod item_overrides;
    pub mod mission_overrides;
    pub mod resources;
    pub mod sequence_overrides;
    pub mod stargate_overrides;
    pub mod start_profiles;
    pub mod world_info_overrides;
}

// Generic helpers come from `cimmeria-test-support` (a dev-dependency), so the
// moved tests keep importing them from `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_test_support::*;
}
