use serde::{Deserialize, Serialize};

use cimmeria_common::types::AccountId;

/// Maximum character level.
pub const MAX_LEVEL: u32 = 20;

/// Cumulative XP thresholds per level, ported from `python/common/Constants.py:127-139`.
/// Index 0 is unused (level 0 doesn't exist). Level N requires `LEVEL_XP[N]` total XP.
const LEVEL_XP: [u64; 21] = [
    0,
    // Levels 1-10
    100, 200, 300, 600, 1_000, 1_600, 2_500, 4_000, 6_000, 9_000,
    // Levels 11-20
    14_000, 18_000, 25_000, 40_000, 60_000, 90_000, 120_000, 180_000, 250_000, 400_000,
];

/// Training points granted per level-up.
pub const TRAINING_POINTS_PER_LEVEL: u32 = 2;

/// Persistent player state corresponding to the SGWPlayer entity.
///
/// Maps to the `sgw.sgw_player` table and the Python `SGWPlayer` class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub account_id: AccountId,
    pub character_name: String,
    pub level: u32,
    pub xp: u64,
    pub training_points: u32,
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
            training_points: 0,
            archetype,
            faction,
            is_online: false,
        }
    }

    /// Award experience points and trigger level-ups as needed.
    /// Returns a list of new levels gained (empty if no level-up occurred).
    pub fn grant_xp(&mut self, amount: u64) -> Vec<u32> {
        self.xp += amount;
        tracing::debug!(
            player = %self.character_name,
            xp_gained = amount,
            total_xp = self.xp,
            "XP granted"
        );

        let mut levels_gained = Vec::new();
        while self.level < MAX_LEVEL && self.xp > LEVEL_XP[self.level as usize] {
            self.level += 1;
            self.training_points += TRAINING_POINTS_PER_LEVEL;
            levels_gained.push(self.level);
            tracing::info!(
                player = %self.character_name,
                new_level = self.level,
                "Player leveled up"
            );
        }
        levels_gained
    }

    /// XP threshold to reach the next level. Returns `u64::MAX` at max level.
    pub fn xp_for_next_level(&self) -> u64 {
        if self.level >= MAX_LEVEL {
            return u64::MAX;
        }
        LEVEL_XP[self.level as usize]
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
    fn xp_for_next_level_uses_table() {
        let p = test_player();
        assert_eq!(p.xp_for_next_level(), 100); // Level 1 threshold
    }

    #[test]
    fn grant_xp_below_threshold_no_level_up() {
        let mut p = test_player();
        p.grant_xp(50);
        assert_eq!(p.level, 1);
        assert_eq!(p.xp, 50);
    }

    #[test]
    fn grant_xp_at_threshold_triggers_level_up() {
        let mut p = test_player();
        p.grant_xp(101); // > 100 threshold for level 1
        assert_eq!(p.level, 2);
        assert_eq!(p.xp, 101);
    }

    #[test]
    fn grant_xp_multi_level_up() {
        let mut p = test_player();
        p.grant_xp(301); // > 100 (level 2) and > 200 (level 3) and > 300 (level 4)
        assert_eq!(p.level, 4);
    }

    #[test]
    fn grant_xp_at_max_level_no_overflow() {
        let mut p = test_player();
        p.level = 20;
        p.xp = 400_000;
        let old_level = p.level;
        p.grant_xp(999_999);
        assert_eq!(p.level, old_level);
    }

    #[test]
    fn xp_for_next_level_at_max_returns_max() {
        let mut p = test_player();
        p.level = 20;
        assert_eq!(p.xp_for_next_level(), u64::MAX);
    }

    #[test]
    fn new_player_has_zero_training_points() {
        let p = test_player();
        assert_eq!(p.training_points, 0);
    }

    #[test]
    fn grant_xp_grants_training_points_on_level_up() {
        let mut p = test_player();
        p.grant_xp(101); // Level 1 → 2
        assert_eq!(p.training_points, 2);
    }

    #[test]
    fn multi_level_up_grants_cumulative_training_points() {
        let mut p = test_player();
        p.grant_xp(301); // Level 1 → 4 (3 level-ups)
        assert_eq!(p.training_points, 6); // 3 × 2
    }

    #[test]
    fn full_level_progression_1_to_20() {
        let mut p = test_player();
        for level in 2..=20u32 {
            let needed = LEVEL_XP[(level - 1) as usize] - p.xp + 1;
            let levels = p.grant_xp(needed);
            assert_eq!(p.level, level, "Should be level {level}");
            assert!(levels.contains(&level));
            assert_eq!(p.training_points, (level - 1) * TRAINING_POINTS_PER_LEVEL);
        }
        assert_eq!(p.level, MAX_LEVEL);
        assert_eq!(p.training_points, 38); // 19 * 2
    }

    #[test]
    fn xp_table_is_monotonically_nondecreasing() {
        for i in 1..LEVEL_XP.len() - 1 {
            assert!(
                LEVEL_XP[i] <= LEVEL_XP[i + 1],
                "XP table not monotonic at index {i}: {} > {}",
                LEVEL_XP[i], LEVEL_XP[i + 1]
            );
        }
    }
}
