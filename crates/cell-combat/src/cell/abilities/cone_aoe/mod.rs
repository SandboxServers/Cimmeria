//! Cone-shaped AoE target collection.
//!
//! 99 effects in the DB carry `target_collection_method = 'TCM_AECone'`,
//! but until this module landed they only damaged the primary target —
//! shotgun blasts, grenade arcs, and "secondary target" splash all
//! resolved as plain single-target hits. This implements the cone
//! collection + secondary fan-out.
//!
//! ## Submodule layout
//!
//! - `geometry` — `is_cone_effect` + `collect_cone_targets` (the planar
//!   cone-containment scan).
//! - `flag_categories` — `log_effect_flag_categories` (effect-flag bitmask
//!   observability).
//! - `fan_out` — `fan_out_cone_effects` (per-effect secondary dispatch).
//!
//! ## Cone geometry
//!
//! The cone is anchored at the **attacker's position**, oriented toward
//! the **primary target's position**. Length comes from `tcm_param1`
//! (range tier name → meters via `EffectDef::tcm_range_meters`), width
//! comes from `tcm_param2` (Narrow/Medium/Wide → half-angle via
//! `EffectDef::tcm_half_angle_radians`).
//!
//! An entity `E` is inside the cone when:
//!   1. `distance(attacker, E) <= cone_length`
//!   2. `angle_between(target - attacker, E - attacker) <= half_angle`
//!   3. `E` is not the primary target (it already took damage upstream)
//!   4. `E` is in the attacker's space (no cross-space leak)
//!   5. `E` is alive and the attacker may hit it (`combat::may_hit_in_area`)
//!
//! ## Players: the duel partner only
//!
//! `collect_cone_targets` scans `combat::area_candidates`: every NPC, plus
//! the attacker's engaged duel partner (SS-D2). No other player is ever a
//! candidate, and each candidate must pass `combat::may_hit_in_area`, which
//! for a player caster is `player_may_attack`, the same rule as the
//! single-target launch.
//!
//! ## Why dispatch lives here, not in `apply_damage_to_target`
//!
//! `apply_damage_to_target` is reused by AoE-secondary callers — if we
//! put cone fan-out inside it, every secondary call would recursively
//! fan out to its own cone, causing exponential blowup. The fan-out
//! happens **above** `apply_damage_to_target` (called once per ability
//! invocation from `use_ability` after the primary commits).
//!
//! ## What's NOT here
//!
//! - **Radius (TCM_AERadius) fan-out.** That's already covered for the
//!   ground-target path by `handle_use_ability_on_ground`. The 300
//!   radius effects on single-target abilities (e.g. proximity-mine
//!   detonations) are a follow-up — they need explicit detonation
//!   triggers, not a fan-out at primary cast.
//! - **Cone for ground-target abilities.** A ground-target ability with
//!   a TCM_AECone effect would have no primary entity to orient toward.
//!   When that case shows up, anchor the cone at the ground click with
//!   the attacker's facing direction instead.

mod fan_out;
mod flag_categories;
mod geometry;

#[cfg(test)]
mod tests;

// Public re-exports — keep `crate::cell::abilities::cone_aoe::Foo` paths
// stable for callers (and `super::*` resolution for `tests`).
pub use fan_out::fan_out_cone_effects;
pub use flag_categories::log_effect_flag_categories;
pub(crate) use flag_categories::EFFECT_FLAGS;
pub use geometry::collect_cone_targets;
// `is_cone_effect` is reached internally via `geometry::` and otherwise only
// by the test module through `super::*`; gate the re-export so the non-test
// build doesn't flag it unused.
#[cfg(test)]
use geometry::is_cone_effect;
