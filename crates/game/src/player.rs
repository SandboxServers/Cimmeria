use serde::{Deserialize, Serialize};

use cimmeria_common::types::AccountId;

/// Experience required per level. Placeholder formula: 1000 * level^2.
fn xp_for_level(level: u32) -> u64 {
    1000 * (level as u64) * (level as u64)
}

/// Persistent player state corresponding to the SGWPlayer entity.
///
/// Maps to the `sgw.sgw_player` table and the Python `SGWPlayer` class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub account_id: AccountId,
    pub character_name: String,
    pub level: u32,
    pub xp: u64,
    pub archetype: String,
    pub faction: String,
    pub is_online: bool,
}

impl PlayerState {
    /// Create a new level-1 player with zero experience.
    pub fn new(account_id: AccountId, character_name: String, archetype: String, faction: String) -> Self {
        Self {
            account_id,
            character_name,
            level: 1,
            xp: 0,
            archetype,
            faction,
            is_online: false,
        }
    }

    /// Award experience points and trigger level-ups as needed.
    pub fn grant_xp(&mut self, amount: u64) {
        self.xp += amount;
        tracing::debug!(
            player = %self.character_name,
            xp_gained = amount,
            total_xp = self.xp,
            "XP granted"
        );

        while self.xp >= self.xp_for_next_level() {
            self.level_up();
        }
    }

    /// Advance one level. Called automatically by `grant_xp` when the
    /// threshold is reached.
    pub fn level_up(&mut self) {
        self.level += 1;
        tracing::info!(
            player = %self.character_name,
            new_level = self.level,
            "Player leveled up"
        );
    }

    /// XP required to reach the next level from the current one.
    pub fn xp_for_next_level(&self) -> u64 {
        xp_for_level(self.level)
    }

    /// Persist player state to the database.
    pub fn save(&self) {
        todo!("PlayerState::save - write to sgw.sgw_player")
    }

    /// Load player state from the database.
    pub fn load(_account_id: AccountId, _character_name: &str) -> Option<Self> {
        todo!("PlayerState::load - read from sgw.sgw_player")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_player() -> PlayerState {
        PlayerState::new(
            AccountId(1),
            "Teal'c".to_string(),
            "Jaffa".to_string(),
            "SGC".to_string(),
        )
    }

    #[test]
    fn new_player_starts_at_level_1() {
        let p = test_player();
        assert_eq!(p.level, 1);
        assert_eq!(p.xp, 0);
        assert!(!p.is_online);
    }

    #[test]
    fn grant_xp_below_threshold() {
        let mut p = test_player();
        p.grant_xp(500);
        assert_eq!(p.level, 1);
        assert_eq!(p.xp, 500);
    }

    #[test]
    fn grant_xp_triggers_level_up() {
        let mut p = test_player();
        // Level 1 -> 2 requires xp_for_level(1) = 1000
        p.grant_xp(1000);
        assert_eq!(p.level, 2);
    }

    #[test]
    fn xp_for_next_level_scales() {
        let p = test_player();
        assert_eq!(p.xp_for_next_level(), 1000); // level 1: 1000*1*1
    }
}
