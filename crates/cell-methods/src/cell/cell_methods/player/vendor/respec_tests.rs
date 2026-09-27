//! AT-08: the cell's gate on `resetMyAbilities` (cell method 72).
//!
//! Bug shapes: a respec accepted away from a trainer (the method is
//! `<Exposed/>`, so any client can send it from anywhere), a refused press
//! that sends nothing (project rule: feedback on the first press), and a
//! replay that reaches the base. Every test drives the real dispatch from
//! method index 72, so the combat-range forward is covered too.

use tokio::sync::mpsc;

use crate::ability_tree::{
    AbilityTreeCatalog, TreeNode, RESPEC_COST_NAQUADAH, RESPEC_FEEDBACK_NOTHING_TRAINED,
    RESPEC_FEEDBACK_NOT_AT_TRAINER,
};
use crate::cell::client_methods::player::{ON_ERROR_CODE, ON_TRAINER_OPEN};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, seed_ability_defs};

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 100;
const TRAINER: u32 = 200;
const DIALOG_NPC: u32 = 201;
const TRAINER_TEMPLATE: i32 = 25;
const ARCH: i32 = 2;
const ROOT: i32 = 597;

/// A player who bought `ROOT` (1 point spent), pinned to a trainer 3 units
/// away (inside `MAX_INTERACT_DISTANCE`), plus a non-trainer NPC.
fn fixture() -> SpaceManager {
    let mut mgr = make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
        p.archetype_id = Some(ARCH);
        p.abilities.add_ability(ROOT);
        p.tree_progress.trained_abilities = vec![ROOT];
        p.tree_progress.tree_points_spent = 1;
        p.last_interaction_target = Some(TRAINER);
    }
    mgr.spawn_npc(TRAINER, "Agnos", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(TRAINER) {
        t.template_id = Some(TRAINER_TEMPLATE);
    }
    mgr.spawn_npc(DIALOG_NPC, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(n) = mgr.get_entity_mut(DIALOG_NPC) {
        n.template_id = Some(99);
    }
    mgr.template_trainer_lists.insert(TRAINER_TEMPLATE, 1);
    mgr.trainer_abilities.insert((1, ARCH), vec![ROOT]);
    seed_ability_defs(&mut mgr, &[ROOT]);
    mgr.ability_tree_catalog =
        AbilityTreeCatalog::from_nodes([TreeNode::with_defaults(ARCH, 0, ROOT, 1, vec![])]);
    mgr
}

/// One outbound message, reduced to what the tests compare.
#[derive(Debug, PartialEq, Eq)]
enum Sent {
    Reset { player_id: i32, cost: i32 },
    Method(u16, Vec<u8>),
}

/// Drive cell method 72 through the player dispatcher, as the wire does.
async fn respec(mgr: &mut SpaceManager) -> Vec<Sent> {
    let (tx, mut rx) = mpsc::channel(16);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let handled = crate::cell::cell_methods::player::dispatch(
        PLAYER,
        crate::cell::cell_methods::player::RESET_MY_ABILITIES,
        &[],
        &tx,
        mgr,
        &engine,
    )
    .await;
    assert!(handled, "method 72 is dispatched");
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(match msg {
            CellToBaseMsg::ResetAbilities {
                entity_id,
                player_id,
                cost,
            } => {
                assert_eq!(entity_id, PLAYER);
                Sent::Reset { player_id, cost }
            }
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(entity_id, PLAYER, "feedback goes to the player");
                Sent::Method(method_index, args)
            }
            other => panic!("unexpected message {other:?}"),
        });
    }
    out
}

/// `onErrorCode(0, 0, code)`: a respec names no ability.
fn error_bytes(code: u16) -> Vec<u8> {
    let mut b = vec![0u8, 0, 0, 0, 0];
    b.extend_from_slice(&code.to_le_bytes());
    b
}

fn methods(sent: &[Sent]) -> Vec<u16> {
    sent.iter()
        .map(|s| match s {
            Sent::Method(m, _) => *m,
            Sent::Reset { .. } => panic!("respec forwarded to the base: {sent:?}"),
        })
        .collect()
}

