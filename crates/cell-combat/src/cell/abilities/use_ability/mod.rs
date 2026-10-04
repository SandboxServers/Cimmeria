//! The `useAbility(abilityId, targetId)` flow, split along its phases.
//!
//! Submodule layout:
//! - `handle` — the main `handle_use_ability` validate → consume → fire →
//!   resolve flow (incl. the archetype-default weapon redirect and the
//!   auto-cycle arm/clear classification).
//! - `beneficial` — who a player's cast lands on (AB-01): the #444 target
//!   gate, `resolve_cast_target` (Self casts on the caster, heals on allies,
//!   the D-AB02 fallback) and the damage-free `fire_beneficial`.
//! - `cast_range` — the range gate shared by the launch and the warmup
//!   fire: the maximum for every caster, the `min_range` for players
//!   (#1016), and the `onErrorCode` 42 refusal.
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
//! - `not_known` — the `onErrorCode` 167 answer to a press of an ability
//!   the player does not know.
//! - `no_mechanics` — `ability_has_mechanics` and the AB-12 refusal of a
//!   press that cannot do anything: `onErrorCode`, a feedback line, no
//!   cooldown (D-AB10).
//! - `weapon_gate` — the weapon-attack launch gates: the holstered-draw
//!   queue, one queued shot at a time, the slot-swap lockout.
//! - `summon` — the pet-summon diversions (pets PT-03): the launch refusals,
//!   and the fire that spawns the pet instead of resolving a target.
//! - `owner_pet` — owner abilities that act on the owner's pet (pets PT-08):
//!   the same launch/fire diversions, and the tick that expires pet buffs
//!   and carries out To The Death.
//! - `support_shot` — beneficial ammo (AM-11d): the ally/self admission and
//!   hostile refusal at launch and fire, and the damage-free resolve that
//!   runs only the ammo's on-hit effect.
//! - `sequence` — the Ability_Begin / Ability_End / Ability_Interrupt
//!   `onSequence`: shared packing, owner + witnesses routing, and the NPC
//!   attack-animation WARNs (NA43).

mod auto_reload;
mod beneficial;
mod cast_range;
mod fire;
mod fire_los;
mod handle;
mod kill_credit;
mod no_mechanics;
mod not_known;
mod owner_pet;
mod sequence;
mod summon;
mod support_shot;
mod warmup;
mod weapon_gate;
mod weapon_redirect;

#[cfg(test)]
mod tests;

// Public re-exports — keep `crate::cell::abilities::use_ability::Foo` paths
// stable for callers (and `super::*` resolution for `tests`).
pub use handle::handle_use_ability;
pub use kill_credit::handle_use_ability_with_kill_credit;
pub use owner_pet::{is_owner_pet_ability, owner_pet_tick, owner_pet_tick_at};

pub(super) use fire::fire_cast;
pub use kill_credit::credit_ground_deaths;
pub(crate) use kill_credit::credited_player;
pub(super) use sequence::{play_ability_sequence, AbilityPhase, PhaseSequence};
#[cfg(test)]
pub(crate) use warmup::resolve_warmups;
pub(crate) use warmup::{attach_ground_point, interrupt_pending_cast, is_casting, InterruptReason};
pub use warmup::{interrupt_unlearned_cast, warmup_tick};

pub use fire_los::{fire_line_of_sight, FireLos};

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use auto_reload::maybe_trigger_auto_reload_for_test;
