//! The `useAbility(abilityId, targetId)` flow, split along its phases.
//!
//! Submodule layout:
//! - `handle` — the main `handle_use_ability` validate → consume → fire →
//!   resolve flow (incl. the archetype-default weapon redirect and the
//!   auto-cycle arm/clear classification).
//! - `auto_reload` — the post-fire auto-reload trigger (`maybe_trigger_auto_reload`).
//! - `kill_credit` — `handle_use_ability_with_kill_credit`, the content-engine
//!   `EntityDeath` wrapper for single-target player-driven casts.
//! - `weapon_redirect` — the read-only archetype-default → active-weapon
//!   RANGED ability redirect resolved at the top of the flow.

mod auto_reload;
mod handle;
mod kill_credit;
mod weapon_redirect;

#[cfg(test)]
mod tests;

// Public re-exports — keep `crate::cell::abilities::use_ability::Foo` paths
// stable for callers (and `super::*` resolution for `tests`).
pub use handle::handle_use_ability;
pub use kill_credit::handle_use_ability_with_kill_credit;

#[cfg(test)]
pub(crate) use auto_reload::maybe_trigger_auto_reload_for_test;
