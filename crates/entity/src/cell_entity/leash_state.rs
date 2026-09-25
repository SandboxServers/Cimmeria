//! Per-NPC leash and reset bookkeeping (NA12, D-NA03).
//!
//! Grouped in one struct so the leash policy's scratch state lives in one
//! place on [`super::CellEntity`] rather than as four loose fields. Every
//! field is server-side runtime state: never persisted, never on the wire.

use std::time::Instant;

use cimmeria_common::Vector3;

/// Leash radius, walk-home timer, lost-target timer and the post-reset
/// re-aggro window for one NPC, plus the chase bookkeeping that decides when
/// an unreachable target makes it give up (NA15).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LeashState {
    /// Per-template leash radius in world units, from
    /// `entity_templates.leash_distance`. `None` means the template does not
    /// set one, and the server default applies
    /// (`cell::combat::LEASH_DISTANCE`, 50 u).
    pub distance_override: Option<f32>,
    /// When the NPC started walking home. Set on entering `Leashing` and
    /// cleared on arrival. The leash tick snaps the NPC home when the walk
    /// takes longer than its timeout.
    pub walk_started_at: Option<Instant>,
    /// When the NPC first found its top-threat target beyond its AoI radius.
    /// Cleared as soon as the target is back in range. Once this has been set
    /// for the lost-target grace period, the NPC drops the target.
    pub target_lost_since: Option<Instant>,
    /// Until this instant, the Idle auto-aggro scan ignores players. Set on
    /// leash arrival so a reset NPC does not re-aggro on the same player in
    /// the same breath (the aggro/leash loop, audit S5).
    pub reaggro_suppressed_until: Option<Instant>,
    /// What the installed chase route was planned toward (NA15). Lets the
    /// fight handler tell "still walking the route for this goal" and "at the
    /// end of a route that cannot reach the target" from "the target moved,
    /// plan again", instead of repathing every tick into a near-zero route
    /// at a mesh island's edge (audit S8).
    pub chase_route: Option<ChaseRoute>,
    /// When the NPC started holding at the end of a route that cannot reach
    /// its target. After the grace period the NPC gives up and walks home.
    /// Cleared whenever a route reaches the target, or the NPC can attack.
    pub unreachable_since: Option<Instant>,
    /// The installed route home is partial: spawn is on another mesh island.
    /// The leash tick walks it to its end and then snaps home, rather than
    /// replanning from the island edge until the walk timeout.
    pub home_route_partial: bool,
    /// When a ranged NPC last stepped back from a target inside its comfort
    /// range (NA32). The next step-back waits out a cooldown from here, so
    /// a player who follows it cannot make it kite forever.
    pub step_back_at: Option<Instant>,
}

/// One chase route: where it was planned toward and where it ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseRoute {
    /// The routed goal: the target offset by the stop distance, or a cover
    /// slot.
    pub goal: Vector3,
    /// The route's last waypoint (the NPC's own position for a route that
    /// came back degenerate).
    pub end: Vector3,
    /// Whether the route reaches `goal`. `false` for a partial corridor, a
    /// route to the nearest on-mesh point to an off-mesh goal, and a
    /// degenerate route.
    pub reaches_goal: bool,
}

impl LeashState {
    /// Forget the chase: its route record and its unreachable timer. Called
    /// on every AI state change, so a fight that resumes later starts fresh.
    pub fn clear_chase(&mut self) {
        self.chase_route = None;
        self.unreachable_since = None;
    }

    /// Whether the post-reset re-aggro window is still open at `now`.
    pub fn reaggro_suppressed(&self, now: Instant) -> bool {
        self.reaggro_suppressed_until.is_some_and(|t| now < t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn reaggro_window_is_open_only_before_its_deadline() {
        let now = Instant::now();
        let mut s = LeashState::default();
        assert!(!s.reaggro_suppressed(now), "no window by default");
        s.reaggro_suppressed_until = Some(now + Duration::from_secs(5));
        assert!(s.reaggro_suppressed(now));
        assert!(!s.reaggro_suppressed(now + Duration::from_secs(5)));
    }
}
