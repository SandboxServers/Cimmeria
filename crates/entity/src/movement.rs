//! Movement controllers for entity locomotion.
//!
//! Movement controllers determine how an entity's position changes over time.
//! Each tick, the active controller's `update` method is called with the
//! elapsed time delta; it returns the new position (or `None` if no movement
//! occurred this tick).
//!
//! The BigWorld engine supports several controller types. This module provides:
//!
//! - [`PlayerMovementController`] -- applies client-authoritative position
//!   updates (with server-side validation).
//! - [`WaypointMovementController`] -- moves an entity along a sequence of
//!   waypoints at a fixed speed (used for NPC patrols and scripted paths).

use cimmeria_common::Vector3;

/// Trait for entity movement strategies.
///
/// Implementors produce position updates each tick. The entity system calls
/// `update(dt)` on the active controller and applies the returned position
/// (if any) to the cell entity.
pub trait MovementController: Send + Sync {
    /// Advance the controller by `dt` seconds and return the new position,
    /// or `None` if the entity did not move this tick.
    fn update(&mut self, dt: f32) -> Option<Vector3>;

    /// Returns `true` if the controller has finished its movement (e.g., a
    /// waypoint path has reached the end with no looping).
    fn is_complete(&self) -> bool;
}

/// Player movement controller -- applies client-authoritative position updates.
///
/// The client sends position packets that the server validates (speed checks,
/// navmesh containment) and applies. Between client updates, the controller
/// may extrapolate or hold the last known position.
pub struct PlayerMovementController {
    /// The most recently validated position.
    current_position: Vector3,

    /// Whether the controller has received at least one client update.
    has_update: bool,
}

impl PlayerMovementController {
    /// Create a new player movement controller at the given starting position.
    pub fn new(start_position: Vector3) -> Self {
        Self {
            current_position: start_position,
            has_update: false,
        }
    }

    /// Apply a client-authoritative position update.
    ///
    /// In a full implementation this would perform speed validation and
    /// navmesh checks before accepting the position.
    pub fn apply_client_update(&mut self, position: Vector3) {
        // TODO: Validate speed, navmesh containment, anti-cheat checks.
        self.current_position = position;
        self.has_update = true;
    }
}

impl MovementController for PlayerMovementController {
    fn update(&mut self, _dt: f32) -> Option<Vector3> {
        if self.has_update {
            self.has_update = false;
            Some(self.current_position)
        } else {
            None
        }
    }

    fn is_complete(&self) -> bool {
        // Player controllers never "complete" -- they persist for the
        // lifetime of the player entity.
        false
    }
}

/// Waypoint movement controller -- moves along a path at a fixed speed.
///
/// Used for NPC patrol routes, scripted entity movement, and any
/// predetermined path traversal. When the final waypoint is reached,
/// `is_complete()` returns `true`.
pub struct WaypointMovementController {
    /// Ordered list of waypoints to visit.
    waypoints: Vec<Vector3>,

    /// Movement speed in world units per second.
    speed: f32,

    /// Index of the waypoint currently being moved toward.
    current_index: usize,

    /// Current interpolated position along the path.
    current_position: Vector3,
}

impl WaypointMovementController {
    /// Create a new waypoint controller with the given path and speed.
    ///
    /// The entity starts at the first waypoint and moves toward subsequent
    /// ones at `speed` world units per second.
    ///
    /// # Panics
    ///
    /// Panics if `waypoints` is empty.
    pub fn new(waypoints: Vec<Vector3>, speed: f32) -> Self {
        assert!(!waypoints.is_empty(), "waypoints must not be empty");
        let start = waypoints[0];
        Self {
            waypoints,
            speed,
            current_index: 1, // Start moving toward the second waypoint.
            current_position: start,
        }
    }

    /// Return the current interpolated position.
    pub fn position(&self) -> Vector3 {
        self.current_position
    }
}

impl MovementController for WaypointMovementController {
    fn update(&mut self, dt: f32) -> Option<Vector3> {
        if self.current_index >= self.waypoints.len() {
            return None; // Already at the end.
        }

        let target = self.waypoints[self.current_index];
        let to_target = target - self.current_position;
        let distance_to_target = to_target.length();
        let step = self.speed * dt;

        if step >= distance_to_target {
            // Reached (or passed) the current waypoint.
            self.current_position = target;
            self.current_index += 1;

            // If there are more waypoints and we have leftover distance,
            // we could recurse, but for simplicity we stop at the waypoint
            // and pick up the next segment on the following tick.
        } else {
            // Move toward the target.
            let direction = to_target.normalized();
            self.current_position = self.current_position + direction * step;
        }

        Some(self.current_position)
    }

    fn is_complete(&self) -> bool {
        self.current_index >= self.waypoints.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_controller_no_update_returns_none() {
        let mut ctrl = PlayerMovementController::new(Vector3::zero());
        assert!(ctrl.update(0.016).is_none());
    }

    #[test]
    fn player_controller_apply_and_update() {
        let mut ctrl = PlayerMovementController::new(Vector3::zero());
        ctrl.apply_client_update(Vector3::new(10.0, 0.0, 20.0));
        let pos = ctrl.update(0.016);
        assert_eq!(pos, Some(Vector3::new(10.0, 0.0, 20.0)));
    }

    #[test]
    fn player_controller_update_consumed() {
        let mut ctrl = PlayerMovementController::new(Vector3::zero());
        ctrl.apply_client_update(Vector3::new(5.0, 0.0, 5.0));
        ctrl.update(0.016); // consumes the update
        assert!(ctrl.update(0.016).is_none()); // no new update
    }

    #[test]
    fn player_controller_never_completes() {
        let ctrl = PlayerMovementController::new(Vector3::zero());
        assert!(!ctrl.is_complete());
    }

    #[test]
    fn waypoint_controller_moves_toward_target() {
        let waypoints = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
        ];
        let mut ctrl = WaypointMovementController::new(waypoints, 5.0);

        // After 1 second at speed 5, should be at (5, 0, 0).
        let pos = ctrl.update(1.0).unwrap();
        assert!((pos.x - 5.0).abs() < 0.01);
        assert!(!ctrl.is_complete());
    }

    #[test]
    fn waypoint_controller_reaches_end() {
        let waypoints = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
        ];
        let mut ctrl = WaypointMovementController::new(waypoints, 100.0);

        // Speed 100, distance 10 -- should reach in one tick.
        ctrl.update(1.0);
        assert!(ctrl.is_complete());
    }

    #[test]
    fn waypoint_controller_multiple_waypoints() {
        let waypoints = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 10.0),
        ];
        let mut ctrl = WaypointMovementController::new(waypoints, 100.0);

        // First tick: reach waypoint 1.
        ctrl.update(1.0);
        assert!(!ctrl.is_complete());

        // Second tick: reach waypoint 2.
        ctrl.update(1.0);
        assert!(ctrl.is_complete());
    }

    #[test]
    fn waypoint_controller_returns_none_when_complete() {
        let waypoints = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
        ];
        let mut ctrl = WaypointMovementController::new(waypoints, 100.0);
        ctrl.update(1.0); // reach the end
        assert!(ctrl.update(1.0).is_none()); // no more movement
    }

    #[test]
    #[should_panic(expected = "waypoints must not be empty")]
    fn waypoint_controller_empty_path_panics() {
        WaypointMovementController::new(vec![], 5.0);
    }

    #[test]
    fn waypoint_controller_position() {
        let waypoints = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
        ];
        let ctrl = WaypointMovementController::new(waypoints, 5.0);
        assert_eq!(ctrl.position(), Vector3::new(0.0, 0.0, 0.0));
    }
}