fn first_error(sent: &[Sent]) -> &[u8] {
    match &sent[0] {
        Sent::Method(ON_ERROR_CODE, args) => args,
        other => panic!("first message must be onErrorCode, got {other:?}"),
    }
}

#[tokio::test]
async fn respec_at_a_reachable_trainer_is_forwarded_with_the_price() {
    let mut mgr = fixture();
    assert_eq!(
        respec(&mut mgr).await,
        vec![Sent::Reset {
            player_id: PLAYER_ID,
            cost: RESPEC_COST_NAQUADAH,
        }]
    );
}

#[tokio::test]
async fn no_pin_rejects_with_error_and_no_resend() {
    let mut mgr = fixture();
    mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = None;
    let sent = respec(&mut mgr).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE]);
    assert_eq!(
        first_error(&sent),
        error_bytes(RESPEC_FEEDBACK_NOT_AT_TRAINER)
    );
}

#[tokio::test]
async fn pin_that_is_not_a_trainer_rejects_with_error_and_no_resend() {
    let mut mgr = fixture();
    mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = Some(DIALOG_NPC);
    let sent = respec(&mut mgr).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE]);
    assert_eq!(
        first_error(&sent),
        error_bytes(RESPEC_FEEDBACK_NOT_AT_TRAINER)
    );
}

#[tokio::test]
async fn despawned_trainer_rejects_with_error_and_no_resend() {
    let mut mgr = fixture();
    mgr.destroy_entity(TRAINER);
    let sent = respec(&mut mgr).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE]);
    assert_eq!(
        first_error(&sent),
        error_bytes(RESPEC_FEEDBACK_NOT_AT_TRAINER)
    );
}

#[tokio::test]
async fn out_of_range_trainer_rejects_then_resends_the_window() {
    let mut mgr = fixture();
    // Walk away: 10 units from the trainer, past MAX_INTERACT_DISTANCE.
    mgr.get_entity_mut(PLAYER).unwrap().position.x = -7.0;
    let sent = respec(&mut mgr).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE, ON_TRAINER_OPEN]);
    assert_eq!(
        first_error(&sent),
        error_bytes(RESPEC_FEEDBACK_NOT_AT_TRAINER)
    );
}

/// A replay after a successful respec (or a first press with nothing
/// bought) never reaches the base, and the press is still answered.
#[tokio::test]
async fn nothing_trained_rejects_then_resends_without_asking_the_base() {
    let mut mgr = fixture();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.tree_progress.trained_abilities.clear();
        p.tree_progress.tree_points_spent = 0;
    }
    let sent = respec(&mut mgr).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE, ON_TRAINER_OPEN]);
    assert_eq!(
        first_error(&sent),
        error_bytes(RESPEC_FEEDBACK_NOTHING_TRAINED)
    );
}

#[tokio::test]
async fn player_without_a_player_id_is_silent() {
    let mut mgr = fixture();
    mgr.get_entity_mut(PLAYER).unwrap().player_id = None;
    assert!(respec(&mut mgr).await.is_empty());
}

/// A double-click or a spamming client: the second press inside the retry
/// window never reaches the base, so a player short of naquadah (which the
/// cell cannot see) cannot drive one row-locking `UPDATE` per packet.
#[tokio::test]
async fn repeat_press_inside_the_retry_window_is_dropped() {
    let mut mgr = fixture();
    assert_eq!(respec(&mut mgr).await.len(), 1, "first press forwarded");
    assert!(
        respec(&mut mgr).await.is_empty(),
        "the repeat is neither forwarded nor answered: the first answer is on its way"
    );
}

#[tokio::test]
async fn press_after_the_retry_window_is_forwarded_again() {
    let mut mgr = fixture();
    mgr.get_entity_mut(PLAYER).unwrap().respec_requested_at =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(2));
    assert_eq!(
        respec(&mut mgr).await,
        vec![Sent::Reset {
            player_id: PLAYER_ID,
            cost: RESPEC_COST_NAQUADAH,
        }]
    );
}
