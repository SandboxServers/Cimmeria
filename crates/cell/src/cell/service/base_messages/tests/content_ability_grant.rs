//! CS-01a: what `ContentAbilitiesGranted` (the content `grant_ability`
//! mirror) does to the cell and the client.
//!
//! Bug shapes: a grant the hotbar never learns about; a grant with no
//! visible line (OD-CS04); credit that never reaches the spend gate, so the
//! open trainer keeps a node greyed that the signature should open; a grant
//! that lands in `trained_abilities`; and a grant mirrored onto whoever
//! inherited a recycled entity id.

use cimmeria_entity::cell_entity::AbilityGrantKind;

use super::*;
use crate::ability_tree::{AbilityTreeCatalog, TreeNode};
use crate::cell::messages::{CellToBaseMsg, ContentAbilitiesGranted};
use crate::mercury::method_idx;

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 100;
const TRAINER: u32 = 200;
/// A Commando signature root, cost 2.
const SIGNATURE: i32 = 646;
/// A node gated on 2 archetype-wide points, nothing else.
const GATED: i32 = 641;

/// Level-5 Commando with 1 training point, nothing bought. The trainer
/// offers `GATED`, which needs 2 points of spend.
fn fixture() -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
        p.archetype_id = Some(2);
        p.level = 5;
        p.tree_progress.training_points = 1;
        p.last_interaction_target = Some(TRAINER);
    }
    mgr.spawn_npc(TRAINER, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(TRAINER) {
        t.template_id = Some(25);
    }
    mgr.template_trainer_lists.insert(25, 1);
    mgr.trainer_abilities.insert((1, 2), vec![GATED]);
    crate::test_support::seed_ability_defs(&mut mgr, &[SIGNATURE, GATED]);
    let mut signature = TreeNode::with_defaults(2, 0, SIGNATURE, 1, vec![]);
    signature.is_branch_root = true;
    signature.skill_point_cost = 2;
    let mut gated = TreeNode::with_defaults(2, 1, GATED, 1, vec![]);
    gated.required_branch_points = 2;
    mgr.ability_tree_catalog = AbilityTreeCatalog::from_nodes([signature, gated]);
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
            assert_eq!(entity_id, PLAYER, "every frame targets the player");
            out.push((method_index, args));
        }
    }
    out
}

fn granted(player_id: i32, learned: Vec<i32>, credited: Vec<i32>) -> BaseToCellMsg {
    BaseToCellMsg::ContentAbilitiesGranted(ContentAbilitiesGranted {
        entity_id: PLAYER,
        player_id,
        chain_id: 7_301,
        source_kind: AbilityGrantKind::Signature,
        learned,
        credited,
        converted: vec![],
        // The fixture's own values: nothing refunded.
        training_points: 1,
        tree_points_spent: 0,
    })
}

