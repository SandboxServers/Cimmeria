//! AT-04: trainer authority on `trainAbility` and the rejection feedback.
//!
//! Every test drives the real handler, so the pin resolution
//! (`cell::interactions::trainer_pin`), the gates and the feedback are
//! exercised together. The wire checks are byte-exact on the seven
//! `onErrorCode` bytes and on the method order.

use tokio::sync::mpsc;

use super::train::handle_train_ability;
use super::train_feedback::{
    error_code, error_code_args, ENTITY_DOES_NOT_HAVE_ABILITY, LEVEL_GREATER_THAN_OR_EQUAL,
    NOT_SPECIFIED_ARCHETYPE, OUTSIDE_DISTANCE_CHECK, STAT_VALUE_LESS_THAN,
};
use crate::ability_tree::{AbilityTreeCatalog, TrainReject, TreeNode};
use crate::cell::client_methods::player::{ON_ERROR_CODE, ON_TRAINER_OPEN};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, seed_ability_defs};

const PLAYER: u32 = 1;
const TRAINER: u32 = 200;
const DIALOG_NPC: u32 = 201;
const TRAINER_TEMPLATE: i32 = 25;
const ARCH: i32 = 2;

const OFFERED: i32 = 597; // offered, level 1, no prereqs: trainable
const NOT_OFFERED: i32 = 598; // in the tree, not on the trainer's list
const HIGH_LEVEL: i32 = 599; // offered, needs level 10
const NEEDS_PREREQ: i32 = 600; // offered, needs 5999
const OTHER_ARCH: i32 = 601; // offered, but only in another archetype's tree

/// Player at the origin pinned to a trainer 3 units away (inside
/// `MAX_INTERACT_DISTANCE`), plus a non-trainer NPC.
fn fixture() -> SpaceManager {
    populate(make_space_manager(), "Agnos")
}

/// [`fixture`] with the player in `player_world`; the NPCs stay in Agnos.
fn populate(mut mgr: SpaceManager, player_world: &str) -> SpaceManager {
    mgr.create_entity(PLAYER, player_world, [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(100);
        p.archetype_id = Some(ARCH);
        p.level = 1;
        p.tree_progress.training_points = 1;
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
    mgr.trainer_abilities.insert(
        (1, ARCH),
        vec![OFFERED, HIGH_LEVEL, NEEDS_PREREQ, OTHER_ARCH],
    );
    seed_ability_defs(
        &mut mgr,
        &[OFFERED, NOT_OFFERED, HIGH_LEVEL, NEEDS_PREREQ, OTHER_ARCH],
    );
    mgr.ability_tree_catalog = AbilityTreeCatalog::from_nodes([
        TreeNode::with_defaults(ARCH, 0, OFFERED, 1, vec![]),
        TreeNode::with_defaults(ARCH, 0, NOT_OFFERED, 1, vec![]),
        TreeNode::with_defaults(ARCH, 1, HIGH_LEVEL, 10, vec![]),
        TreeNode::with_defaults(ARCH, 1, NEEDS_PREREQ, 1, vec![5999]),
        TreeNode::with_defaults(ARCH + 1, 0, OTHER_ARCH, 1, vec![]),
    ]);
    mgr
}

/// One outbound message, reduced to what the tests compare.
#[derive(Debug, PartialEq, Eq)]
enum Sent {
    Train(i32),
    Method(u16, Vec<u8>),
}

async fn train(mgr: &mut SpaceManager, ability_id: i32) -> Vec<Sent> {
    let (tx, mut rx) = mpsc::channel(16);
    handle_train_ability(PLAYER, ability_id, &tx, mgr).await;
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(match msg {
            CellToBaseMsg::TrainAbility { ability_id, .. } => Sent::Train(ability_id),
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(entity_id, PLAYER, "feedback goes to the buyer");
                Sent::Method(method_index, args)
            }
            other => panic!("unexpected message {other:?}"),
        });
    }
    out
}

fn methods(sent: &[Sent]) -> Vec<u16> {
    sent.iter()
        .map(|s| match s {
            Sent::Method(m, _) => *m,
            Sent::Train(_) => panic!("purchase forwarded: {sent:?}"),
        })
        .collect()
}

/// `onErrorCode(0, ability_id, code)` exactly as the client parses it.
fn error_bytes(ability_id: i32, code: u16) -> Vec<u8> {
    let mut b = vec![0u8];
    b.extend_from_slice(&ability_id.to_le_bytes());
    b.extend_from_slice(&code.to_le_bytes());
    b
}

fn first_error(sent: &[Sent]) -> &[u8] {
    match &sent[0] {
        Sent::Method(ON_ERROR_CODE, args) => args,
        other => panic!("first message must be onErrorCode, got {other:?}"),
    }
}

// --- trainer authority, one test per rejection --------------------------

