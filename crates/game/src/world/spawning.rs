use serde::{Deserialize, Serialize};

use cimmeria_common::math::Vector3;

/// A spawn point within a spawn set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnPoint {
    pub point_id: i32,
    pub position: Vector3,
    pub rotation_yaw: f32,
}

/// A set of spawn points that produces entities on a timer.
///
/// Corresponds to the spawn_sets + spawn_points + spawnlist tables.
/// Each spawn set manages a group of spawn points in a world space,
/// periodically creating mob/NPC entities when they die or despawn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSet {
    pub set_id: i32,
    pub space_id: i32,
    pub moniker: String,
    pub points: Vec<SpawnPoint>,
    pub max_alive: u32,
    pub current_alive: u32,
    pub respawn_delay_secs: f32,
    pub pending_respawns: Vec<PendingRespawn>,
}

/// A respawn event waiting to fire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRespawn {
    pub point_id: i32,
    pub remaining_secs: f32,
}

impl SpawnSet {
    /// Create a new spawn set.
    pub fn new(set_id: i32, space_id: i32, moniker: String, max_alive: u32, respawn_delay_secs: f32) -> Self {
        Self {
            set_id,
            space_id,
            moniker,
            points: Vec::new(),
            max_alive,
            current_alive: 0,
            respawn_delay_secs,
            pending_respawns: Vec::new(),
        }
    }

    /// Tick the spawn timer, creating entities whose timers have expired.
    /// Returns the list of spawn point IDs that should produce new entities.
    pub fn tick(&mut self, delta_secs: f32) -> Vec<i32> {
        let mut ready = Vec::new();

        self.pending_respawns.retain_mut(|respawn| {
            respawn.remaining_secs -= delta_secs;
            if respawn.remaining_secs <= 0.0 && self.current_alive < self.max_alive {
                ready.push(respawn.point_id);
                self.current_alive += 1;
                false // remove from pending
            } else {
                true
            }
        });

        ready
    }

    /// Notify the spawn set that an entity from this set has died.
    pub fn on_entity_died(&mut self, point_id: i32) {
        self.current_alive = self.current_alive.saturating_sub(1);
        self.pending_respawns.push(PendingRespawn {
            point_id,
            remaining_secs: self.respawn_delay_secs,
        });
    }

    /// Perform the initial spawn of all points up to max_alive.
    pub fn initial_spawn(&mut self) -> Vec<i32> {
        let mut spawned = Vec::new();
        for point in &self.points {
            if self.current_alive >= self.max_alive {
                break;
            }
            spawned.push(point.point_id);
            self.current_alive += 1;
        }
        spawned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_spawn_respects_max() {
        let mut set = SpawnSet::new(1, 1, "jaffa".to_string(), 2, 30.0);
        set.points = vec![
            SpawnPoint { point_id: 1, position: Vector3::zero(), rotation_yaw: 0.0 },
            SpawnPoint { point_id: 2, position: Vector3::zero(), rotation_yaw: 0.0 },
            SpawnPoint { point_id: 3, position: Vector3::zero(), rotation_yaw: 0.0 },
        ];
        let spawned = set.initial_spawn();
        assert_eq!(spawned.len(), 2);
        assert_eq!(set.current_alive, 2);
    }

    #[test]
    fn death_triggers_respawn_timer() {
        let mut set = SpawnSet::new(1, 1, "jaffa".to_string(), 2, 10.0);
        set.current_alive = 2;
        set.on_entity_died(1);
        assert_eq!(set.current_alive, 1);
        assert_eq!(set.pending_respawns.len(), 1);

        // Not ready yet
        let ready = set.tick(5.0);
        assert!(ready.is_empty());

        // Now ready
        let ready = set.tick(6.0);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], 1);
        assert_eq!(set.current_alive, 2);
    }
}
