//! `tree_info`: the `onAbilityTreeInfo` payload built from the catalog.

use std::collections::HashSet;

use super::super::*;
use crate::test_support::LogCapture;

const ARCH: i32 = 3;

/// Branches deliberately interleaved and ids descending inside each
/// branch, so a sort by id or a regroup that loses insertion order shows.
fn scrambled_catalog() -> AbilityTreeCatalog {
    AbilityTreeCatalog::from_nodes([
        TreeNode::with_defaults(ARCH, 1, 912, 1, vec![]),
        TreeNode::with_defaults(ARCH, 0, 905, 1, vec![]),
        TreeNode::with_defaults(ARCH, 2, 930, 1, vec![]),
        TreeNode::with_defaults(ARCH, 0, 901, 1, vec![]),
        TreeNode::with_defaults(ARCH, 1, 911, 1, vec![]),
        TreeNode::with_defaults(ARCH, 0, 903, 1, vec![]),
        TreeNode::with_defaults(ARCH + 1, 0, 777, 1, vec![]),
    ])
}

/// AT-02 acceptance: the tree the client is shown is the catalog the
/// trainer gates on, in the same order. Each branch equals the catalog's
/// own `tree()` filtered to that branch, and every id in branch `i` is the
/// node `evaluate_train` plans with `tree_index == i`. Fails if `tree_info`
/// ever sorts, dedups, regroups or reads a source other than the catalog.
#[test]
fn tree_info_order_is_catalog_order_the_trainer_uses() {
    let catalog = scrambled_catalog();
    let data = tree_info(&catalog, ARCH, 42);

    for (branch, ids) in data.trees.iter().enumerate() {
        let catalog_order: Vec<i32> = catalog
            .tree(ARCH)
            .iter()
            .filter(|n| n.tree_index == branch as i32)
            .map(|n| n.ability_id)
            .collect();
        assert_eq!(ids, &catalog_order, "branch {branch}");

        let known = HashSet::new();
        for &ability_id in ids {
            let plan = evaluate_train(&TrainContext {
                catalog: &catalog,
                ability_id,
                ability_exists: true,
                player_id: Some(42),
                archetype_id: Some(ARCH),
                level: 1,
                known: &known,
                tree_points_spent: 0,
                // Enough to clear the spend gate: this test is about order.
                training_points: 100,
                // At a trainer offering the node, in range: only order matters here.
                trainer: TrainerPin::Trainer {
                    offered: std::slice::from_ref(&ability_id),
                    in_range: true,
                },
            })
            .expect("fixture node is trainable");
            assert_eq!(plan.tree_index, branch as i32, "ability {ability_id}");
        }
    }
    assert_eq!(
        data.trees,
        [vec![905, 901, 903], vec![912, 911], vec![930]],
        "pinned: catalog insertion order within each branch"
    );
}

/// An archetype with no rows gets three empty branches and exactly one
/// structured WARN on `abilities`; nothing is fabricated.
#[test]
fn tree_info_for_archetype_without_rows_is_empty_and_warns_once() {
    let catalog = scrambled_catalog();
    let capture = LogCapture::install();

    let data = tree_info(&catalog, 8, 77);

    assert!(data.trees.iter().all(Vec::is_empty), "no fabricated tree");
    let warn = capture
        .find_event(
            tracing::Level::WARN,
            "no ability-tree rows",
            "archetype_has_no_tree",
        )
        .expect("empty archetype must WARN");
    assert_eq!(warn.target, "abilities");
    assert!(warn.has_field("event", "tree_missing"), "{warn:?}");
    assert!(warn.has_field("archetype_id", "8"), "{warn:?}");
    assert!(warn.has_field("player_id", "77"), "{warn:?}");
    assert_eq!(
        capture
            .all()
            .iter()
            .filter(|c| c.level == tracing::Level::WARN)
            .count(),
        1,
        "exactly one WARN"
    );
}

/// A populated archetype logs nothing at WARN or above.
#[test]
fn tree_info_for_populated_archetype_is_silent() {
    let catalog = scrambled_catalog();
    let capture = LogCapture::install();

    let data = tree_info(&catalog, ARCH + 1, 77);

    assert_eq!(data.trees, [vec![777], vec![], vec![]]);
    assert!(
        capture.all().iter().all(|c| c.level > tracing::Level::WARN),
        "{:?}",
        capture.all()
    );
}

/// A node outside the three wire branches (impossible under
/// `tree_index_sanity`) is dropped with an ERROR, never pushed into a
/// neighbouring branch.
#[test]
fn tree_info_drops_out_of_range_branch_with_error() {
    let catalog = AbilityTreeCatalog::from_nodes([
        TreeNode::with_defaults(ARCH, 0, 901, 1, vec![]),
        TreeNode::with_defaults(ARCH, 3, 940, 1, vec![]),
    ]);
    let capture = LogCapture::install();

    let data = tree_info(&catalog, ARCH, 42);

    assert_eq!(data.trees, [vec![901], vec![], vec![]]);
    let err = capture
        .find_event(
            tracing::Level::ERROR,
            "outside the three",
            "tree_index_out_of_range",
        )
        .expect("out-of-range node must log ERROR");
    assert!(err.has_field("ability_id", "940"), "{err:?}");
}

/// Byte-exact `onAbilityTreeInfo(ARRAY<ARRAY<INT32>>)` args from a catalog.
/// Nodes are inserted out of branch order and out of id order: the payload
/// groups by branch and keeps catalog order inside each branch, with the
/// empty middle branch still on the wire as a zero count. The `mapLoaded`
/// record around these args is pinned in `cimmeria-wire`
/// (`world_data::tests::ability_tree_info`).
#[test]
fn tree_info_args_bytes_from_fixture_catalog() {
    let catalog = AbilityTreeCatalog::from_nodes([
        TreeNode::with_defaults(ARCH, 0, 30, 1, vec![]),
        TreeNode::with_defaults(ARCH, 2, 20, 1, vec![]),
        TreeNode::with_defaults(ARCH, 0, 10, 1, vec![]),
    ]);

    let args = tree_info(&catalog, ARCH, 42).serialize();

    #[rustfmt::skip]
    let expected: &[u8] = &[
        0x03, 0x00, 0x00, 0x00, // outer count: 3 branches
        0x02, 0x00, 0x00, 0x00, // branch 0: 2 ids
        0x1E, 0x00, 0x00, 0x00, //   30
        0x0A, 0x00, 0x00, 0x00, //   10
        0x00, 0x00, 0x00, 0x00, // branch 1: 0 ids
        0x01, 0x00, 0x00, 0x00, // branch 2: 1 id
        0x14, 0x00, 0x00, 0x00, //   20
    ];
    assert_eq!(args, expected);
}