#[tokio::test]
async fn purchase_at_a_reachable_trainer_is_forwarded() {
    let mut mgr = fixture();
    assert_eq!(train(&mut mgr, OFFERED).await, vec![Sent::Train(OFFERED)]);
}

#[tokio::test]
async fn no_pin_rejects_with_error_and_no_resend() {
    let mut mgr = fixture();
    mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = None;
    let sent = train(&mut mgr, OFFERED).await;
    assert_eq!(
        methods(&sent),
        vec![ON_ERROR_CODE],
        "nothing pinned to re-send"
    );
    assert_eq!(
        first_error(&sent),
        error_bytes(OFFERED, OUTSIDE_DISTANCE_CHECK)
    );
}

#[tokio::test]
async fn pin_that_is_not_a_trainer_rejects_with_error_and_no_resend() {
    let mut mgr = fixture();
    mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = Some(DIALOG_NPC);
    let sent = train(&mut mgr, OFFERED).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE]);
    assert_eq!(
        first_error(&sent),
        error_bytes(OFFERED, OUTSIDE_DISTANCE_CHECK)
    );
}

#[tokio::test]
async fn despawned_trainer_rejects_with_error_and_no_resend() {
    let mut mgr = fixture();
    mgr.destroy_entity(TRAINER);
    assert!(mgr.get_entity(TRAINER).is_none(), "fixture: trainer gone");
    let sent = train(&mut mgr, OFFERED).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE]);
    assert_eq!(
        first_error(&sent),
        error_bytes(OFFERED, OUTSIDE_DISTANCE_CHECK)
    );
}

#[tokio::test]
async fn node_the_trainer_does_not_offer_is_rejected_then_resent() {
    let mut mgr = fixture();
    let sent = train(&mut mgr, NOT_OFFERED).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE, ON_TRAINER_OPEN]);
    assert_eq!(
        first_error(&sent),
        error_bytes(NOT_OFFERED, OUTSIDE_DISTANCE_CHECK)
    );
}

#[tokio::test]
async fn out_of_range_trainer_is_rejected_then_resent_all_greyed() {
    let mut mgr = fixture();
    // Walk away: 10 units from the trainer, past MAX_INTERACT_DISTANCE.
    mgr.get_entity_mut(PLAYER).unwrap().position.x = -7.0;
    let sent = train(&mut mgr, OFFERED).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE, ON_TRAINER_OPEN]);
    assert_eq!(
        first_error(&sent),
        error_bytes(OFFERED, OUTSIDE_DISTANCE_CHECK)
    );
    // The re-sent window greys every node, the purchase gate's verdict.
    let Sent::Method(_, open) = &sent[1] else {
        unreachable!()
    };
    let count = u32::from_le_bytes(open[4..8].try_into().unwrap()) as usize;
    assert_eq!(count, 4);
    for i in 0..count {
        assert_eq!(
            open[8 + i * 5 + 4],
            0,
            "entry {i} must be greyed out of range"
        );
    }
}

/// A trainer in another space at in-range coordinates must not count as
/// reachable: positions are per-space, and `get_entity` searches every space.
#[tokio::test]
async fn trainer_in_another_space_is_out_of_range() {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /><Space WorldName="Castle" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    let mut mgr = populate(mgr, "Castle");
    assert_ne!(
        mgr.get_entity_space_id(PLAYER),
        mgr.get_entity_space_id(TRAINER),
        "fixture: player and trainer in different spaces"
    );
    let sent = train(&mut mgr, OFFERED).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE, ON_TRAINER_OPEN]);
    assert_eq!(
        first_error(&sent),
        error_bytes(OFFERED, OUTSIDE_DISTANCE_CHECK)
    );
}

// --- the node-gate error codes, byte-exact ------------------------------

/// AT-03's points gate through the real handler: code 35, then the re-send.
#[tokio::test]
async fn not_enough_points_sends_code_35_then_resend() {
    let mut mgr = fixture();
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .tree_progress
        .training_points = 0;
    let sent = train(&mut mgr, OFFERED).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE, ON_TRAINER_OPEN]);
    assert_eq!(
        first_error(&sent),
        [0, 0x55, 0x02, 0, 0, 0x23, 0x00],
        "onErrorCode(SystemID 0, InstanceID 597, ErrorCodeID 35)"
    );
}

#[tokio::test]
async fn level_too_low_sends_code_9_then_resend() {
    let mut mgr = fixture();
    let sent = train(&mut mgr, HIGH_LEVEL).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE, ON_TRAINER_OPEN]);
    assert_eq!(
        first_error(&sent),
        [0, 0x57, 0x02, 0, 0, 0x09, 0x00],
        "onErrorCode(SystemID 0, InstanceID 599, ErrorCodeID 9)"
    );
}

#[tokio::test]
async fn missing_prerequisite_sends_code_167_then_resend() {
    let mut mgr = fixture();
    let sent = train(&mut mgr, NEEDS_PREREQ).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE, ON_TRAINER_OPEN]);
    assert_eq!(
        first_error(&sent),
        [0, 0x58, 0x02, 0, 0, 0xA7, 0x00],
        "onErrorCode(SystemID 0, InstanceID 600, ErrorCodeID 167)"
    );
}

