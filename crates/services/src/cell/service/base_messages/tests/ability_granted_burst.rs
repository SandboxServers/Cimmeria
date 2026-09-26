//! AT-03: what `AbilityGranted` and `ProgressionChanged` do to the cell and
//! the client.
//!
//! Bug shapes: the point counter staying stale after a purchase (audit
//! A-07: the base returned the new value and the cell only logged it), the
//! trainer re-send computing `trainable` from pre-purchase points, and the
//! cell's level/points drifting from the base after a level-up so a node
//! the base would sell is shown locked.

use super::*;
use crate::ability_tree::{AbilityTreeCatalog, TreeNode};
use crate::cell::messages::CellToBaseMsg;
use crate::mercury::method_idx;

const PLAYER: u32 = 1;
const TRAINER: u32 = 200;

/// Commando player at level 1 with 1 training point, trainer offering 597
/// and 646 (level 1) and 641 (level 5).
fn fixture(trainer_pinned: bool) -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(100);
        p.archetype_id = Some(2);
        p.level = 1;
        p.tree_progress.training_points = 1;
        p.last_interaction_target = trainer_pinned.then_some(TRAINER);
    }
    mgr.spawn_npc(TRAINER, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(TRAINER) {
        t.template_id = Some(25);
    }
    mgr.template_trainer_lists.insert(25, 1);
    mgr.trainer_abilities.insert((1, 2), vec![597, 646, 641]);
    crate::test_support::seed_ability_defs(&mut mgr, &[597, 646, 641]);
    mgr.ability_tree_catalog = AbilityTreeCatalog::from_nodes([
        TreeNode::with_defaults(2, 1, 597, 1, vec![]),
        TreeNode::with_defaults(2, 1, 646, 1, vec![]),
        TreeNode::with_defaults(2, 1, 641, 5, vec![]),
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
            assert_eq!(entity_id, PLAYER, "every burst frame targets the buyer");
            out.push((method_index, args));
        }
    }
    out
}

fn granted(ability_id: i32, training_points: i32, tree_points_spent: i32) -> BaseToCellMsg {
    BaseToCellMsg::AbilityGranted {
        entity_id: PLAYER,
        ability_id,
        training_points,
        tree_points_spent,
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

#[tokio::test]
async fn grant_burst_is_known_abilities_then_points_then_trainer_resend() {
    let mut mgr = fixture(true);
    let frames = deliver(&mut mgr, granted(597, 0, 1)).await;

    let order: Vec<u16> = frames.iter().map(|(m, _)| *m).collect();
    assert_eq!(
        order,
        vec![
            method_idx::ON_KNOWN_ABILITIES_UPDATE,
            method_idx::ON_ENTITY_PROPERTY,
            method_idx::ON_TRAINER_OPEN,
        ],
        "the point counter must reach the client before the trainer re-send"
    );
    assert_eq!(
        frames[1].1,
        vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        "onEntityProperty(GENERICPROPERTY_TrainingPoints = 1, 0)"
    );
    // The re-send reads the mirrored points: 646 costs 1 and the player now
    // has 0, so it is locked. With the pre-purchase point it would read 1.
    assert_eq!(trainable(&frames[2].1, 646), 0);
}

#[tokio::test]
async fn grant_without_a_pinned_trainer_still_refreshes_the_counter() {
    let mut mgr = fixture(false);
    let frames = deliver(&mut mgr, granted(597, 4, 1)).await;
    assert_eq!(
        frames,
        vec![
            (method_idx::ON_KNOWN_ABILITIES_UPDATE, frames[0].1.clone()),
            (
                method_idx::ON_ENTITY_PROPERTY,
                vec![0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00]
            ),
        ]
    );
}

#[tokio::test]
async fn grant_mirrors_provenance_points_and_spend_onto_the_cell() {
    let mut mgr = fixture(false);
    deliver(&mut mgr, granted(597, 3, 2)).await;
    // A duplicate grant (the base never sends one, but a replayed message
    // must not double the provenance entry).
    deliver(&mut mgr, granted(597, 3, 2)).await;
    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(p.abilities.has_ability(597));
    assert_eq!(
        p.tree_progress,
        cimmeria_entity::cell_entity::TreeProgress {
            trained_abilities: vec![597],
            tree_points_spent: 2,
            training_points: 3,
        }
    );
}

#[tokio::test]
async fn progression_changed_opens_a_node_without_a_relog() {
    let mut mgr = fixture(true);
    deliver(
        &mut mgr,
        BaseToCellMsg::ProgressionChanged {
            entity_id: PLAYER,
            level: 5,
            training_points: 4,
        },
    )
    .await;
    let p = mgr.get_entity(PLAYER).unwrap();
    assert_eq!((p.level, p.tree_progress.training_points), (5, 4));

    // The trainer, reopened, now offers the level-5 node.
    let (tx, mut rx) = mpsc::channel(4);
    assert!(crate::cell::interactions::try_open_trainer(PLAYER, TRAINER, &tx, &mgr).await);
    let Ok(CellToBaseMsg::EntityMethodCall { args, .. }) = rx.try_recv() else {
        panic!("expected onTrainerOpen");
    };
    assert_eq!(trainable(&args, 641), 1, "level 5 opens 641");
}

#[tokio::test]
async fn progression_changed_sends_nothing_to_the_client() {
    // The base already sent the level-up bundle; the cell only mirrors.
    let mut mgr = fixture(true);
    let frames = deliver(
        &mut mgr,
        BaseToCellMsg::ProgressionChanged {
            entity_id: PLAYER,
            level: 2,
            training_points: 2,
        },
    )
    .await;
    assert!(frames.is_empty());
}

/// The level gate reads the level `InitPlayerState` hydrates. On `main`
/// before AT-03 nothing stamped a player's cell level, so every player was
/// level 1 to the trainer and the level-5 node stayed locked for a level-5
/// character. Removing the stamp in the `InitPlayerState` arm fails this.
#[tokio::test]
async fn level_gate_reads_the_level_hydrated_at_world_entry() {
    let mut mgr = fixture(true);
    mgr.connect_entity(PLAYER);
    deliver(
        &mut mgr,
        BaseToCellMsg::InitPlayerState {
            entity_id: PLAYER,
            player_id: 100,
            account_id: 6,
            world_name: "Agnos".into(),
            archetype_id: 2,
            saved_missions: vec![],
            abilities: vec![],
            active_bandolier_slot: 0,
            bandolier_items: vec![],
            system_options: cimmeria_entity::cell_entity::SystemOptions::default(),
            state_field: 0,
            access_level: 0,
            known_stargates: vec![],
            tree_progress: cimmeria_entity::cell_entity::TreeProgress {
                trained_abilities: vec![],
                tree_points_spent: 0,
                training_points: 2,
            },
            level: 5,
            character_name: None,
            body_set: None,
        },
    )
    .await;

    let (tx, mut rx) = mpsc::channel(4);
    assert!(crate::cell::interactions::try_open_trainer(PLAYER, TRAINER, &tx, &mgr).await);
    let Ok(CellToBaseMsg::EntityMethodCall { args, .. }) = rx.try_recv() else {
        panic!("expected onTrainerOpen");
    };
    assert_eq!(
        trainable(&args, 641),
        1,
        "a level-5 character must be able to train the level-5 node"
    );
}
