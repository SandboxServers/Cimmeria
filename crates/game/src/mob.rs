use serde::{Deserialize, Serialize};

use cimmeria_common::math::Vector3;

use crate::being::BeingStats;

/// Runtime state for an SGWMob entity (hostile creature).
///
/// Mobs are spawned by the world spawning system, follow patrol paths,
/// aggro on nearby players, and drop loot on death.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobState {
    pub spawn_id: i32,
    pub stats: BeingStats,
    pub loot_table_id: Option<i32>,
    pub aggro_radius: f32,
    pub patrol_path: Vec<Vector3>,
    pub respawn_time_secs: u32,
    pub current_target: Option<i32>,
}

impl MobState {
    /// Create a new mob with the given spawn configuration.
    pub fn new(spawn_id: i32, stats: BeingStats, aggro_radius: f32, respawn_time_secs: u32) -> Self {
        Self {
            spawn_id,
            stats,
            loot_table_id: None,
            aggro_radius,
            patrol_path: Vec::new(),
            respawn_time_secs,
            current_target: None,
        }
    }

    /// Check if a position is within aggro range of this mob's position.
    pub fn is_in_aggro_range(&self, mob_pos: &Vector3, target_pos: &Vector3) -> bool {
        mob_pos.distance_squared_to(target_pos) <= self.aggro_radius * self.aggro_radius
    }

    /// Advance the mob's AI by one tick. Placeholder for the behavior tree.
    pub fn tick_ai(&mut self, _mob_pos: &Vector3, _delta_secs: f32) {
        todo!("MobState::tick_ai - behavior tree execution")
    }

    /// Called when this mob is killed. Handles loot generation and respawn scheduling.
    pub fn on_death(&mut self) {
        self.current_target = None;
        tracing::info!(spawn_id = self.spawn_id, "Mob died, scheduling respawn");
        todo!("MobState::on_death - loot drop + respawn timer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggro_range_check() {
        let mob = MobState::new(1, BeingStats::new(100, 5, 3), 10.0, 30);
        let mob_pos = Vector3::new(0.0, 0.0, 0.0);
        let close = Vector3::new(5.0, 0.0, 0.0);
        let far = Vector3::new(20.0, 0.0, 0.0);

        assert!(mob.is_in_aggro_range(&mob_pos, &close));
        assert!(!mob.is_in_aggro_range(&mob_pos, &far));
    }
}
