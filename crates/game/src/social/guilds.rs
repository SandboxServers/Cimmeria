use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Guild rank matching EOrganizationRank from the database schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GuildRank {
    Member,
    Officer,
    Leader,
}

/// A member entry within a guild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMember {
    pub account_id: i32,
    pub character_name: String,
    pub rank: GuildRank,
}

/// A persistent guild (organization).
///
/// Corresponds to the EOrganizationType/EPersistentOrganizationType
/// enums and the organization persistence tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guild {
    pub guild_id: i32,
    pub name: String,
    pub leader_account_id: i32,
    pub members: HashMap<i32, GuildMember>,
}

impl Guild {
    /// Create a new guild with the founding player as leader.
    pub fn new(guild_id: i32, name: String, leader_account_id: i32, leader_name: String) -> Self {
        let mut members = HashMap::new();
        members.insert(
            leader_account_id,
            GuildMember {
                account_id: leader_account_id,
                character_name: leader_name,
                rank: GuildRank::Leader,
            },
        );

        Self {
            guild_id,
            name,
            leader_account_id,
            members,
        }
    }

    /// Add a member at the default rank.
    pub fn add_member(&mut self, account_id: i32, character_name: String) {
        self.members.insert(
            account_id,
            GuildMember {
                account_id,
                character_name,
                rank: GuildRank::Member,
            },
        );
    }

    /// Remove a member. Cannot remove the leader.
    pub fn remove_member(&mut self, account_id: i32) -> bool {
        if account_id == self.leader_account_id {
            return false;
        }
        self.members.remove(&account_id).is_some()
    }

    /// Promote a member to the given rank.
    pub fn set_rank(&mut self, account_id: i32, rank: GuildRank) -> bool {
        if let Some(member) = self.members.get_mut(&account_id) {
            member.rank = rank;
            true
        } else {
            false
        }
    }

    /// Persist guild state to the database.
    pub fn save(&self) {
        todo!("Guild::save - write to organization persistence tables")
    }

    /// Load a guild from the database.
    pub fn load(_guild_id: i32) -> Option<Self> {
        todo!("Guild::load - read from organization persistence tables")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_guild_has_leader() {
        let g = Guild::new(1, "SG-1".to_string(), 10, "O'Neill".to_string());
        assert_eq!(g.members.len(), 1);
        assert_eq!(g.members[&10].rank, GuildRank::Leader);
    }

    #[test]
    fn cannot_remove_leader() {
        let mut g = Guild::new(1, "SG-1".to_string(), 10, "O'Neill".to_string());
        assert!(!g.remove_member(10));
    }

    #[test]
    fn add_and_promote_member() {
        let mut g = Guild::new(1, "SG-1".to_string(), 10, "O'Neill".to_string());
        g.add_member(11, "Carter".to_string());
        assert!(g.set_rank(11, GuildRank::Officer));
        assert_eq!(g.members[&11].rank, GuildRank::Officer);
    }
}
