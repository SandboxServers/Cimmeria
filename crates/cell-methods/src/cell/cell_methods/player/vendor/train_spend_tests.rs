//! AT-03 purchase-side tests for `handle_train_ability` (cell): the spend
//! gates stop a purchase before the base sees it, a passing purchase carries
//! the node's cost and branch, and a node with no authored `training_cost`
//! raises the `train_raw_cost_zero` WARN.

use tokio::sync::mpsc;
use tracing::Level;

use super::train::handle_train_ability;
use crate::ability_tree::{AbilityTreeCatalog, TreeNode};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, seed_ability_defs, LogCapture};

const PLAYER: u32 = 1;
const ARCH: i32 = 3;
const NODE: i32 = 5101;
const TRAINER: u32 = 200;

/// A level-10 player with `points` training points and `spent` spend, and
/// one node: branch 2, cost 2, gated on 4 points, raw cost `raw_cost`.
fn fixture(points: i32, spent: i32, raw_cost: i32) -> SpaceManager {
    let mut mgr = make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(100);
        p.archetype_id = Some(ARCH);
        p.level = 10;
        p.tree_progress.training_points = points;
        p.tree_progress.tree_points_spent = spent;
        p.last_interaction_target = Some(TRAINER);
    }
    // A reachable trainer offering the node, so only the spend gates decide
    // (AT-04's trainer gates run after them).
    mgr.spawn_npc(TRAINER, "Agnos", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(TRAINER) {
        t.template_id = Some(25);
    }
    mgr.template_trainer_lists.insert(25, 1);
    mgr.trainer_abilities.insert((1, ARCH), vec![NODE]);
    seed_ability_defs(&mut mgr, &[NODE]);
    let mut node = TreeNode::with_defaults(ARCH, 2, NODE, 5, vec![]);
    node.skill_point_cost = 2;
    node.required_branch_points = 4;
    node.raw_training_cost = raw_cost;
    mgr.ability_tree_catalog = AbilityTreeCatalog::from_nodes([node]);
    mgr
}

/// The `TrainAbility` forwarded to the base, if any. A rejection's
/// `onErrorCode` and trainer re-send (AT-04) are skipped.
async fn buy(mgr: &mut SpaceManager) -> Option<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(4);
    handle_train_ability(PLAYER, NODE, &tx, mgr).await;
    std::iter::from_fn(|| rx.try_recv().ok())
        .find(|m| matches!(m, CellToBaseMsg::TrainAbility { .. }))
}

#[tokio::test]
async fn purchase_carries_node_cost_and_branch_to_the_base() {
    let mut mgr = fixture(2, 4, 7);
    match buy(&mut mgr).await {
        Some(CellToBaseMsg::TrainAbility {
            ability_id,
            cost,
            tree_index,
            ..
        }) => assert_eq!((ability_id, cost, tree_index), (NODE, 2, 2)),
        _ => panic!("expected TrainAbility"),
    }
}

#[tokio::test]
async fn spend_gate_stops_the_purchase_before_the_base() {
    let capture = LogCapture::install();
    let mut mgr = fixture(9, 3, 7);
    assert!(buy(&mut mgr).await.is_none(), "3 of 4 points spent: locked");
    assert!(capture
        .find_event(Level::INFO, "archetype-wide spend", "spend_gate")
        .is_some());
}

#[tokio::test]
async fn too_few_points_stop_the_purchase_before_the_base() {
    let capture = LogCapture::install();
    let mut mgr = fixture(1, 4, 7);
    assert!(buy(&mut mgr).await.is_none(), "cost 2, 1 point");
    assert!(capture
        .find_event(
            Level::INFO,
            "not enough training points",
            "not_enough_points"
        )
        .is_some());
}

#[tokio::test]
async fn raw_cost_zero_purchase_warns_and_still_goes_ahead() {
    let capture = LogCapture::install();
    let mut mgr = fixture(2, 4, 0);
    assert!(matches!(
        buy(&mut mgr).await,
        Some(CellToBaseMsg::TrainAbility { cost: 2, .. })
    ));
    let warn = capture
        .find_message(Level::WARN, "raw training_cost 0")
        .expect("train_raw_cost_zero WARN");
    assert_eq!(warn.target, "abilities");
    assert!(warn.has_field("event", "train_raw_cost_zero"));
    assert!(warn.has_field("ability_id", &NODE.to_string()));
}

#[tokio::test]
async fn authored_cost_purchase_does_not_warn() {
    let capture = LogCapture::install();
    let mut mgr = fixture(2, 4, 7);
    assert!(buy(&mut mgr).await.is_some());
    assert!(capture
        .find_message(Level::WARN, "raw training_cost 0")
        .is_none());
}
