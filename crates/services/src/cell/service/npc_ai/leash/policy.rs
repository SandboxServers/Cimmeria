//! Leash policy: when a fighting NPC gives up, and the numbers that shape the
//! walk home. Pure functions over positions and clocks, so each rule has a
//! unit test that needs no `SpaceManager`.
//!
//! The rules (NA12, D-NA03 as corrected by D-NA10, starting values from
//! D-NA09):
//!
//! - The leash is measured on the **NPC's own** horizontal distance from its
//!   spawn, never the target's. The old test was spawn-to-target in 3D, so
//!   the tutorial player's usual spot 49.6-49.95 u from the Cellblock Guard's
//!   spawn bounded every chase, and an aggressive NPC standing at its spawn
//!   leashed every 6 s against a player 60 u away (audit S3, S5).
//! - Hysteresis: between `leash_distance` and `leash_distance + 5` an NPC
//!   keeps fighting a target it can already hit, and gives up only if it
//!   would have to chase further *away* from home. Beyond the band it always
//!   gives up. An NPC at the boundary therefore does not flicker between
//!   fighting and leashing.
//! - A vertical cap stops an NPC that has been drawn two storeys away from
//!   its post, which the horizontal distance cannot see.
//! - A target beyond the NPC's AoI radius for a grace period is lost.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;

/// Width of the hysteresis band above the leash radius, in world units.
pub(in crate::cell::service) const LEASH_HYSTERESIS: f32 = 5.0;

/// Vertical distance from spawn beyond which an NPC leashes whatever its
/// horizontal distance, in world units. A Castle storey is 5-10 u, so 20 u is
/// "at least two floors away from its post". A starting value, tuned at UAT.
pub(in crate::cell::service) const LEASH_VERTICAL_CAP: f32 = 20.0;

/// Horizontal distance from spawn at which a walking NPC counts as home.
pub(in crate::cell::service) const LEASH_ARRIVE_RADIUS: f32 = 1.5;

/// A walk home that takes longer than this is abandoned for a snap. At the
/// default 6 u/s that is 120 u of path, more than twice the leash radius.
pub(in crate::cell::service) const LEASH_WALK_TIMEOUT: Duration = Duration::from_secs(20);

/// How long a target may stay beyond the NPC's AoI radius before the NPC
/// drops it.
pub(in crate::cell::service) const TARGET_LOST_GRACE: Duration = Duration::from_secs(5);

/// After a leash reset, the Idle auto-aggro scan ignores players for this
/// long, so the NPC does not re-aggro on the player it just gave up on.
pub(in crate::cell::service) const REAGGRO_SUPPRESSION: Duration = Duration::from_secs(5);

/// Why a fighting NPC gave up. Enumerated because `trigger` is a SigNoz
/// group-by key on the `npc_ai decision_outcome=leashed` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell::service) enum LeashTrigger {
    /// Horizontal NPC-to-spawn distance beyond `leash_distance + hysteresis`.
    BeyondBand,
    /// Inside the hysteresis band, and the next move would take the NPC
    /// further from home.
    ChaseOutward,
    /// Vertical NPC-to-spawn distance beyond [`LEASH_VERTICAL_CAP`].
    VerticalCap,
}

impl LeashTrigger {
    /// Stable snake_case label. Treat as API.
    pub(in crate::cell::service) fn label(self) -> &'static str {
        match self {
            Self::BeyondBand => "beyond_band",
            Self::ChaseOutward => "chase_outward",
            Self::VerticalCap => "vertical_cap",
        }
    }
}

/// XZ distance, ignoring height. The leash radius is a footprint on the map;
/// height has its own cap.
pub(in crate::cell::service) fn horizontal_distance(a: &Vector3, b: &Vector3) -> f32 {
    let (dx, dz) = (a.x - b.x, a.z - b.z);
    (dx * dx + dz * dz).sqrt()
}

/// The NPC's leash radius: its template's override when set, else the server
/// default.
pub(in crate::cell::service) fn leash_radius(distance_override: Option<f32>) -> f32 {
    distance_override.unwrap_or(crate::cell::combat::LEASH_DISTANCE)
}

/// Whether a fighting NPC at `npc_pos` should give up, and why.
///
/// `wants_to_advance` is true when the NPC is about to move toward its target
/// because it cannot hit it from where it stands (out of range, or no line of
/// sight). A stationary NPC never advances.
pub(in crate::cell::service) fn leash_trigger(
    npc_pos: &Vector3,
    spawn: &Vector3,
    target_pos: &Vector3,
    leash_distance: f32,
    wants_to_advance: bool,
) -> Option<LeashTrigger> {
    if (npc_pos.y - spawn.y).abs() > LEASH_VERTICAL_CAP {
        return Some(LeashTrigger::VerticalCap);
    }
    let npc_to_spawn = horizontal_distance(npc_pos, spawn);
    if npc_to_spawn > leash_distance + LEASH_HYSTERESIS {
        return Some(LeashTrigger::BeyondBand);
    }
    if wants_to_advance
        && npc_to_spawn > leash_distance
        && horizontal_distance(target_pos, spawn) > npc_to_spawn
    {
        return Some(LeashTrigger::ChaseOutward);
    }
    None
}

