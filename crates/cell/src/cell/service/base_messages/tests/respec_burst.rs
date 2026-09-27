//! AT-08: what `AbilitiesReset` does to the cell and the client.
//!
//! Bug shapes: the cell keeping the pre-respec spend (AT-03's spend gate
//! then opens nodes the base would refuse, `gates/spend.rs`), refunded
//! abilities staying known on the cell, the point counter or balance
//! staying stale, the trainer re-send computing `trainable` from the old
//! state, a warming cast of a refunded ability still firing (AT-10), and a
//! refusal from the base that reaches the player as silence.

use std::time::{Duration, Instant};

use super::*;
use crate::ability_tree::{AbilityTreeCatalog, RespecOutcome, TreeNode};
use crate::cell::messages::CellToBaseMsg;
use crate::mercury::method_idx;
use cimmeria_entity::cell_entity::{PendingCast, TreeProgress};

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 100;
const TRAINER: u32 = 200;
const STARTER: i32 = 1646;
const ROOT: i32 = 597;
const NODE: i32 = 646;
/// `onTimerUpdate`, sent twice by the warmup interrupt.
const ON_TIMER_UPDATE: u16 = 12;

/// Commando at level 1 who knows the starter and bought `ROOT` and `NODE`
/// (3 points spent, 0 left), pinned to a trainer in range that offers both.
fn fixture() -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
        p.archetype_id = Some(2);
        p.level = 1;
        for id in [STARTER, ROOT, NODE] {
            p.abilities.add_ability(id);
        }
        p.tree_progress = TreeProgress {
            trained_abilities: vec![ROOT, NODE],
            tree_points_spent: 3,
            training_points: 0,
        };
        p.last_interaction_target = Some(TRAINER);
    }
    mgr.spawn_npc(TRAINER, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(TRAINER) {
        t.template_id = Some(25);
    }
    mgr.template_trainer_lists.insert(25, 1);
    mgr.trainer_abilities.insert((1, 2), vec![ROOT, NODE]);
    crate::test_support::seed_ability_defs(&mut mgr, &[STARTER, ROOT, NODE]);
    // NODE needs ROOT and 1 point of archetype spend.
    let mut node = TreeNode::with_defaults(2, 1, NODE, 1, vec![ROOT]);
    node.required_branch_points = 1;
    mgr.ability_tree_catalog =
        AbilityTreeCatalog::from_nodes([TreeNode::with_defaults(2, 1, ROOT, 1, vec![]), node]);
    mgr
}

async fn deliver(mgr: &mut SpaceManager, outcome: RespecOutcome) -> Vec<(u16, Vec<u8>)> {
    deliver_as(mgr, PLAYER_ID, outcome).await
}

/// Deliver an `AbilitiesReset` the base addressed to character `player_id`.
async fn deliver_as(
    mgr: &mut SpaceManager,
    player_id: i32,
    outcome: RespecOutcome,
) -> Vec<(u16, Vec<u8>)> {
    let (tx, mut rx) = mpsc::channel(32);
    let engine = ChainEngine::new();
    let msg = BaseToCellMsg::AbilitiesReset {
        entity_id: PLAYER,
        player_id,
        outcome,
    };
    handle_base_message(msg, &tx, mgr, &engine, &[]).await;
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = m
        {
            assert_eq!(
                entity_id, PLAYER,
                "every frame targets the respeccing player"
            );
            out.push((method_index, args));
        }
    }
    out
}

