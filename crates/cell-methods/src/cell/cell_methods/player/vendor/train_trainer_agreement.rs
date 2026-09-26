//! The trainer's `trainable` byte and the `trainAbility` purchase decision
//! must agree for every offered node (audit A-23/A-27). The client enables a
//! Train button from the byte alone, so a node shown trainable that the
//! purchase gate rejects is a button that does nothing.
//!
//! Fixture: one node per rejection reason plus one that passes. Both sides
//! are driven through their real entry points.

use tokio::sync::mpsc;

use super::train::handle_train_ability;
use crate::ability_tree::{AbilityTreeCatalog, TreeNode};
use crate::cell::interactions::try_open_trainer;
use crate::cell::messages::CellToBaseMsg;
use crate::test_support::{make_space_manager, seed_ability_defs};

const PLAYER: u32 = 1;
const TRAINER: u32 = 200;
const ARCH: i32 = 2;

const PASSES: i32 = 5001;
const UNKNOWN_DEF: i32 = 5002; // in the tree, no AbilityDef
const KNOWN: i32 = 5003; // already known
const OTHER_ARCH: i32 = 5004; // only in another archetype's tree
const HIGH_LEVEL: i32 = 5005; // needs level 10
const NEEDS_PREREQ: i32 = 5006; // needs 5999, not known
const SPEND_LOCKED: i32 = 5007; // needs 4 points spent, player has spent 2
const TOO_EXPENSIVE: i32 = 5008; // costs 5, player has 3

const OFFERED: [i32; 8] = [
    PASSES,
    UNKNOWN_DEF,
    KNOWN,
    OTHER_ARCH,
    HIGH_LEVEL,
    NEEDS_PREREQ,
    SPEND_LOCKED,
    TOO_EXPENSIVE,
];

fn fixture() -> crate::cell::space_manager::SpaceManager {
    let mut mgr = make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(100);
        p.archetype_id = Some(ARCH);
        p.level = 5;
        p.abilities.add_ability(KNOWN);
        p.tree_progress.tree_points_spent = 2;
        p.tree_progress.training_points = 3;
    }
    mgr.spawn_npc(TRAINER, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(TRAINER) {
        t.template_id = Some(25);
    }
    mgr.template_trainer_lists.insert(25, 1);
    mgr.trainer_abilities.insert((1, ARCH), OFFERED.to_vec());
    seed_ability_defs(
        &mut mgr,
        &[
            PASSES,
            KNOWN,
            OTHER_ARCH,
            HIGH_LEVEL,
            NEEDS_PREREQ,
            SPEND_LOCKED,
            TOO_EXPENSIVE,
        ],
    );
    let mut spend_locked = TreeNode::with_defaults(ARCH, 2, SPEND_LOCKED, 1, vec![]);
    spend_locked.required_branch_points = 4;
    let mut too_expensive = TreeNode::with_defaults(ARCH, 2, TOO_EXPENSIVE, 1, vec![]);
    too_expensive.skill_point_cost = 5;
    mgr.ability_tree_catalog = AbilityTreeCatalog::from_nodes([
        TreeNode::with_defaults(ARCH, 0, PASSES, 1, vec![]),
        TreeNode::with_defaults(ARCH, 0, UNKNOWN_DEF, 1, vec![]),
        TreeNode::with_defaults(ARCH, 0, KNOWN, 1, vec![]),
        TreeNode::with_defaults(ARCH + 1, 0, OTHER_ARCH, 1, vec![]),
        TreeNode::with_defaults(ARCH, 1, HIGH_LEVEL, 10, vec![]),
        TreeNode::with_defaults(ARCH, 1, NEEDS_PREREQ, 1, vec![5999]),
        spend_locked,
        too_expensive,
    ]);
    mgr
}

/// `(ability_id, trainable)` pairs from one `onTrainerOpen`.
async fn trainer_bytes() -> Vec<(i32, u8)> {
    let mgr = fixture();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(try_open_trainer(PLAYER, TRAINER, &tx, &mgr).await);
    let Ok(CellToBaseMsg::EntityMethodCall { args, .. }) = rx.try_recv() else {
        panic!("expected onTrainerOpen");
    };
    let count = u32::from_le_bytes(args[4..8].try_into().unwrap()) as usize;
    (0..count)
        .map(|i| {
            let at = 8 + i * 5;
            (
                i32::from_le_bytes(args[at..at + 4].try_into().unwrap()),
                args[at + 4],
            )
        })
        .collect()
}

async fn purchase_forwarded(ability_id: i32) -> bool {
    let mut mgr = fixture();
    let (tx, mut rx) = mpsc::channel(8);
    handle_train_ability(PLAYER, ability_id, &tx, &mut mgr).await;
    matches!(rx.try_recv(), Ok(CellToBaseMsg::TrainAbility { .. }))
}

#[tokio::test]
async fn trainable_byte_matches_purchase_decision_for_every_node() {
    let bytes = trainer_bytes().await;
    assert_eq!(
        bytes.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        OFFERED.to_vec(),
        "trainer lists every offered node in offer order"
    );
    for (ability_id, trainable) in bytes {
        let forwarded = purchase_forwarded(ability_id).await;
        assert_eq!(
            trainable == 1,
            forwarded,
            "ability {ability_id}: trainable byte {trainable} disagrees with purchase \
             decision (forwarded = {forwarded})"
        );
    }
    // Exactly one node passes; the fixture would be vacuous otherwise.
    assert!(purchase_forwarded(PASSES).await);
    assert!(!purchase_forwarded(UNKNOWN_DEF).await);
}
