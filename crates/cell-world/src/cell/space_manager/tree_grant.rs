//! Which archetype-tree abilities a player does not know yet: the plan the
//! GM `gmGiveAllAbilities` (154) and the Debug Area ability granter
//! (`gm_ability_bulk` content action) both send to the base.

use super::SpaceManager;

/// A planned "grant the whole tree".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeGrantPlan {
    /// The player's character.
    pub player_id: i32,
    /// `sgw_player.archetype`, the tree's key.
    pub archetype: i32,
    /// How many nodes the archetype's tree holds (capstones included).
    pub tree_len: usize,
    /// The tree's ability ids the player does not know, in tree order,
    /// without repeats. Never empty.
    pub missing: Vec<i32>,
}

/// Why [`SpaceManager::plan_tree_grant`] has nothing to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeGrantRefusal {
    /// No such entity.
    EntityGone,
    /// The entity has no `player_id`.
    NotPlayer,
    /// The character has no archetype.
    NoArchetype,
    /// No tree is loaded for the archetype.
    NoTree { archetype: i32 },
    /// The player knows every ability of the tree already.
    NothingToGrant { tree_len: usize },
}

impl TreeGrantRefusal {
    /// The `reason` value logs carry.
    pub fn reason(self) -> &'static str {
        match self {
            Self::EntityGone => "caller_gone",
            Self::NotPlayer => "caller_not_player",
            Self::NoArchetype => "no_archetype",
            Self::NoTree { .. } => "no_tree",
            Self::NothingToGrant { .. } => "nothing_to_grant",
        }
    }
}

impl SpaceManager {
    /// Plan granting `entity_id` every ability of its archetype's tree that
    /// it does not know yet: all three branches and the capstones.
    pub fn plan_tree_grant(&self, entity_id: u32) -> Result<TreeGrantPlan, TreeGrantRefusal> {
        let entity = self
            .get_entity(entity_id)
            .ok_or(TreeGrantRefusal::EntityGone)?;
        let player_id = entity.player_id.ok_or(TreeGrantRefusal::NotPlayer)?;
        let archetype = entity.archetype_id.ok_or(TreeGrantRefusal::NoArchetype)?;
        let tree = self.ability_tree_catalog.tree(archetype);
        if tree.is_empty() {
            return Err(TreeGrantRefusal::NoTree { archetype });
        }
        let mut missing: Vec<i32> = Vec::new();
        for node in tree {
            let id = node.ability_id;
            if !entity.abilities.has_ability(id) && !missing.contains(&id) {
                missing.push(id);
            }
        }
        if missing.is_empty() {
            return Err(TreeGrantRefusal::NothingToGrant {
                tree_len: tree.len(),
            });
        }
        Ok(TreeGrantPlan {
            player_id,
            archetype,
            tree_len: tree.len(),
            missing,
        })
    }
}
