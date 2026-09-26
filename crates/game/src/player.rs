use serde::{Deserialize, Serialize};

use cimmeria_common::types::AccountId;

/// Maximum character level.
///
/// 50 is a PROJECT FINAL v2 value (ability-trees campaign decision D-AT02),
/// not recovered retail data: the 2009 server capped at 20.
pub const MAX_LEVEL: u32 = 50;

/// Cumulative XP thresholds, indexed by the player's CURRENT level: a
/// level-`L` player advances to `L + 1` once total XP exceeds `LEVEL_XP[L]`.
/// This is the only XP table in the workspace; every consumer (the base's
/// `handle_grant_xp`, the world-entry `onMaxExpUpdate`) reads it through
/// [`max_exp_for_level`] or directly.
///
/// - Index 0 is unused (level 0 does not exist).
/// - Levels 1-20 are the values ported from `python/common/Constants.py:127-139`,
///   which the deprecated Python itself marked as a placeholder.
/// - Levels 21-49 are PROJECT FINAL v2 values, not retail data: sheet
///   `15_Emulator_Level_1_50` of
///   `docs/analysis/ability-trees/source/SGW_All_Classes_Progression_EMULATOR_FINAL_v2_LEVEL50.xlsx`
///   (column "Server LEVEL_XP[level]"), a 15% cumulative extension rounded
///   to the nearest 5,000.
/// - Index 50 is a display sentinel only. The grant loop stops at
///   [`MAX_LEVEL`], so it never gates a level-up; it is what the client's XP
///   bar is told the "next level" threshold is at the cap. There is no level 51.
pub const LEVEL_XP: [u64; MAX_LEVEL as usize + 1] = [
    0, // Levels 1-10
    100, 200, 300, 600, 1_000, 1_600, 2_500, 4_000, 6_000, 9_000, // Levels 11-20
    14_000, 18_000, 25_000, 40_000, 60_000, 90_000, 120_000, 180_000, 250_000, 400_000,
    // Levels 21-30 (PROJECT FINAL v2)
    460_000, 530_000, 610_000, 700_000, 805_000, 925_000, 1_065_000, 1_225_000, 1_410_000,
    1_620_000, // Levels 31-40 (PROJECT FINAL v2)
    1_865_000, 2_145_000, 2_465_000, 2_835_000, 3_260_000, 3_750_000, 4_310_000, 4_955_000,
    5_700_000, 6_555_000, // Levels 41-49 (PROJECT FINAL v2), then the level-50 sentinel
    7_540_000, 8_670_000, 9_970_000, 11_465_000, 13_185_000, 15_165_000, 17_440_000, 20_055_000,
    23_065_000, 26_525_000,
];

/// Training points granted per level-up.
///
/// PROJECT FINAL v2 (D-AT02): one point per level. A character holds
/// [`STARTING_TRAINING_POINTS`] at level 1, so an unspent character at level
/// `L` has exactly `L` points and 50 in total at the cap.
pub const TRAINING_POINTS_PER_LEVEL: u32 = 1;

/// Training points a freshly created (level 1) character starts with (D-AT02).
pub const STARTING_TRAINING_POINTS: u32 = 1;

/// The "next level" XP threshold the client is shown for a player at
/// `level` (`onMaxExpUpdate`). Clamped to the table, so a level at or above
/// [`MAX_LEVEL`] gets the display sentinel and level 0 gets 0.
pub fn max_exp_for_level(level: u32) -> u64 {
    LEVEL_XP[level.min(MAX_LEVEL) as usize]
}

/// Advance `level` for a player whose total XP is now `total_xp`, adding
/// [`TRAINING_POINTS_PER_LEVEL`] to `training_points` for each level gained.
/// Returns the levels gained, in order (empty when no boundary is crossed).
///
/// The one level-up rule shared by [`PlayerState::grant_xp`] and the base
/// service's `handle_grant_xp`. It stops at [`MAX_LEVEL`]: XP past the last
/// threshold never makes a level 51 or a point past the cap.
pub fn apply_level_ups(level: &mut u32, training_points: &mut u32, total_xp: u64) -> Vec<u32> {
    let mut gained = Vec::new();
    while *level < MAX_LEVEL && total_xp > LEVEL_XP[*level as usize] {
        *level += 1;
        *training_points = training_points.saturating_add(TRAINING_POINTS_PER_LEVEL);
        gained.push(*level);
    }
    gained
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
    pub training_points: u32,
    pub archetype: String,
    pub faction: String,
    pub is_online: bool,
}

