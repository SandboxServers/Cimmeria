use serde::{Deserialize, Serialize};

use crate::inventory::loot::LootDrop;

/// State of a lootable container or corpse in the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootableState {
    pub entity_id: i32,
    pub loot_table_id: i32,
    pub contents: Vec<LootDrop>,
    pub is_looted: bool,
}

impl LootableState {
    /// Create a new lootable that hasn't been rolled yet.
    pub fn new(entity_id: i32, loot_table_id: i32) -> Self {
        Self {
            entity_id,
            loot_table_id,
            contents: Vec::new(),
            is_looted: false,
        }
    }

    /// Roll the loot table and populate contents. Only rolls once.
    pub fn generate_contents(&mut self) {
        if !self.contents.is_empty() {
            return;
        }
        todo!("LootableState::generate_contents - look up LootTable and roll")
    }

    /// Player takes a specific item from the loot window.
    pub fn take_item(&mut self, _index: usize, _player_entity_id: i32) -> LootResult {
        todo!("LootableState::take_item - move item to player inventory")
    }

    /// Player takes all remaining items.
    pub fn take_all(&mut self, _player_entity_id: i32) -> Vec<LootResult> {
        todo!("LootableState::take_all - move all items to player inventory")
    }
}

/// Result of a loot pickup attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LootResult {
    Success,
    InventoryFull,
    AlreadyLooted,
    InvalidIndex,
}
