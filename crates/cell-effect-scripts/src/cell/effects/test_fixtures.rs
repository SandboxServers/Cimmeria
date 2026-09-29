//! Shared fixtures for the effect-script unit tests (`scripts`, `heal`,
//! `stat_buff`, the ammo families).
//!
//! The world half is `cimmeria_cell_world::test_fixtures::effects`; this
//! wrapper installs the script registry on the manager, as the cell does at
//! startup, so `dispatch_by_name` finds the scripts.

use crate::cell::space_manager::SpaceManager;

pub(crate) use crate::test_support::effect_with_nvp;

/// The world's effect target (`make_mgr_with_target`: one space `W`, player
/// entity 1 at HEALTH 50/100 and FOCUS 200/1000), with every script
/// registered.
pub(crate) fn make_mgr_with_target() -> SpaceManager {
    let mut mgr = crate::test_support::make_mgr_with_target();
    super::registry::install(&mut mgr);
    mgr
}
