//! The `EffectScript` implementations, and the table that registers them.
//!
//! The effect layer these scripts plug into (`EffectScript`, `EffectContext`,
//! `dispatch_by_name` / `dispatch_on_remove`, the `EffectScripts` registry
//! type, the passive pass, the stat-buff ledger and the ammo shot helpers) is
//! in `cimmeria_cell_world::cell::effects`, re-exported here whole so the
//! moved scripts keep their `super::…` paths. Three modules share a name with
//! a world module and extend it: [`registry`] (the table, beside the world's
//! registry type), [`pet_scripts`] (the pet scripts, beside the world's
//! script-name predicates) and [`stat_buff`] (the stimpack script, beside the
//! world's ledger). Each re-exports its world namesake.
//!
//! One file per script family, as before the move:
//!
//! - [`scripts`]: the damage, shield, stun and suppression scripts
//!   (`heal.rs`'s pool heals re-exported);
//! - [`heal`]: `HealHealth` and `HealFocus`;
//! - [`cover_stance`]: the Cover Stance grant and removal (NA22);
//! - [`pet_scripts`]: the owner abilities that act on a pet (pets PT-08);
//! - [`stat_buff`]: the consumable stimpacks' timed attribute buff;
//! - the special-ammo families (ammo campaign): [`ammo_dart_cc`],
//!   [`ammo_dart_support`], [`ammo_dart_tech`], [`ammo_emp`], and
//!   [`ammo_incendiary`] (the burn, which runs `RangedEnergyDamage`).

pub use cimmeria_cell_world::cell::effects::*;

pub mod ammo_dart_cc;
pub mod ammo_dart_support;
pub mod ammo_dart_tech;
pub mod ammo_emp;
pub mod ammo_incendiary;
pub mod cover_stance;
pub mod heal;
#[cfg(test)]
mod heal_seed_live_db_tests;
pub mod pet_scripts;
pub mod registry;
pub mod scripts;
pub mod stat_buff;
#[cfg(test)]
mod test_fixtures;
