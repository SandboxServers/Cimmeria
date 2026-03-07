use serde::{Deserialize, Serialize};

/// Mission objective types corresponding to the mission_objectives table.
///
/// Each variant represents a different way a player can make progress on
/// a mission step (kill enemies, collect items, visit a location, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MissionObjective {
    /// Kill a certain number of a specific mob type.
    KillCount {
        mob_moniker: String,
        required: u32,
        current: u32,
    },
    /// Collect a certain number of a specific item.
    CollectItem {
        item_template_id: i32,
        required: u32,
        current: u32,
    },
    /// Enter a specific region in the world.
    VisitRegion {
        region_id: i32,
        visited: bool,
    },
    /// Talk to a specific NPC.
    TalkToNpc {
        npc_moniker: String,
        talked: bool,
    },
    /// Interact with a specific world object.
    UseObject {
        interaction_id: i32,
        used: bool,
    },
    /// Survive for a duration (escort/defense missions).
    Timer {
        duration_secs: f32,
        elapsed_secs: f32,
    },
}

impl MissionObjective {
    /// Returns `true` if this objective has been fully completed.
    pub fn is_complete(&self) -> bool {
        match self {
            MissionObjective::KillCount { required, current, .. } => current >= required,
            MissionObjective::CollectItem { required, current, .. } => current >= required,
            MissionObjective::VisitRegion { visited, .. } => *visited,
            MissionObjective::TalkToNpc { talked, .. } => *talked,
            MissionObjective::UseObject { used, .. } => *used,
            MissionObjective::Timer { duration_secs, elapsed_secs } => elapsed_secs >= duration_secs,
        }
    }

    /// Completion ratio for progress display (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        match self {
            MissionObjective::KillCount { required, current, .. } => {
                *current as f32 / *required as f32
            }
            MissionObjective::CollectItem { required, current, .. } => {
                *current as f32 / *required as f32
            }
            MissionObjective::VisitRegion { visited, .. } => if *visited { 1.0 } else { 0.0 },
            MissionObjective::TalkToNpc { talked, .. } => if *talked { 1.0 } else { 0.0 },
            MissionObjective::UseObject { used, .. } => if *used { 1.0 } else { 0.0 },
            MissionObjective::Timer { duration_secs, elapsed_secs } => {
                (elapsed_secs / duration_secs).min(1.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kill_count_completion() {
        let obj = MissionObjective::KillCount {
            mob_moniker: "jaffa_guard".to_string(),
            required: 5,
            current: 5,
        };
        assert!(obj.is_complete());
        assert!((obj.progress() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn kill_count_partial() {
        let obj = MissionObjective::KillCount {
            mob_moniker: "jaffa_guard".to_string(),
            required: 10,
            current: 3,
        };
        assert!(!obj.is_complete());
        assert!((obj.progress() - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn visit_region_not_visited() {
        let obj = MissionObjective::VisitRegion {
            region_id: 1,
            visited: false,
        };
        assert!(!obj.is_complete());
        assert!((obj.progress() - 0.0).abs() < f32::EPSILON);
    }
}
