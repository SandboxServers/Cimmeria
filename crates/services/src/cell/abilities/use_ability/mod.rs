//! The `useAbility(abilityId, targetId)` flow, split along its phases.
//!
//! Submodule layout:
//! - `handle` — the main `handle_use_ability` validate → consume → fire →
//!   resolve flow (incl. the archetype-default weapon redirect and the
//!   auto-cycle arm/clear classification).
//! - `fire_los` — the players-only fire-time line-of-sight gate (NA31).
//! - `auto_reload` — the post-fire auto-reload trigger (`maybe_trigger_auto_reload`).
//! - `kill_credit` — `handle_use_ability_with_kill_credit`, the content-engine
//!   `EntityDeath` wrapper for single-target player-driven casts.
//! - `weapon_redirect` — the read-only archetype-default → active-weapon
//!   RANGED ability redirect resolved at the top of the flow.
//! - `fire` — the post-warmup half of a cast (ammo, `Ability_End`, target
//!   resolution), run at once for a zero warmup or by the warmup tick.
//! - `warmup` — the pending cast between `Ability_Begin` and the fire: the
//!   launch side, the 100 ms tick, and the interrupt (AT-10).
//! - `sequence` — the shared `onSequence` packing for the ability phases.

mod auto_reload;
mod fire;
mod fire_los;
mod handle;
mod kill_credit;
mod sequence;
mod warmup;
mod weapon_redirect;

#[cfg(test)]
mod tests;

// Public re-exports — keep `crate::cell::abilities::use_ability::Foo` paths
// stable for callers (and `super::*` resolution for `tests`).
pub use handle::handle_use_ability;
pub use kill_credit::handle_use_ability_with_kill_credit;

pub(super) use fire::fire_cast;
pub(crate) use kill_credit::credit_ground_deaths;
#[cfg(test)]
pub(crate) use warmup::resolve_warmups;
pub(crate) use warmup::{
    attach_ground_point, interrupt_pending_cast, is_casting, warmup_tick, InterruptReason,
};

pub(crate) use fire_los::{fire_line_of_sight, FireLos};

#[cfg(test)]
pub(crate) use auto_reload::maybe_trigger_auto_reload_for_test;
