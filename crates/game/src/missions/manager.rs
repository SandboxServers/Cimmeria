use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::objectives::MissionObjective;
use super::rewards::MissionReward;

/// State of a mission in a player's journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionStatus {
    Active,
    Complete,
    Failed,
    TurnedIn,
}

/// A mission instance being tracked by a player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedMission {
    pub mission_id: i32,
    pub status: MissionStatus,
    pub current_step: u32,
    pub objectives: Vec<MissionObjective>,
    pub rewards: Vec<MissionReward>,
}

/// Tracks all active and completed missions for a player.
///
/// Corresponds to the sgw_mission table and the mission management
/// logic from the Python SGWPlayer class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionTracker {
    player_entity_id: i32,
    missions: HashMap<i32, TrackedMission>,
}

impl MissionTracker {
    /// Create an empty mission tracker for a player.
    pub fn new(player_entity_id: i32) -> Self {
        Self {
            player_entity_id,
            missions: HashMap::new(),
        }
    }

    /// Accept a new mission. Returns `false` if already tracking this mission.
    pub fn accept_mission(&mut self, mission: TrackedMission) -> bool {
        if self.missions.contains_key(&mission.mission_id) {
            return false;
        }
        tracing::info!(
            player = self.player_entity_id,
            mission_id = mission.mission_id,
            "Mission accepted"
        );
        self.missions.insert(mission.mission_id, mission);
        true
    }

    /// Abandon an active mission. Returns `false` if not found.
    pub fn abandon_mission(&mut self, mission_id: i32) -> bool {
        if self.missions.remove(&mission_id).is_some() {
            tracing::info!(
                player = self.player_entity_id,
                mission_id = mission_id,
                "Mission abandoned"
            );
            true
        } else {
            false
        }
    }

    /// Get a reference to a tracked mission.
    pub fn get_mission(&self, mission_id: i32) -> Option<&TrackedMission> {
        self.missions.get(&mission_id)
    }

    /// Get a mutable reference to a tracked mission.
    pub fn get_mission_mut(&mut self, mission_id: i32) -> Option<&mut TrackedMission> {
        self.missions.get_mut(&mission_id)
    }

    /// Check if all objectives in a mission's current step are complete.
    pub fn is_step_complete(&self, mission_id: i32) -> bool {
        self.missions
            .get(&mission_id)
            .map(|m| m.objectives.iter().all(|o| o.is_complete()))
            .unwrap_or(false)
    }

    /// Return all active mission IDs.
    pub fn active_mission_ids(&self) -> Vec<i32> {
        self.missions
            .iter()
            .filter(|(_, m)| m.status == MissionStatus::Active)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Persist the tracker state to the database.
    pub fn save(&self) {
        todo!("MissionTracker::save - write to sgw_mission")
    }

    /// Load a player's mission tracker from the database.
    pub fn load(_player_entity_id: i32) -> Self {
        todo!("MissionTracker::load - read from sgw_mission")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_mission(id: i32) -> TrackedMission {
        TrackedMission {
            mission_id: id,
            status: MissionStatus::Active,
            current_step: 0,
            objectives: vec![MissionObjective::VisitRegion {
                region_id: 1,
                visited: true,
            }],
            rewards: vec![MissionReward::xp_only(1, 100)],
        }
    }

    #[test]
    fn accept_and_query_mission() {
        let mut tracker = MissionTracker::new(1);
        assert!(tracker.accept_mission(test_mission(10)));
        assert!(tracker.get_mission(10).is_some());
    }

    #[test]
    fn reject_duplicate_mission() {
        let mut tracker = MissionTracker::new(1);
        tracker.accept_mission(test_mission(10));
        assert!(!tracker.accept_mission(test_mission(10)));
    }

    #[test]
    fn abandon_mission() {
        let mut tracker = MissionTracker::new(1);
        tracker.accept_mission(test_mission(10));
        assert!(tracker.abandon_mission(10));
        assert!(tracker.get_mission(10).is_none());
    }

    #[test]
    fn step_complete_check() {
        let mut tracker = MissionTracker::new(1);
        tracker.accept_mission(test_mission(10));
        assert!(tracker.is_step_complete(10));
    }
}
