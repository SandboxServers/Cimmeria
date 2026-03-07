use serde::{Deserialize, Serialize};

/// Loot distribution mode for a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LootMode {
    FreeForAll,
    RoundRobin,
    MasterLooter,
}

/// A player group (party) for cooperative play.
///
/// Groups are ephemeral and exist only while members are online.
/// Corresponds to the group management logic in the Python layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub group_id: u32,
    pub leader_entity_id: i32,
    pub member_entity_ids: Vec<i32>,
    pub max_size: usize,
    pub loot_mode: LootMode,
}

impl Group {
    /// Create a new group with the creator as leader.
    pub fn new(group_id: u32, leader_entity_id: i32) -> Self {
        Self {
            group_id,
            leader_entity_id,
            member_entity_ids: vec![leader_entity_id],
            max_size: 4,
            loot_mode: LootMode::FreeForAll,
        }
    }

    /// Invite a player to the group. Returns `false` if the group is full.
    pub fn add_member(&mut self, entity_id: i32) -> bool {
        if self.member_entity_ids.len() >= self.max_size {
            return false;
        }
        if self.member_entity_ids.contains(&entity_id) {
            return false;
        }
        self.member_entity_ids.push(entity_id);
        true
    }

    /// Remove a member from the group.
    pub fn remove_member(&mut self, entity_id: i32) {
        self.member_entity_ids.retain(|&id| id != entity_id);
        // If leader left, promote next member
        if entity_id == self.leader_entity_id {
            if let Some(&new_leader) = self.member_entity_ids.first() {
                self.leader_entity_id = new_leader;
            }
        }
    }

    /// Returns `true` if the group has no members (should be disbanded).
    pub fn is_empty(&self) -> bool {
        self.member_entity_ids.is_empty()
    }

    /// Returns the number of current members.
    pub fn member_count(&self) -> usize {
        self.member_entity_ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_group_has_leader() {
        let g = Group::new(1, 100);
        assert_eq!(g.member_count(), 1);
        assert_eq!(g.leader_entity_id, 100);
    }

    #[test]
    fn add_member_up_to_max() {
        let mut g = Group::new(1, 100);
        assert!(g.add_member(101));
        assert!(g.add_member(102));
        assert!(g.add_member(103));
        assert!(!g.add_member(104)); // full
    }

    #[test]
    fn leader_promotion_on_leave() {
        let mut g = Group::new(1, 100);
        g.add_member(101);
        g.remove_member(100);
        assert_eq!(g.leader_entity_id, 101);
    }
}
