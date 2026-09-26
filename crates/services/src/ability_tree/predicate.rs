//! `evaluate_train`: the only place a trainability gate is decided.

use std::collections::HashSet;

use cimmeria_entity::abilities::AbilityManager;

use super::catalog::{AbilityTreeCatalog, TreeNode};
use super::gates;

/// The player's known-ability set, however the caller holds it.
pub trait KnownAbilities {
    fn knows(&self, ability_id: i32) -> bool;
}

impl KnownAbilities for AbilityManager {
    fn knows(&self, ability_id: i32) -> bool {
        self.has_ability(ability_id)
    }
}

impl KnownAbilities for HashSet<i32> {
    fn knows(&self, ability_id: i32) -> bool {
        self.contains(&ability_id)
    }
}

/// Everything a gate may read. Built by the caller from its own state; the
/// predicate never touches `SpaceManager` or the database.
pub struct TrainContext<'a> {
    pub catalog: &'a AbilityTreeCatalog,
    /// The ability the player wants to train.
    pub ability_id: i32,
    /// Whether `ability_id` resolves to an ability definition
    /// (`SpaceManager::ability_defs` on the cell).
    pub ability_exists: bool,
    /// `sgw_player.player_id`; `None` for an entity that is not a loaded
    /// player character.
    pub player_id: Option<i32>,
    /// `EArchetype` enum position; `None` before `InitPlayerState`.
    pub archetype_id: Option<i32>,
    pub level: i32,
    pub known: &'a dyn KnownAbilities,
    /// Archetype-wide tree points spent (`sgw_player.tree_points_spent`).
    pub tree_points_spent: i32,
    /// Unspent training points (`sgw_player.training_points`).
    pub training_points: i32,
}

/// A purchase that passed every gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainPlan {
    pub player_id: i32,
    pub archetype_id: i32,
    pub ability_id: i32,
    pub tree_index: i32,
    /// Training points to debit: the node's `skill_point_cost`.
    pub cost: i32,
    /// The ability's authored `training_cost`, for diagnostics only.
    pub raw_training_cost: i32,
}

/// Why a node cannot be trained. One variant per gate outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainReject {
    /// `ability_id` has no ability definition.
    UnknownAbility,
    /// The entity has no `player_id`.
    NoPlayerId,
    /// The player already knows the ability. Callers keep this silent: it is
    /// a replayed packet or a double-click, not an error.
    AlreadyKnown,
    /// The entity has no archetype.
    NoArchetype,
    /// The ability is not in the player's archetype tree.
    NotInArchetypeTree,
    LevelTooLow {
        required: i32,
        actual: i32,
    },
    /// The first prerequisite (in node order) the player does not know.
    MissingPrerequisite {
        missing: i32,
    },
    /// The archetype-wide spend is below the node's `required_branch_points`.
    SpendGate {
        required: i32,
        spent: i32,
    },
    /// Training points are below the node's `skill_point_cost`.
    NotEnoughPoints {
        cost: i32,
        available: i32,
    },
}

impl TrainReject {
    /// Stable snake_case name for logs and metrics. The last three match
    /// the `train_rejected reason=` values `train.rs` has always logged.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::UnknownAbility => "unknown_ability",
            Self::NoPlayerId => "no_player_id",
            Self::AlreadyKnown => "already_known",
            Self::NoArchetype => "no_archetype",
            Self::NotInArchetypeTree => "not_in_archetype_tree",
            Self::LevelTooLow { .. } => "level_too_low",
            Self::MissingPrerequisite { .. } => "missing_prerequisite",
            Self::SpendGate { .. } => "spend_gate",
            Self::NotEnoughPoints { .. } => "not_enough_points",
        }
    }
}

/// Decide whether the player in `ctx` may train `ctx.ability_id`.
///
/// Resolution first (the ability exists, the player is a loaded character
/// who does not already know it, and the node is in their archetype's tree),
/// then every node gate in `gates::NODE_GATES`, in order. The first failure
/// wins, so the gate order is the rejection priority.
pub fn evaluate_train(ctx: &TrainContext<'_>) -> Result<TrainPlan, TrainReject> {
    let resolved = gates::node::resolve(ctx)?;
    for gate in gates::NODE_GATES {
        gate(ctx, resolved.node)?;
    }
    Ok(plan(&resolved))
}

fn plan(resolved: &gates::Resolved<'_>) -> TrainPlan {
    let node: &TreeNode = resolved.node;
    TrainPlan {
        player_id: resolved.player_id,
        archetype_id: resolved.archetype_id,
        ability_id: node.ability_id,
        tree_index: node.tree_index,
        cost: node.skill_point_cost,
        raw_training_cost: node.raw_training_cost,
    }
}
