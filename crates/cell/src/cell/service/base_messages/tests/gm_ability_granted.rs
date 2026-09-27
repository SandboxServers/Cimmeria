//! PT-07: what `GmAbilityGranted` (the GM `.giveability` mirror) does to the
//! cell and the client.
//!
//! Bug shapes: a grant the hotbar never learns about until relog; a grant
//! that lands in `trained_abilities` (the respec gate would then count a GM
//! grant as a trainer purchase); and a grant mirrored onto whoever inherited
//! a recycled entity id.

use super::*;
use crate::ability_tree::{AbilityTreeCatalog, TreeNode};
use crate::cell::messages::CellToBaseMsg;
use crate::mercury::method_idx;

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 100;
const TRAINER: u32 = 200;

/// Level-5 Commando with 1 training point; the trainer offers 597 and 641,
/// where 641 needs 597.
fn fixture(trainer_pinned: bool) -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
        p.archetype_id = Some(2);
        p.level = 5;
        p.tree_progress.training_points = 1;
        p.last_interaction_target = trainer_pinned.then_some(TRAINER);
    }
    mgr.spawn_npc(TRAINER, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(TRAINER) {
        t.template_id = Some(25);
    }
    mgr.template_trainer_lists.insert(25, 1);
    mgr.trainer_abilities.insert((1, 2), vec![597, 641]);
    crate::test_support::seed_ability_defs(&mut mgr, &[597, 641]);
    mgr.ability_tree_catalog = AbilityTreeCatalog::from_nodes([
        TreeNode::with_defaults(2, 1, 597, 1, vec![]),
        TreeNode::with_defaults(2, 1, 641, 1, vec![597]),
    ]);
    mgr
}

async fn deliver(mgr: &mut SpaceManager, msg: BaseToCellMsg) -> Vec<(u16, Vec<u8>)> {
    let (tx, mut rx) = mpsc::channel(32);
    let engine = ChainEngine::new();
    handle_base_message(msg, &tx, mgr, &engine, &[]).await;
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = m
        {
            assert_eq!(entity_id, PLAYER, "every frame targets the subject");
            out.push((method_index, args));
        }
    }
    out
}

fn gm_granted(player_id: i32, ability_id: i32) -> BaseToCellMsg {
    BaseToCellMsg::GmAbilityGranted {
        entity_id: PLAYER,
        player_id,
        ability_id,
    }
}

/// `trainable` byte for `ability_id` in an `onTrainerOpen` payload.
fn trainable(args: &[u8], ability_id: i32) -> u8 {
    let count = u32::from_le_bytes(args[4..8].try_into().unwrap()) as usize;
    (0..count)
        .map(|i| 8 + i * 5)
        .find(|&at| i32::from_le_bytes(args[at..at + 4].try_into().unwrap()) == ability_id)
        .map(|at| args[at + 4])
        .expect("ability offered")
}

/// The ability joins the known set only, and the hotbar list goes out. No
/// point counter: nothing about the points changed.
#[tokio::test]
async fn gm_grant_mirrors_the_known_set_only_and_refreshes_the_hotbar() {
    let mut mgr = fixture(false);
    let frames = deliver(&mut mgr, gm_granted(PLAYER_ID, 2826)).await;

    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(p.abilities.has_ability(2826), "the cell knows the grant");
    assert!(
        p.tree_progress.trained_abilities.is_empty(),
        "a GM grant is not a trainer purchase"
    );
    assert_eq!(p.tree_progress.training_points, 1, "points unchanged");
    let order: Vec<u16> = frames.iter().map(|(m, _)| *m).collect();
    assert_eq!(order, vec![method_idx::ON_KNOWN_ABILITIES_UPDATE]);
    let payload = &frames[0].1;
    assert!(
        payload
            .windows(4)
            .any(|w| w == 2826i32.to_le_bytes().as_slice()),
        "onKnownAbilitiesUpdate must carry 2826: {payload:?}"
    );
}

/// The entity id now plays another character: nothing is mirrored and
/// nothing is sent.
#[tokio::test]
async fn gm_grant_for_another_character_is_ignored() {
    let mut mgr = fixture(false);
    let frames = deliver(&mut mgr, gm_granted(PLAYER_ID + 1, 2826)).await;
    assert!(frames.is_empty(), "no hotbar refresh for a stranger");
    assert!(!mgr.get_entity(PLAYER).unwrap().abilities.has_ability(2826));
}

/// Granting a prerequisite with a trainer open re-sends the trainer, and the
/// dependent node reads trainable at once.
#[tokio::test]
async fn gm_grant_of_a_prerequisite_unlocks_the_open_trainer() {
    let mut mgr = fixture(true);
    let frames = deliver(&mut mgr, gm_granted(PLAYER_ID, 597)).await;
    let order: Vec<u16> = frames.iter().map(|(m, _)| *m).collect();
    assert_eq!(
        order,
        vec![
            method_idx::ON_KNOWN_ABILITIES_UPDATE,
            method_idx::ON_TRAINER_OPEN
        ]
    );
    assert_eq!(trainable(&frames[1].1, 641), 1, "597 known opens 641");
}
