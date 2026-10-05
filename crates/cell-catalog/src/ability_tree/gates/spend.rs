//! Spend gates: the archetype-wide spend has opened the node, and the player
//! can pay for it.
//!
//! `required_branch_points` counts trainer points spent **across the
//! archetype** (decision D-AT01, workbook sheets `13_Progression_Rules` and
//! `16_Emulator_Decisions`). Counted per branch, every branch deadlocks after
//! its root (audit A-20). Branch isolation comes from prerequisites instead,
//! which are always in the node's own branch.
//!
//! The spend the gate compares is the **effective** spend:
//! `tree_points_spent` (trainer purchases, D-AT03) plus branch credit, the
//! `skill_point_cost` of every node of the player's archetype tree they hold
//! through a non-`gm` grant provenance (Class Start v6 CS-01a, OD-CS06: a
//! free class signature counts as owned and as branch credit). A starter
//! with no provenance row, a GM grant and a grant of an ability outside the
//! archetype's tree add nothing. Refunds and the respec still use
//! `tree_points_spent` alone: credit is never paid out.

use super::super::catalog::TreeNode;
use super::super::predicate::{TrainContext, TrainReject};

/// Branch credit: the summed `skill_point_cost` of the credited grants that
/// are nodes of `archetype_id`'s tree. A duplicated id counts once.
pub fn grant_credit(
    catalog: &super::super::catalog::AbilityTreeCatalog,
    archetype_id: i32,
    credited_grants: &[i32],
) -> i32 {
    credited_grants
        .iter()
        .enumerate()
        .filter(|&(i, id)| !credited_grants[..i].contains(id))
        .filter_map(|(_, &id)| catalog.node(archetype_id, id))
        .map(|node| node.skill_point_cost.max(0))
        .sum()
}

/// The archetype-wide effective spend meets the node's
/// `required_branch_points`.
///
/// Cell-side only: the base's purchase `UPDATE` does not re-check it. That
/// is safe while the effective spend only grows between the cell's reads,
/// because a stale cell value can only under-count. A respec lowers
/// `tree_points_spent` and resets the cell mirror in the same step (AT-08);
/// the GM reset keeps every credited grant, so it never lowers the credit.
pub(super) fn branch_points(ctx: &TrainContext<'_>, node: &TreeNode) -> Result<(), TrainReject> {
    let spent = ctx.tree_points_spent.saturating_add(grant_credit(
        ctx.catalog,
        node.archetype_id,
        ctx.credited_grants,
    ));
    if spent < node.required_branch_points {
        return Err(TrainReject::SpendGate {
            required: node.required_branch_points,
            spent,
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
