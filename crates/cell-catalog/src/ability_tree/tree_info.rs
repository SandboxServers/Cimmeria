//! The `onAbilityTreeInfo` payload, built from the catalog.
//!
//! The client joins this list with the trainer's `onTrainerOpen` list, so
//! it must come from the same [`AbilityTreeCatalog`] the trainer's
//! `evaluate_train` reads, in the same order. There is no second query and
//! no hand-copied fallback: an archetype with no rows gets an empty tree
//! and one WARN.

use cimmeria_entity::abilities::AbilityTreeData;

use super::AbilityTreeCatalog;

/// The archetype's tree grouped into the three `onAbilityTreeInfo`
/// branches. Within a branch, ids keep catalog order (`tree_index,
/// ability_index`), which is the order the trainer sees.
///
/// `player_id` only labels the logs.
pub fn tree_info(
    catalog: &AbilityTreeCatalog,
    archetype_id: i32,
    player_id: i32,
) -> AbilityTreeData {
    let nodes = catalog.tree(archetype_id);
    let mut data = AbilityTreeData::default();
    if nodes.is_empty() {
        tracing::warn!(
            target: "abilities",
            event = "tree_missing",
            reason = "archetype_has_no_tree",
            player_id,
            archetype_id,
            "Archetype has no ability-tree rows; sending an empty onAbilityTreeInfo"
        );
        return data;
    }

    let branch_count = data.trees.len();
    for node in nodes {
        // `tree_index_sanity` keeps this in 0..=2, so a miss is a schema
        // change the wire shape (always three branches) cannot carry.
        match usize::try_from(node.tree_index) {
            Ok(i) if i < branch_count => data.trees[i].push(node.ability_id),
            _ => tracing::error!(
                target: "abilities",
                event = "tree_node_dropped",
                reason = "tree_index_out_of_range",
                player_id,
                archetype_id,
                ability_id = node.ability_id,
                tree_index = node.tree_index,
                "Ability-tree node outside the three onAbilityTreeInfo branches; dropped"
            ),
        }
    }
    data
}
