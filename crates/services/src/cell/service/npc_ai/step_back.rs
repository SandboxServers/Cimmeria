//! Ranged step-back (NA32, D-NA15): a ranged NPC that is attacking in place
//! and finds its target inside its comfort range walks back along the
//! navmesh to regain range.
//!
//! This grows the old min-range backup (issue #329), which only fired
//! inside an ability's `min_range` and so never for the seeded guards,
//! whose abilities all carry `min_range = 0`. The rule:
//!
//! - **Who.** A mobile NPC that is not holding a cover slot, whose chosen
//!   ability has a `min_range`, or whose abilities are all ranged
//!   ([`super::ability_select::npc_is_ranged_only`]). A melee NPC never
//!   steps back, and neither does a staff Jaffa with a melee swing in its
//!   set: at close range it swings instead. Stationary NPCs are pinned. An
//!   NPC in cover keeps its slot (NA22/NA23); a flanked one has already
//!   given the slot up, and then steps back like any other.
//! - **When.** The target is closer than the comfort range,
//!   `max(min_range, 2 u)` ([`comfort_range`]).
//! - **Where.** Straight away from the target, horizontally, to
//!   [`retreat_distance`] (the comfort range plus 3 u), slid across the
//!   navmesh by `moveAlongSurface` so a wall or ledge stops it
//!   ([`super::ability_select::step_back_waypoint_on_mesh`]). The 3 u gap is
//!   the hysteresis: a step-back ends well outside the range that starts
//!   one.
//! - **How often.** At most once per [`STEP_BACK_COOLDOWN`] (3 s). A player
//!   who follows the NPC gets shot at from close range between steps rather
//!   than chasing it across the room. Inside a hard `min_range` during the
//!   cooldown the NPC cannot fire, so it holds and faces its target.
//! - **Cornered.** A slide that gains less than [`STEP_BACK_MIN_GAIN`] (the
//!   NPC has its back to a wall) is not taken; the NPC fires from where it
//!   is, or holds inside a hard `min_range`, and the cooldown starts anyway
//!   so it does not re-plan the same blocked step every tick.
//! - **Mid-step.** While the cooldown runs and the NPC is still walking its
//!   step-back route, the fight tick leaves it walking
//!   (`step_back_walking`); the attack arm would otherwise clear the route.
//!
//! The chase stop distance is unchanged (NA15): a chase only runs while the
//! target is out of range or out of sight, so it rarely ends inside 2 u,
//! and when it does the next tick steps back once and the cooldown holds.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;

use crate::cell::space_manager::SpaceManager;

use super::ability_select::step_back_waypoint_on_mesh;
use super::leash::policy::horizontal_distance;

/// The closest a ranged NPC with no `min_range` lets a target come before it
/// steps back, in world units. Twice the combined radii the chase stops at
/// (`chase::policy::COMBINED_RADII`): a target that close is in the NPC's
/// face.
pub(in crate::cell::service) const STEP_BACK_COMFORT: f32 = 2.0;

/// How far past the comfort range a step-back goes. The hysteresis band.
pub(in crate::cell::service) const STEP_BACK_MARGIN: f32 = 3.0;

/// The least time between two step-backs of one NPC.
pub(in crate::cell::service) const STEP_BACK_COOLDOWN: Duration = Duration::from_secs(3);

/// A step that gains less distance from the target than this is not worth
/// walking: the NPC is cornered.
pub(in crate::cell::service) const STEP_BACK_MIN_GAIN: f32 = 0.5;

/// The comfort range for an ability with `min_range`.
pub(in crate::cell::service) fn comfort_range(min_range: f32) -> f32 {
    min_range.max(STEP_BACK_COMFORT)
}

/// How far from the target a step-back ends, horizontally.
pub(in crate::cell::service) fn retreat_distance(min_range: f32) -> f32 {
    comfort_range(min_range) + STEP_BACK_MARGIN
}

/// Whether an NPC with this ability `min_range` steps back at all.
pub(in crate::cell::service) fn steps_back(min_range: f32, ranged_only: bool) -> bool {
    min_range > 0.0 || ranged_only
}

/// The policy verdict before any geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell::service) enum StepBackCall {
    /// Not a stepping NPC, or the target is outside the comfort range.
    Fire,
    /// The cooldown is running. `dead_zone` says the target is inside the
    /// ability's hard `min_range`, so the NPC cannot fire either.
    Cooling { dead_zone: bool },
    /// Step back.
    Step,
}

/// Pure step-back decision: see the module docs.
pub(in crate::cell::service) fn decide(
    steps: bool,
    dist_to_target: f32,
    min_range: f32,
    last_step: Option<Instant>,
    now: Instant,
) -> StepBackCall {
    if !steps || dist_to_target >= comfort_range(min_range) {
        return StepBackCall::Fire;
    }
    if last_step.is_some_and(|t| now.saturating_duration_since(t) < STEP_BACK_COOLDOWN) {
        return StepBackCall::Cooling {
            dead_zone: min_range > 0.0 && dist_to_target < min_range,
        };
    }
    StepBackCall::Step
}

/// What the fight tick does after the step-back check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell::service) enum StepBackOutcome {
    /// Go on and attack in place.
    Fire,
    /// A step-back waypoint was installed: this tick is done.
    Stepped,
    /// Inside a hard `min_range` with no step to take: hold fire.
    Hold,
}

/// One fight tick's inputs to [`step_back`].
pub(in crate::cell::service) struct StepBackStep {
    pub npc_id: u32,
    pub target_id: u32,
    pub npc_pos: Vector3,
    pub target_pos: Vector3,
    pub dist_to_target: f32,
    pub min_range: f32,
    pub ranged_only: bool,
    pub ability_id: Option<i32>,
}

