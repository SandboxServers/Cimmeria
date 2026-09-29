//! # cimmeria-cell-effect-scripts
//!
//! The effect scripts: step 4 of the plugin migration in
//! `docs/architecture/plugin-architecture.md` (#962), a registry move rather
//! than a plugin.
//!
//! - [`EFFECT_SCRIPTS`] is the table of every script, keyed by the
//!   `script_name` an effect row carries. The composition root
//!   (`cimmeria-services`) builds the cell's
//!   [`EffectScripts`](cimmeria_cell_world::cell::effects::registry::EffectScripts)
//!   registry from it at startup, and the cell installs that on its
//!   `SpaceManager`, where `dispatch_by_name` and `dispatch_on_remove` look a
//!   script up.
//! - [`cell::effects`] holds the `EffectScript` implementations, moved from
//!   `cimmeria-cell-world` with their tests: the heals, the damage, shield,
//!   stun and suppression scripts, Cover Stance, the pet scripts, the
//!   stimpack stat buff and the special-ammo scripts. The module re-exports
//!   the world's effect layer, so the moved code keeps its `super::…` and
//!   `crate::cell::effects::…` paths.
//!
//! Nothing depends on this crate but the composition root and test code, so
//! editing or adding a script rebuilds this crate, the facade and the
//! binaries only. The effect runtime stays below, in `cimmeria-cell-world`
//! (the trait, the context, the registry type, the passive pass, the
//! stat-buff ledger and the ammo shot helpers combat calls) and
//! `cimmeria-cell-combat` (pulsing, channels, the stat-buff tick).
//!
//! **Adding a script:** write a zero-sized struct with an `impl EffectScript`
//! in the module for its family, add one `("Name", &Type)` row to
//! [`EFFECT_SCRIPTS`], and seed the effect row's `script_name`.

#![warn(unreachable_pub)]

pub mod cell;

pub use cell::effects::registry::{effect_scripts, EFFECT_SCRIPTS};

// Generic helpers come from `cimmeria-test-support` (a dev-dependency) and the
// world fixtures from `cimmeria_cell_world::test_fixtures`, so the moved tests
// keep importing both from `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_cell_world::test_fixtures::*;
    pub(crate) use cimmeria_test_support::*;
}
