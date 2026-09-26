//! `AbilityTreeCatalog::load` against the seeded database.

use super::super::*;
use crate::test_support::require_db_or_skip;

/// The stub seed sets none of the v2 columns, so every loaded node must
/// carry the schema defaults, and `raw_training_cost` must come from
/// `resources.abilities.training_cost`.
#[tokio::test]
async fn catalog_loads_seed_with_v2_defaults_and_joined_training_cost() {
    let pool = require_db_or_skip!();
    let catalog = AbilityTreeCatalog::load(&pool).await.expect("catalog load");

    let seeded: i64 = sqlx::query_scalar("SELECT count(*) FROM resources.archetype_ability_tree")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(catalog.len() as i64, seeded, "one node per table row");
    assert!(!catalog.is_empty(), "seed must provide tree rows");

    // Soldier (1) and Commando (2) are seeded; ordering follows tree_index,
    // then ability_index.
    for arch in [1, 2] {
        let tree = catalog.tree(arch);
        assert!(!tree.is_empty(), "archetype {arch} must have a tree");
        assert!(
            tree.windows(2)
                .all(|w| (w[0].tree_index, w[0].node_order) < (w[1].tree_index, w[1].node_order)),
            "archetype {arch} tree must be ordered by tree_index, ability_index"
        );
    }

    let (ability_id, cost): (i32, i32) = sqlx::query_as(
        "SELECT t.ability_id, a.training_cost \
         FROM resources.archetype_ability_tree t \
         JOIN resources.abilities a ON a.ability_id = t.ability_id \
         WHERE t.archetype = 'ARCHETYPE_Commando' \
         ORDER BY t.tree_index, t.ability_index LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let node = catalog.node(2, ability_id).expect("Commando node present");
    assert_eq!(node.raw_training_cost, cost, "training_cost is joined");

    for node in catalog.tree(1).iter().chain(catalog.tree(2)) {
        assert_eq!(
            (
                node.required_branch_points,
                node.skill_point_cost,
                node.is_branch_root,
                node.is_capstone,
            ),
            (0, 1, false, false),
            "stub row {} must carry the v2 column defaults",
            node.ability_id
        );
    }
}