/// Run the step-back rule for one fight tick. Installs the waypoint and
/// records the time when it steps; logs the decision either way.
pub(in crate::cell::service) fn step_back(
    space_mgr: &mut SpaceManager,
    s: StepBackStep,
    now: Instant,
) -> StepBackOutcome {
    let steps = steps_back(s.min_range, s.ranged_only);
    let last = space_mgr
        .get_entity(s.npc_id)
        .and_then(|n| n.leash.step_back_at);
    let dead_zone = s.min_range > 0.0 && s.dist_to_target < s.min_range;
    let call = decide(steps, s.dist_to_target, s.min_range, last, now);
    // A step-back still being walked is left to finish: stopping to fire
    // (the attack arm clears the path) would cut it short every tick.
    if matches!(call, StepBackCall::Cooling { .. }) && step_back_in_flight(space_mgr, s.npc_id) {
        super::note_outcome("step_back_walking");
        return StepBackOutcome::Stepped;
    }
    match call {
        StepBackCall::Fire | StepBackCall::Cooling { dead_zone: false } => {
            return StepBackOutcome::Fire
        }
        StepBackCall::Cooling { dead_zone: true } => {
            super::note_outcome("step_back_cooling");
            log(&s, "step_back_cooling", None);
            return StepBackOutcome::Hold;
        }
        StepBackCall::Step => {}
    }
    let waypoint = step_back_waypoint_on_mesh(
        space_mgr,
        s.npc_id,
        s.npc_pos,
        s.target_pos,
        retreat_distance(s.min_range),
    );
    if let Some(npc) = space_mgr.get_entity_mut(s.npc_id) {
        npc.leash.step_back_at = Some(now);
    }
    let gain = waypoint.map_or(0.0, |w| {
        horizontal_distance(&w, &s.target_pos) - horizontal_distance(&s.npc_pos, &s.target_pos)
    });
    let Some(waypoint) = waypoint.filter(|_| gain >= STEP_BACK_MIN_GAIN) else {
        super::note_outcome("step_back_cornered");
        log(&s, "step_back_cornered", None);
        return if dead_zone {
            StepBackOutcome::Hold
        } else {
            StepBackOutcome::Fire
        };
    };
    if let Some(npc) = space_mgr.get_entity_mut(s.npc_id) {
        super::replace_nav_path_on(npc, [waypoint]);
    }
    space_mgr
        .npc_detectors
        .note_move_source(s.npc_id, super::detectors::MoveSource::Backup);
    // Inside a hard min_range this is the old min-range backup; keep its
    // outcome label so existing SigNoz queries still find it.
    let outcome = if dead_zone {
        "min_range_backup"
    } else {
        "step_back"
    };
    super::note_outcome(outcome);
    log(&s, outcome, Some(waypoint));
    StepBackOutcome::Stepped
}

/// Whether the NPC is still walking the step-back route it was given.
fn step_back_in_flight(space_mgr: &SpaceManager, npc_id: u32) -> bool {
    space_mgr
        .get_entity(npc_id)
        .is_some_and(|n| !n.nav_path.is_empty())
        && space_mgr.npc_detectors.move_source(npc_id) == Some(super::detectors::MoveSource::Backup)
}

fn log(s: &StepBackStep, outcome: &'static str, waypoint: Option<Vector3>) {
    let w = waypoint.unwrap_or(s.npc_pos);
    tracing::debug!(
        target: "npc_ai",
        event = "decision",
        decision_outcome = outcome,
        npc_id = s.npc_id,
        target_id = s.target_id,
        ability_id = s.ability_id,
        dist_to_target = s.dist_to_target,
        min_range = s.min_range,
        comfort_range = comfort_range(s.min_range),
        retreat_distance = retreat_distance(s.min_range),
        stepped = waypoint.is_some(),
        backup_x = w.x,
        backup_y = w.y,
        backup_z = w.z,
        "NPC AI: ranged NPC with its target inside its comfort range"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comfort_and_retreat_ranges() {
        assert_eq!(comfort_range(0.0), 2.0, "no min_range: the 2 u floor");
        assert_eq!(comfort_range(5.0), 5.0, "a sniper keeps its own");
        assert_eq!(retreat_distance(0.0), 5.0);
        assert_eq!(retreat_distance(5.0), 8.0);
    }

    #[test]
    fn only_ranged_npcs_step_back() {
        assert!(steps_back(0.0, true), "all-ranged set");
        assert!(steps_back(5.0, false), "an ability with a min_range");
        assert!(!steps_back(0.0, false), "melee or mixed set");
        let now = Instant::now();
        assert_eq!(decide(false, 1.0, 0.0, None, now), StepBackCall::Fire);
    }

    #[test]
    fn step_inside_comfort_fire_outside() {
        let now = Instant::now();
        assert_eq!(decide(true, 1.5, 0.0, None, now), StepBackCall::Step);
        assert_eq!(decide(true, 2.0, 0.0, None, now), StepBackCall::Fire);
        assert_eq!(decide(true, 4.0, 5.0, None, now), StepBackCall::Step);
    }

    #[test]
    fn cooldown_gates_the_next_step() {
        let t0 = Instant::now();
        let soon = t0 + Duration::from_millis(2_900);
        let later = t0 + STEP_BACK_COOLDOWN;
        assert_eq!(
            decide(true, 1.0, 0.0, Some(t0), soon),
            StepBackCall::Cooling { dead_zone: false }
        );
        assert_eq!(
            decide(true, 3.0, 5.0, Some(t0), soon),
            StepBackCall::Cooling { dead_zone: true }
        );
        assert_eq!(decide(true, 1.0, 0.0, Some(t0), later), StepBackCall::Step);
    }
}
