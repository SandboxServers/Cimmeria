use serde::{Deserialize, Serialize};

/// A reward granted upon mission completion.
///
/// Corresponds to the mission_rewards and mission_reward_groups tables
/// in the database schema. A mission can grant multiple rewards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionReward {
    pub reward_id: i32,
    pub xp: u64,
    pub money: i64,
    pub item_template_id: Option<i32>,
    pub item_count: u32,
}

impl MissionReward {
    /// Create an XP-only reward.
    pub fn xp_only(reward_id: i32, xp: u64) -> Self {
        Self {
            reward_id,
            xp,
            money: 0,
            item_template_id: None,
            item_count: 0,
        }
    }

    /// Create a reward with both XP and money.
    pub fn xp_and_money(reward_id: i32, xp: u64, money: i64) -> Self {
        Self {
            reward_id,
            xp,
            money,
            item_template_id: None,
            item_count: 0,
        }
    }

    /// Create an item reward.
    pub fn item(reward_id: i32, item_template_id: i32, item_count: u32) -> Self {
        Self {
            reward_id,
            xp: 0,
            money: 0,
            item_template_id: Some(item_template_id),
            item_count,
        }
    }

    /// Grant this reward to a player entity.
    pub fn grant_to(&self, _player_entity_id: i32) {
        todo!("MissionReward::grant_to - apply XP, money, and items to player")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xp_only_reward() {
        let r = MissionReward::xp_only(1, 500);
        assert_eq!(r.xp, 500);
        assert_eq!(r.money, 0);
        assert!(r.item_template_id.is_none());
    }

    #[test]
    fn item_reward() {
        let r = MissionReward::item(2, 42, 3);
        assert_eq!(r.item_template_id, Some(42));
        assert_eq!(r.item_count, 3);
    }
}
