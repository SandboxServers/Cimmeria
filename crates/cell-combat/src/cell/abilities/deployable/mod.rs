//! Deployables (Phase 0): the cast and the pulse. Design and evidence:
//! `docs/gameplay/deployables.md`; decision 28 of
//! `docs/architecture/abilities-and-effects-system.md`.
//!
//! A player ability with a `resources.deployables` row
//! (`SpaceManager::deployable_specs`) places a stationary object at a ground
//! point instead of hitting a target. It rides the ordinary cast of
//! decision 21 with these diversions:
//!
//! 1. **Ground point** ([`launch::handle_deploy_on_ground`], from
//!    `useAbilityOnGroundTarget`). The client's point is validated before
//!    anything is charged: finite, within the ability's `max_range` of the
//!    caster, in line of sight of the caster's eye where the world has an
//!    occluder, and on the navmesh where the world enforces containment
//!    (snapped to the floor where a mesh covers it). A refusal sends
//!    `onErrorCode` and a `CHAN_FEEDBACK` line and logs `deploy_refused`
//!    with its `reason`. So does a press during the cooldown or another
//!    warmup, which the ordinary launch refuses silently. The point is then
//!    staged on the registry and the cast launched with target 0.
//! 2. **Launch** (`handle.rs`). The client's target is discarded, and a
//!    deployable launched without a staged point (a plain `useAbility`
//!    naming it) is refused with feedback.
//! 3. **Warmup.** The ability's own (1012: 2 s, shortened by `speedDeploy`
//!    through the `SpeedDeploy` flag). Every decision-21 interrupt applies
//!    and drops the staged point.
//! 4. **Fire** ([`fire::fire_deploy`], from `fire::fire_cast` ahead of the
//!    damage pipeline). Re-check, place the object, play `Ability_End`, and
//!    remove the owner's oldest one past `max_active` (1: a re-cast replaces
//!    the object).
//! 5. **Pulse** ([`tick::deployable_tick`], every AoI tick). The world-side
//!    verdict removes a deployable whose owner died, left or changed space,
//!    or whose last pulse ran. Otherwise, each interval it applies the pulse
//!    effect to every target the owner may hit in its radius, through
//!    `apply_damage_to_target` with the owner as the attacker: the owner
//!    gets the threat, the kill XP and the mission credit, and the owner's
//!    hostility rule (`may_hit_in_area`) decides the targets, so players,
//!    pets and friendly NPCs are never hit (an engaged duel partner is, as
//!    for any area ability of the owner's).
//!
//! NPC casters never deploy: [`player_deployable`] answers only for
//! players.

mod feedback;
mod fire;
mod launch;
mod tick;

#[cfg(test)]
mod tests;

use cimmeria_cell_catalog::cell::spawner::DeployableSpec;

use super::super::space_manager::SpaceManager;

pub(in crate::cell::abilities) use fire::fire_deploy;
pub(in crate::cell::abilities) use launch::{handle_deploy_on_ground, refuse_unstaged_launch};
pub use tick::{deployable_tick, deployable_tick_at};

/// The deployable row for `ability_id` when `entity_id` is a player, else
/// `None`. The one question every deployable diversion asks.
pub(in crate::cell::abilities) fn player_deployable(
    space_mgr: &SpaceManager,
    entity_id: u32,
    ability_id: i32,
) -> Option<DeployableSpec> {
    let spec = space_mgr.deployable_specs.deployable_for(ability_id)?;
    space_mgr
        .get_entity(entity_id)
        .filter(|e| e.is_player)
        .map(|_| spec)
}

/// The `TargetID` a deployable cast's phase sequences carry: the caster, as
/// python sent `targetId or ent.entityId` for an untargeted cast. Anything
/// else keeps the target it was given.
pub(in crate::cell::abilities) fn phase_sequence_target(
    space_mgr: &SpaceManager,
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
) -> i32 {
    if target_id <= 0 && player_deployable(space_mgr, entity_id, ability_id).is_some() {
        entity_id as i32
    } else {
        target_id
    }
}
