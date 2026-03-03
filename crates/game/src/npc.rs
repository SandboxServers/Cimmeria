use serde::{Deserialize, Serialize};

use crate::being::BeingStats;

/// Runtime state for an SGWNpc entity (non-hostile, interactive NPC).
///
/// NPCs can serve as dialog speakers, vendors, or trainers depending
/// on which optional IDs are set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcState {
    pub stats: BeingStats,
    pub dialog_set_id: Option<i32>,
    pub vendor_list_id: Option<i32>,
    pub trainer_list_id: Option<i32>,
}

impl NpcState {
    /// Create a new NPC with the given stats and no assigned roles.
    pub fn new(stats: BeingStats) -> Self {
        Self {
            stats,
            dialog_set_id: None,
            vendor_list_id: None,
            trainer_list_id: None,
        }
    }

    /// Returns `true` if this NPC has dialog interactions available.
    pub fn has_dialog(&self) -> bool {
        self.dialog_set_id.is_some()
    }

    /// Returns `true` if this NPC can sell items.
    pub fn is_vendor(&self) -> bool {
        self.vendor_list_id.is_some()
    }

    /// Returns `true` if this NPC can train abilities.
    pub fn is_trainer(&self) -> bool {
        self.trainer_list_id.is_some()
    }

    /// Begin a dialog interaction with a player.
    pub fn start_dialog(&self, _player_entity_id: i32) {
        todo!("NpcState::start_dialog - open dialog screen for player")
    }

    /// Open the vendor UI for a player.
    pub fn open_vendor(&self, _player_entity_id: i32) {
        todo!("NpcState::open_vendor - send item list to player client")
    }

    /// Open the trainer UI for a player.
    pub fn open_trainer(&self, _player_entity_id: i32) {
        todo!("NpcState::open_trainer - send ability list to player client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npc_roles() {
        let mut npc = NpcState::new(BeingStats::new(100, 0, 1));
        assert!(!npc.has_dialog());
        assert!(!npc.is_vendor());
        assert!(!npc.is_trainer());

        npc.dialog_set_id = Some(42);
        npc.vendor_list_id = Some(7);
        assert!(npc.has_dialog());
        assert!(npc.is_vendor());
        assert!(!npc.is_trainer());
    }
}