/// **Guard (review F1): a bought node converted by a grant.** The cell
/// drops it from `trained_abilities`, takes the base's refunded points and
/// spend, sends the point counter and a line, and keeps it known. Revert
/// the conversion mirror and `trained_abilities` still lists it, so a
/// respec would strip the free signature.
#[tokio::test]
async fn a_converted_purchase_leaves_trained_and_refunds_points() {
    let mut mgr = fixture();
    {
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        p.abilities.add_ability(SIGNATURE);
        p.tree_progress.trained_abilities = vec![SIGNATURE];
        p.tree_progress.tree_points_spent = 2;
    }
    let msg = BaseToCellMsg::ContentAbilitiesGranted(ContentAbilitiesGranted {
        entity_id: PLAYER,
        player_id: PLAYER_ID,
        chain_id: 7_301,
        source_kind: AbilityGrantKind::Signature,
        learned: vec![],
        credited: vec![SIGNATURE],
        converted: vec![SIGNATURE],
        training_points: 3,
        tree_points_spent: 0,
    });
    let frames = deliver(&mut mgr, msg).await;

    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(p.abilities.has_ability(SIGNATURE));
    assert!(p.tree_progress.trained_abilities.is_empty());
    assert_eq!(
        (
            p.tree_progress.training_points,
            p.tree_progress.tree_points_spent
        ),
        (3, 0)
    );
    assert_eq!(p.tree_progress.credited_grants, vec![SIGNATURE]);
    let order: Vec<u16> = frames.iter().map(|(m, _)| *m).collect();
    assert_eq!(
        order,
        vec![
            method_idx::ON_ENTITY_PROPERTY,
            method_idx::ON_PLAYER_COMMUNICATION,
            method_idx::ON_TRAINER_OPEN,
        ]
    );
    assert!(says(&frames[1].1, "is now yours for free"));
    // Effective spend is unchanged (0 trained + 2 credit), so the gated
    // node stays open.
    assert_eq!(trainable(&frames[2].1, GATED), 1);
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

fn says(args: &[u8], text: &str) -> bool {
    let needle: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    args.windows(needle.len()).any(|w| w == needle)
}

/// **Guard: the grant is learned, visible and credited.** Known set and
/// credit updated, not trained; the hotbar list, one "You have learned"
/// line, then the trainer re-send in which the signature's credit opens the
/// gated node. Drop the credit in the mirror (or in the spend gate) and
/// the node reads 0.
#[tokio::test]
async fn a_content_grant_is_learned_announced_and_opens_the_gated_node() {
    let mut mgr = fixture();
    let frames = deliver(
        &mut mgr,
        granted(PLAYER_ID, vec![SIGNATURE], vec![SIGNATURE]),
    )
    .await;

    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(p.abilities.has_ability(SIGNATURE));
    assert_eq!(p.tree_progress.credited_grants, vec![SIGNATURE]);
    assert!(p.tree_progress.trained_abilities.is_empty());
    assert_eq!(p.tree_progress.tree_points_spent, 0, "credit is not spend");

    let order: Vec<u16> = frames.iter().map(|(m, _)| *m).collect();
    assert_eq!(
        order,
        vec![
            method_idx::ON_KNOWN_ABILITIES_UPDATE,
            method_idx::ON_PLAYER_COMMUNICATION,
            method_idx::ON_TRAINER_OPEN,
        ]
    );
    assert!(says(&frames[1].1, "You have learned "), "a visible line");
    assert_eq!(
        trainable(&frames[2].1, GATED),
        1,
        "the credit opens the node"
    );
}

/// An already-known ability that just got its row: credit only. No hotbar
/// refresh and no line (nothing new to learn), but the trainer re-sends.
#[tokio::test]
async fn credit_for_a_known_ability_refreshes_only_the_trainer() {
    let mut mgr = fixture();
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .abilities
        .add_ability(SIGNATURE);
    let frames = deliver(&mut mgr, granted(PLAYER_ID, vec![], vec![SIGNATURE])).await;
    let order: Vec<u16> = frames.iter().map(|(m, _)| *m).collect();
    assert_eq!(order, vec![method_idx::ON_TRAINER_OPEN]);
    assert_eq!(trainable(&frames[0].1, GATED), 1);
}

/// A replay (nothing learned, credit already held) sends nothing.
#[tokio::test]
async fn a_replayed_grant_sends_nothing() {
    let mut mgr = fixture();
    deliver(
        &mut mgr,
        granted(PLAYER_ID, vec![SIGNATURE], vec![SIGNATURE]),
    )
    .await;
    let frames = deliver(&mut mgr, granted(PLAYER_ID, vec![], vec![SIGNATURE])).await;
    assert!(frames.is_empty());
    assert_eq!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .tree_progress
            .credited_grants,
        vec![SIGNATURE],
        "credit is not duplicated"
    );
}

/// The entity id now plays another character: nothing is mirrored.
#[tokio::test]
async fn a_grant_for_another_character_is_ignored() {
    let mut mgr = fixture();
    let frames = deliver(
        &mut mgr,
        granted(PLAYER_ID + 1, vec![SIGNATURE], vec![SIGNATURE]),
    )
    .await;
    assert!(frames.is_empty());
    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(!p.abilities.has_ability(SIGNATURE));
    assert!(p.tree_progress.credited_grants.is_empty());
}

#[test]
fn learned_line_names_the_ability_or_its_id() {
    use super::super::content_ability_grant::learned_line;
    assert_eq!(
        learned_line(Some("Stealth I"), 646),
        "You have learned Stealth I."
    );
    assert_eq!(
        learned_line(None, 646),
        "You have learned a new ability (#646)."
    );
}
