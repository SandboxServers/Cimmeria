//! Unit tests for the chase policy. The tick tests over the real
//! `castle_cellblock.nav` are in `service/tests/npc_ai/path_robustness.rs`.

use cimmeria_common::Vector3;

use super::policy::{chase_goal, goal_moved, stop_distance, COMBINED_RADII};

fn horizontal(a: &Vector3, b: &Vector3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// Revert-proof: returning `min_range` alone stops a melee NPC (min 0) at
/// distance 0, inside its target, which is the 0.35 u park of audit S10.
#[test]
fn the_stop_distance_is_the_min_range_but_never_inside_the_target() {
    assert_eq!(stop_distance(0.0, 3.0), COMBINED_RADII, "melee: min 0");
    assert_eq!(stop_distance(0.0, 30.0), COMBINED_RADII, "ranged: min 0");
    assert_eq!(stop_distance(5.0, 30.0), 5.0, "a sniper stops at its min");
    assert_eq!(
        stop_distance(5.0, 4.0),
        4.0,
        "never beyond max range when the range allows a stop"
    );
    assert_eq!(
        stop_distance(0.0, 0.5),
        COMBINED_RADII,
        "a max range under the radii cannot pull the NPC inside the target"
    );
}

/// The goal sits `stop` short of the target on the NPC's side, at the
/// target's height. Revert-proof: routing to the raw target puts the goal at
/// distance 0.
#[test]
fn the_chase_goal_is_offset_toward_the_npc() {
    let npc = Vector3::new(10.0, 5.0, 0.0);
    let target = Vector3::new(0.0, 2.0, 0.0);
    let g = chase_goal(&npc, &target, 1.0);
    assert!((horizontal(&g, &target) - 1.0).abs() < 1e-5, "{g:?}");
    assert!((g.x - 1.0).abs() < 1e-5 && g.z.abs() < 1e-5, "{g:?}");
    assert_eq!(g.y, target.y);

    // An NPC already inside the stop distance routes back out to it.
    let close = Vector3::new(0.3, 2.0, 0.0);
    let g = chase_goal(&close, &target, 1.0);
    assert!((horizontal(&g, &target) - 1.0).abs() < 1e-5, "{g:?}");

    // Straight above: no bearing, still not inside the target.
    let above = Vector3::new(0.0, 9.0, 0.0);
    let g = chase_goal(&above, &target, 1.0);
    assert!((horizontal(&g, &target) - 1.0).abs() < 1e-5, "{g:?}");
}

/// A player walking 3 u down a ramp and 2 u lower is a new route; the old
/// test (3D distance > 5 from the old endpoint) saw 3.6 u and kept the old
/// route on the upper level. Revert-proof: `planned.distance_to(goal) > 5.0`
/// fails the first assertion.
#[test]
fn a_goal_that_drops_a_level_is_a_repath_even_when_close() {
    let planned = Vector3::new(0.0, 10.0, 0.0);
    let down_the_ramp = Vector3::new(3.0, 8.0, 0.0);
    assert!(planned.distance_to(&down_the_ramp) < 5.0);
    assert!(goal_moved(&planned, &down_the_ramp));

    assert!(!goal_moved(&planned, &Vector3::new(4.0, 10.5, 0.0)));
    assert!(goal_moved(&planned, &Vector3::new(5.5, 10.0, 0.0)));
}