fn reset() -> RespecOutcome {
    RespecOutcome::Reset {
        refunded: vec![ROOT, NODE],
        training_points: 3,
        naquadah: 500,
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
async fn respec_burst_is_known_abilities_points_cash_then_trainer_resend() {
    let mut mgr = fixture();
    let frames = deliver(&mut mgr, reset()).await;

    let order: Vec<u16> = frames.iter().map(|(m, _)| *m).collect();
    assert_eq!(
        order,
        vec![
            method_idx::ON_KNOWN_ABILITIES_UPDATE,
            method_idx::ON_ENTITY_PROPERTY,
            method_idx::ON_CASH_CHANGED,
            method_idx::ON_TRAINER_OPEN,
        ]
    );
    assert_eq!(
        frames[0].1,
        vec![0x01, 0x00, 0x00, 0x00, 0x6E, 0x06, 0x00, 0x00],
        "onKnownAbilitiesUpdate([1646]): only the starter survives"
    );
    assert_eq!(
        frames[1].1,
        vec![0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00],
        "onEntityProperty(GENERICPROPERTY_TrainingPoints = 1, 3)"
    );
    assert_eq!(
        frames[2].1,
        vec![0xF4, 0x01, 0x00, 0x00],
        "onCashChanged(500)"
    );
    // Cross-task (AT-03): with the spend reset and the points back, the
    // root is buyable again at once. NODE is not: its prerequisite is gone.
    assert_eq!(
        trainable(&frames[3].1, ROOT),
        1,
        "the root is buyable again"
    );
    assert_eq!(trainable(&frames[3].1, NODE), 0);
}

#[tokio::test]
async fn respec_mirrors_the_reset_onto_the_cell() {
    let mut mgr = fixture();
    deliver(&mut mgr, reset()).await;
    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(
        p.abilities.has_ability(STARTER),
        "non-trainer grants survive"
    );
    assert!(!p.abilities.has_ability(ROOT));
    assert!(!p.abilities.has_ability(NODE));
    assert_eq!(
        p.tree_progress,
        TreeProgress {
            trained_abilities: vec![],
            tree_points_spent: 0,
            training_points: 3,
        }
    );
}

#[tokio::test]
async fn respec_interrupts_a_warming_cast_of_a_refunded_ability_first() {
    let mut mgr = fixture();
    let (anchor, space_id) = {
        let p = mgr.get_entity(PLAYER).unwrap();
        (p.position, p.space_id)
    };
    mgr.get_entity_mut(PLAYER).unwrap().pending_cast = Some(PendingCast {
        ability_id: NODE,
        target_id: 0,
        ground: None,
        effect_seq: 1,
        fire_at: Instant::now() + Duration::from_secs(5),
        warmup_secs: 5.0,
        anchor,
        space_id,
        weapon_instance: None,
    });
    mgr.pending_casts.insert(PLAYER);

    let frames = deliver(&mut mgr, reset()).await;

    assert!(mgr.get_entity(PLAYER).unwrap().pending_cast.is_none());
    assert!(!mgr.pending_casts.contains(&PLAYER));
    let order: Vec<u16> = frames.iter().map(|(m, _)| *m).collect();
    assert_eq!(
        order,
        vec![
            ON_TIMER_UPDATE,
            ON_TIMER_UPDATE,
            method_idx::ON_KNOWN_ABILITIES_UPDATE,
            method_idx::ON_ENTITY_PROPERTY,
            method_idx::ON_CASH_CHANGED,
            method_idx::ON_TRAINER_OPEN,
        ],
        "the cancelled warmup reaches the client before the respec burst"
    );
}

#[tokio::test]
async fn respec_leaves_a_warming_cast_of_a_kept_ability_alone() {
    let mut mgr = fixture();
    let (anchor, space_id) = {
        let p = mgr.get_entity(PLAYER).unwrap();
        (p.position, p.space_id)
    };
    mgr.get_entity_mut(PLAYER).unwrap().pending_cast = Some(PendingCast {
        ability_id: STARTER,
        target_id: 0,
        ground: None,
        effect_seq: 1,
        fire_at: Instant::now() + Duration::from_secs(5),
        warmup_secs: 5.0,
        anchor,
        space_id,
        weapon_instance: None,
    });
    mgr.pending_casts.insert(PLAYER);

    let frames = deliver(&mut mgr, reset()).await;

    assert!(mgr.get_entity(PLAYER).unwrap().pending_cast.is_some());
    assert_eq!(frames[0].0, method_idx::ON_KNOWN_ABILITIES_UPDATE);
}

#[tokio::test]
async fn short_balance_sends_code_35_then_resend_and_changes_nothing() {
    let mut mgr = fixture();
    let before = mgr.get_entity(PLAYER).unwrap().tree_progress.clone();
    let frames = deliver(&mut mgr, RespecOutcome::NotEnoughNaquadah { naquadah: 10 }).await;

    assert_eq!(
        frames.iter().map(|(m, _)| *m).collect::<Vec<_>>(),
        vec![method_idx::ON_ERROR_CODE, method_idx::ON_TRAINER_OPEN]
    );
    assert_eq!(
        frames[0].1,
        vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x23, 0x00],
        "onErrorCode(Ability, 0, StatValueLessThan = 35)"
    );
    let p = mgr.get_entity(PLAYER).unwrap();
    assert_eq!(p.tree_progress, before);
    assert!(p.abilities.has_ability(ROOT));
}

#[tokio::test]
async fn nothing_to_reset_from_the_base_sends_code_167_then_resend() {
    let mut mgr = fixture();
    let frames = deliver(&mut mgr, RespecOutcome::NothingToReset).await;

    assert_eq!(
        frames.iter().map(|(m, _)| *m).collect::<Vec<_>>(),
        vec![method_idx::ON_ERROR_CODE, method_idx::ON_TRAINER_OPEN]
    );
    assert_eq!(
        frames[0].1,
        vec![0x00, 0x00, 0x00, 0x00, 0x00, 0xA7, 0x00],
        "onErrorCode(Ability, 0, EntityDoesNotHaveAbility = 167)"
    );
}

/// Bug shape: the base checked the session before its `UPDATE`; if the
/// entity id was reused by another character meanwhile, that character
/// must not lose abilities or get the first one's balance.
#[tokio::test]
async fn reset_for_another_character_is_ignored() {
    let mut mgr = fixture();
    let before = mgr.get_entity(PLAYER).unwrap().tree_progress.clone();
    let frames = deliver_as(&mut mgr, PLAYER_ID + 1, reset()).await;

    assert!(frames.is_empty(), "nothing is sent to the other character");
    let p = mgr.get_entity(PLAYER).unwrap();
    assert_eq!(p.tree_progress, before);
    assert!(p.abilities.has_ability(ROOT));
}
