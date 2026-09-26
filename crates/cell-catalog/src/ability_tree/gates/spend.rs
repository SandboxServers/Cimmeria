//! Spend gates: the archetype-wide spend has opened the node, and the player
//! can pay for it.
//!
//! `required_branch_points` counts trainer points spent **across the
//! archetype** (decision D-AT01, workbook sheets `13_Progression_Rules` and
//! `16_Emulator_Decisions`). Counted per branch, every branch deadlocks after
//! its root (audit A-20). Branch isolation comes from prerequisites instead,
//! which are always in the node's own branch.
//!
//! Only trainer purchases add to `tree_points_spent` (D-AT03). A starter or
//! quest-granted ability satisfies a prerequisite, but it never counts as
//! spend.

use super::super::catalog::TreeNode;
use super::super::predicate::{TrainContext, TrainReject};

/// The archetype-wide spend meets the node's `required_branch_points`.
///
/// Cell-side only: the base's purchase `UPDATE` does not re-check it. That
/// is safe while `tree_points_spent` only grows, because a stale cell value
/// can only under-count. A respec that lowers the spend (AT-08) must reset
/// the cell mirror in the same step, or add a base-side check.
pub(super) fn branch_points(ctx: &TrainContext<'_>, node: &TreeNode) -> Result<(), TrainReject> {
    if ctx.tree_points_spent < node.required_branch_points {
        return Err(TrainReject::SpendGate {
            required: node.required_branch_points,
            spent: ctx.tree_points_spent,
        });
    }
    Ok(())
}

/// The player has at least the node's `skill_point_cost` in training points.
pub(super) fn points(ctx: &TrainContext<'_>, node: &TreeNode) -> Result<(), TrainReject> {
    if ctx.training_points < node.skill_point_cost {
        return Err(TrainReject::NotEnoughPoints {
            cost: node.skill_point_cost,
            available: ctx.training_points,
        });
    }
    Ok(())
}