impl PlayerState {
    /// Create a new level-1 player with zero experience and
    /// [`STARTING_TRAINING_POINTS`].
    pub fn new(
        account_id: AccountId,
        character_name: String,
        archetype: String,
        faction: String,
    ) -> Self {
        Self {
            account_id,
            character_name,
            level: 1,
            xp: 0,
            training_points: STARTING_TRAINING_POINTS,
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

        let levels_gained = apply_level_ups(&mut self.level, &mut self.training_points, self.xp);
        for &lvl in &levels_gained {
            tracing::info!(
                player = %self.character_name,
                new_level = lvl,
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

    /// Levels 1-20 must still be the Python-ported values, and 21-50 the
    /// workbook values. Hard-coded from sheet `15_Emulator_Level_1_50`,
    /// column "Server LEVEL_XP[level]", of
    /// `docs/analysis/ability-trees/source/SGW_All_Classes_Progression_EMULATOR_FINAL_v2_LEVEL50.xlsx`
    /// (branch `docs/ability-trees-campaign`), read row by row for levels
    /// 1..=50. Index 0 is the unused level-0 slot. Written out a second time
    /// on purpose so an edit to `LEVEL_XP` cannot also edit its oracle.
    #[test]
    fn level_xp_table_matches_workbook_sheet_15() {
        const WORKBOOK: [u64; 51] = [
            0, 100, 200, 300, 600, 1000, 1600, 2500, 4000, 6000, 9000, 14000, 18000, 25000, 40000,
            60000, 90000, 120000, 180000, 250000, 400000, 460000, 530000, 610000, 700000, 805000,
            925000, 1065000, 1225000, 1410000, 1620000, 1865000, 2145000, 2465000, 2835000,
            3260000, 3750000, 4310000, 4955000, 5700000, 6555000, 7540000, 8670000, 9970000,
            11465000, 13185000, 15165000, 17440000, 20055000, 23065000, 26525000,
        ];
        assert_eq!(MAX_LEVEL, 50);
        assert_eq!(LEVEL_XP.len(), 51);
        assert_eq!(LEVEL_XP, WORKBOOK);
    }

    #[test]
    fn max_exp_for_level_clamps_to_sentinel() {
        assert_eq!(max_exp_for_level(0), 0);
        assert_eq!(max_exp_for_level(1), 100);
        assert_eq!(max_exp_for_level(20), 400_000);
        assert_eq!(max_exp_for_level(21), 460_000);
        assert_eq!(max_exp_for_level(49), 23_065_000);
        assert_eq!(max_exp_for_level(50), 26_525_000);
        assert_eq!(max_exp_for_level(99), 26_525_000);
    }

    /// Level 20 is no longer the cap: crossing 400,000 XP at level 20 must
    /// reach 21 and grant exactly one point.
    #[test]
    fn grant_xp_crosses_20_to_21() {
        let mut p = test_player();
        p.level = 20;
        p.xp = 400_000;
        p.training_points = 20;
        let gained = p.grant_xp(1);
        assert_eq!(gained, vec![21]);
        assert_eq!(p.level, 21);
        assert_eq!(p.training_points, 21);
        assert_eq!(p.xp_for_next_level(), 460_000);
    }

    #[test]
    fn grant_xp_crosses_49_to_50() {
        let mut p = test_player();
        p.level = 49;
        p.xp = 23_065_000;
        p.training_points = 49;
        let gained = p.grant_xp(1);
        assert_eq!(gained, vec![50]);
        assert_eq!(p.level, 50);
        assert_eq!(p.training_points, 50);
    }

    /// At the cap no amount of XP makes a level 51 or a 51st point.
    #[test]
    fn grant_xp_at_max_level_no_overflow() {
        let mut p = test_player();
        p.level = MAX_LEVEL;
        p.xp = 26_525_000;
        p.training_points = 50;
        let gained = p.grant_xp(999_999_999);
        assert!(gained.is_empty());
        assert_eq!(p.level, 50);
        assert_eq!(p.training_points, 50);
    }

    #[test]
    fn xp_for_next_level_at_max_returns_max() {
        let mut p = test_player();
        p.level = 50;
        assert_eq!(p.xp_for_next_level(), u64::MAX);
    }

    #[test]
    fn new_player_starts_with_one_training_point() {
        let p = test_player();
        assert_eq!(p.training_points, 1);
        assert_eq!(STARTING_TRAINING_POINTS, 1);
    }

    #[test]
    fn grant_xp_grants_one_training_point_per_level() {
        let mut p = test_player();
        p.grant_xp(101); // Level 1 -> 2
        assert_eq!(p.training_points, 2);
    }

    #[test]
    fn multi_level_up_grants_cumulative_training_points() {
        let mut p = test_player();
        p.grant_xp(301); // Level 1 -> 4 (3 level-ups)
        assert_eq!(p.training_points, 4); // 1 starting + 3 x 1
    }

    /// v2 economy: an unspent character at level `L` holds exactly `L`
    /// points, 50 in total at the cap.
    #[test]
    fn full_level_progression_1_to_50() {
        let mut p = test_player();
        for level in 2..=MAX_LEVEL {
            let needed = LEVEL_XP[(level - 1) as usize] - p.xp + 1;
            let levels = p.grant_xp(needed);
            assert_eq!(p.level, level, "Should be level {level}");
            assert!(levels.contains(&level));
            assert_eq!(p.training_points, level);
        }
        assert_eq!(p.level, 50);
        assert_eq!(p.training_points, 50);
        // One grant spanning the whole table from level 1 lands on 50 too.
        let mut q = test_player();
        let gained = q.grant_xp(u64::MAX / 2);
        assert_eq!(gained.len(), 49);
        assert_eq!(q.level, 50);
        assert_eq!(q.training_points, 50);
    }

    #[test]
    fn xp_table_is_strictly_increasing() {
        for i in 1..LEVEL_XP.len() - 1 {
            assert!(
                LEVEL_XP[i] < LEVEL_XP[i + 1],
                "XP table not strictly increasing at index {i}: {} >= {}",
                LEVEL_XP[i],
                LEVEL_XP[i + 1]
            );
        }
    }
}
