//! `AbilityTreeCatalog::load` against the seeded database.

use super::super::*;
use crate::test_support::require_db_or_skip;

/// Every loaded node must carry the seeded v2 column values exactly as
/// the table holds them, and `raw_training_cost` must come from
/// `resources.abilities.training_cost`.
#[tokio::test]
async fn catalog_loads_seed_v2_columns_and_joined_training_cost() {
    let pool = require_db_or_skip!();
    let catalog = AbilityTreeCatalog::load(&pool).await.expect("catalog load");

    let seeded: i64 = sqlx::query_scalar("SELECT count(*) FROM resources.archetype_ability_tree")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(catalog.len() as i64, seeded, "one node per table row");
    assert!(!catalog.is_empty(), "seed must provide tree rows");

    // Soldier (1) through Sholva (7) are seeded; ordering follows
    // tree_index, then ability_index.
    for arch in 1..=7 {
        let tree = catalog.tree(arch);
        assert!(!tree.is_empty(), "archetype {arch} must have a tree");
        assert!(
            tree.windows(2)
                .all(|w| (w[0].tree_index, w[0].node_order) < (w[1].tree_index, w[1].node_order)),
            "archetype {arch} tree must be ordered by tree_index, ability_index"
        );
    }

    let (ability_id, cost): (i32, i32) = sqlx::query_as(
        "SELECT t.ability_id, a.training_cost          FROM resources.archetype_ability_tree t          JOIN resources.abilities a ON a.ability_id = t.ability_id          WHERE t.archetype = 'ARCHETYPE_Commando'          ORDER BY t.tree_index, t.ability_index LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let node = catalog.node(2, ability_id).expect("Commando node present");
    assert_eq!(node.raw_training_cost, cost, "training_cost is joined");

    // The loader must map each v2 column to its own field. Compare every
    // node against the raw row so a swapped or dropped column fails here.
    type V2Row = (i32, i32, i32, i32, bool, bool, Option<String>);
    let rows: Vec<V2Row> = sqlx::query_as(
        "SELECT array_position(enum_range(NULL::resources.\"EArchetype\"), archetype) - 1,                 ability_id, required_branch_points, skill_point_cost,                 is_branch_root, is_capstone, branch_name          FROM resources.archetype_ability_tree",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    for (arch, id, branch_points, point_cost, root, capstone, branch) in &rows {
        let node = catalog
            .node(*arch, *id)
            .unwrap_or_else(|| panic!("archetype {arch} ability {id} missing from catalog"));
        assert_eq!(
            (
                node.required_branch_points,
                node.skill_point_cost,
                node.is_branch_root,
                node.is_capstone,
                node.branch_name.as_deref(),
            ),
            (*branch_points, *point_cost, *root, *capstone, branch.as_deref()),
            "archetype {arch} ability {id}: catalog must carry the seeded v2 columns"
        );
    }

    // The comparison above is only meaningful if the seed sets values
    // other than the schema defaults (0, 1, false, false, NULL).
    let nodes = || (1..=7).flat_map(|arch| catalog.tree(arch).iter());
    assert!(nodes().any(|n| n.required_branch_points > 0), "seed sets spend gates");
    assert!(nodes().any(|n| n.is_branch_root), "seed sets branch roots");
    assert!(nodes().any(|n| n.is_capstone), "seed sets capstones");
    assert!(nodes().all(|n| n.branch_name.is_some()), "seed names every branch");
}