/// Whether a target at `dist_to_target` from the NPC is out of the NPC's
/// perception, meaning beyond its AoI radius.
pub(in crate::cell::service) fn target_out_of_perception(
    dist_to_target: f32,
    aoi_radius: f32,
) -> bool {
    dist_to_target > aoi_radius
}

/// Whether a target first seen out of perception at `since` has been gone
/// for the whole grace period at `now`.
pub(in crate::cell::service) fn target_lost_for_grace(since: Instant, now: Instant) -> bool {
    now.saturating_duration_since(since) >= TARGET_LOST_GRACE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32, z: f32) -> Vector3 {
        Vector3::new(x, y, z)
    }

    /// The bug shape of audit S3: the tutorial player stands 49.9 u from the
    /// guard's spawn while the guard is still at its spawn. The old
    /// spawn-to-target metric leashed at 50; this one must not leash at all,
    /// whether or not the guard wants to close the distance.
    #[test]
    fn npc_at_spawn_never_leashes_whatever_the_target_distance() {
        let spawn = v(0.0, 0.0, 0.0);
        for target in [v(49.9, 0.0, 0.0), v(60.0, 0.0, 0.0), v(200.0, 3.0, 0.0)] {
            for advance in [false, true] {
                assert_eq!(
                    leash_trigger(&spawn, &spawn, &target, 50.0, advance),
                    None,
                    "target {target:?} advance {advance}"
                );
            }
        }
    }

    #[test]
    fn beyond_the_band_always_leashes() {
        let spawn = v(0.0, 0.0, 0.0);
        let npc = v(55.5, 0.0, 0.0);
        let target = v(40.0, 0.0, 0.0); // back toward home: still leashes
        assert_eq!(
            leash_trigger(&npc, &spawn, &target, 50.0, false),
            Some(LeashTrigger::BeyondBand)
        );
    }

    /// Hysteresis: inside the band an NPC that can hit its target keeps
    /// fighting, and one that must chase further out gives up.
    #[test]
    fn inside_the_band_only_an_outward_chase_leashes() {
        let spawn = v(0.0, 0.0, 0.0);
        let npc = v(52.0, 0.0, 0.0);
        let outward = v(70.0, 0.0, 0.0);
        let inward = v(30.0, 0.0, 0.0);
        assert_eq!(leash_trigger(&npc, &spawn, &outward, 50.0, false), None);
        assert_eq!(
            leash_trigger(&npc, &spawn, &outward, 50.0, true),
            Some(LeashTrigger::ChaseOutward)
        );
        assert_eq!(leash_trigger(&npc, &spawn, &inward, 50.0, true), None);
    }

    /// Height counts only through the cap: 30 u up a ramp at 10 u out is a
    /// leash, 10 u up is not.
    #[test]
    fn vertical_cap_catches_what_the_horizontal_radius_cannot() {
        let spawn = v(0.0, 0.0, 0.0);
        let target = v(12.0, 30.0, 0.0);
        assert_eq!(
            leash_trigger(&v(10.0, 30.0, 0.0), &spawn, &target, 50.0, false),
            Some(LeashTrigger::VerticalCap)
        );
        assert_eq!(
            leash_trigger(&v(10.0, 10.0, 0.0), &spawn, &target, 50.0, false),
            None
        );
    }

    #[test]
    fn template_override_replaces_the_default_radius() {
        assert_eq!(leash_radius(None), 50.0);
        assert_eq!(leash_radius(Some(80.0)), 80.0);
        let spawn = v(0.0, 0.0, 0.0);
        let npc = v(60.0, 0.0, 0.0);
        assert_eq!(
            leash_trigger(&npc, &spawn, &npc, leash_radius(Some(80.0)), true),
            None
        );
    }

    #[test]
    fn lost_target_needs_the_whole_grace_period() {
        let t0 = Instant::now();
        assert!(target_out_of_perception(100.5, 100.0));
        assert!(!target_out_of_perception(99.0, 100.0));
        assert!(!target_lost_for_grace(t0, t0 + Duration::from_secs(4)));
        assert!(target_lost_for_grace(t0, t0 + TARGET_LOST_GRACE));
    }

    #[test]
    fn trigger_labels_are_stable() {
        assert_eq!(LeashTrigger::BeyondBand.label(), "beyond_band");
        assert_eq!(LeashTrigger::ChaseOutward.label(), "chase_outward");
        assert_eq!(LeashTrigger::VerticalCap.label(), "vertical_cap");
    }
}