#[tokio::test]
async fn wrong_archetype_sends_code_6_then_resend() {
    let mut mgr = fixture();
    let sent = train(&mut mgr, OTHER_ARCH).await;
    assert_eq!(methods(&sent), vec![ON_ERROR_CODE, ON_TRAINER_OPEN]);
    assert_eq!(
        first_error(&sent),
        [0, 0x59, 0x02, 0, 0, 0x06, 0x00],
        "onErrorCode(SystemID 0, InstanceID 601, ErrorCodeID 6)"
    );
}

#[test]
fn trainer_rejection_code_43_bytes() {
    assert_eq!(
        error_code_args(597, OUTSIDE_DISTANCE_CHECK),
        [0, 0x55, 0x02, 0, 0, 0x2B, 0x00],
        "onErrorCode(SystemID 0, InstanceID 597, ErrorCodeID 43)"
    );
}

/// The whole AT-E1 mapping in one place. A variant whose code changes
/// fails here before it reaches a player.
#[test]
fn error_code_mapping_matches_at_e1() {
    let cases = [
        (TrainReject::AlreadyKnown, None),
        (TrainReject::UnknownAbility, None),
        (TrainReject::NoPlayerId, None),
        (TrainReject::NoArchetype, None),
        (TrainReject::NotInArchetypeTree, Some(6)),
        (
            TrainReject::LevelTooLow {
                required: 2,
                actual: 1,
            },
            Some(9),
        ),
        (TrainReject::MissingPrerequisite { missing: 1 }, Some(167)),
        (
            TrainReject::SpendGate {
                required: 4,
                spent: 0,
            },
            Some(35),
        ),
        (
            TrainReject::NotEnoughPoints {
                cost: 1,
                available: 0,
            },
            Some(35),
        ),
        (TrainReject::NoTrainerPinned, Some(43)),
        (TrainReject::TrainerDespawned, Some(43)),
        (TrainReject::PinNotATrainer, Some(43)),
        (TrainReject::NotOfferedByTrainer, Some(43)),
        (TrainReject::TrainerOutOfRange, Some(43)),
    ];
    for (reject, code) in cases {
        assert_eq!(error_code(&reject), code, "{}", reject.reason());
    }
    assert_eq!(STAT_VALUE_LESS_THAN, 35);
    assert_eq!(
        (
            NOT_SPECIFIED_ARCHETYPE,
            LEVEL_GREATER_THAN_OR_EQUAL,
            ENTITY_DOES_NOT_HAVE_ABILITY
        ),
        (6, 9, 167)
    );
}

// --- silence and ordering ------------------------------------------------

/// A replayed purchase of a known ability sends nothing at all: no
/// forward, no error code, no trainer re-send. Holds out of range too,
/// because "already known" outranks the trainer gates.
#[tokio::test]
async fn duplicate_purchase_is_silent() {
    let mut mgr = fixture();
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .abilities
        .add_ability(OFFERED);
    assert_eq!(train(&mut mgr, OFFERED).await, vec![]);
    mgr.get_entity_mut(PLAYER).unwrap().position.x = -7.0;
    assert_eq!(train(&mut mgr, OFFERED).await, vec![], "far away too");
}

/// An ability id with no definition is a forged packet: no code names it and
/// it earns no trainer re-send (each re-send is a full `onTrainerOpen`).
#[tokio::test]
async fn unknown_ability_id_is_silent() {
    let mut mgr = fixture();
    assert_eq!(train(&mut mgr, 424_242).await, vec![]);
}

/// The error code precedes the re-send, and the re-send is the pinned
/// trainer's full `onTrainerOpen`: TrainerID is the pin, and the one
/// trainable node stays trainable.
#[tokio::test]
async fn rejection_is_followed_by_the_pinned_trainer_resend() {
    let mut mgr = fixture();
    let sent = train(&mut mgr, HIGH_LEVEL).await;
    assert_eq!(sent.len(), 2, "{sent:?}");
    let Sent::Method(ON_TRAINER_OPEN, open) = &sent[1] else {
        panic!("second message must be onTrainerOpen: {sent:?}");
    };
    assert_eq!(
        i32::from_le_bytes(open[0..4].try_into().unwrap()),
        TRAINER as i32
    );
    let entries: Vec<(i32, u8)> = (0..4)
        .map(|i| {
            let at = 8 + i * 5;
            (
                i32::from_le_bytes(open[at..at + 4].try_into().unwrap()),
                open[at + 4],
            )
        })
        .collect();
    assert_eq!(
        entries,
        vec![
            (OFFERED, 1),
            (HIGH_LEVEL, 0),
            (NEEDS_PREREQ, 0),
            (OTHER_ARCH, 0)
        ]
    );
}
