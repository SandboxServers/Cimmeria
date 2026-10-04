//! AB-N2: what `GmAbilitiesChanged` (the `gmResetAbilities` /
//! `gmGiveAllAbilities` mirror) does to the cell and the client.
//!
//! Bug shapes: a reset that leaves the cell knowing removed abilities or
//! the old tree spend (the trainer gates would then disagree with the row);
//! a give-all that sends one hotbar update per ability instead of one; and a
//! change mirrored onto whoever inherited a recycled entity id.

use super::*;
use crate::cell::messages::{CellToBaseMsg, GmAbilitiesChanged, GmAbilityChange};
use crate::mercury::method_idx;
use cimmeria_entity::cell_entity::TreeProgress;

const GM: u32 = 1;
const PLAYER_ID: i32 = 100;
const STARTERS: [i32; 2] = [592, 1646];
const TRAINED: [i32; 2] = [597, 646];
const QUEST: i32 = 2826;

fn fixture() -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(GM, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(GM).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.archetype_id = Some(2);
    for id in STARTERS.into_iter().chain(TRAINED).chain([QUEST]) {
        p.abilities.add_ability(id);
    }
    p.tree_progress = TreeProgress {
        trained_abilities: TRAINED.to_vec(),
        tree_points_spent: 2,
        training_points: 1,
    };
    crate::test_support::seed_ability_defs(&mut mgr, &[592, 597, 646, 1646, 2826, 700, 701]);
    mgr
}

async fn deliver(mgr: &mut SpaceManager, changed: GmAbilitiesChanged) -> Vec<(u16, Vec<u8>)> {
    let (tx, mut rx) = mpsc::channel(32);
    let engine = ChainEngine::new();
    handle_base_message(
        BaseToCellMsg::GmAbilitiesChanged(changed),
        &tx,
        mgr,
        &engine,
        &[],
    )
    .await;
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = m
        {
            assert_eq!(entity_id, GM, "every frame targets the GM");
            out.push((method_index, args));
        }
    }
    out
}

fn count(frames: &[(u16, Vec<u8>)], method: u16) -> usize {
    frames.iter().filter(|(m, _)| *m == method).count()
}

fn known(mgr: &SpaceManager) -> Vec<i32> {
    let mut ids = mgr.get_entity(GM).unwrap().abilities.known_ability_ids();
    ids.sort_unstable();
    ids
}

/// **Guard: reset leaves the starters.** The removed ids leave the known
/// set, the tree spend and provenance clear, the refunded points land, and
/// the client gets one known-abilities update, the point counter and the
/// result line.
#[tokio::test]
async fn gm_reset_mirror_leaves_the_starters_and_the_refund() {
    let mut mgr = fixture();
    let frames = deliver(
        &mut mgr,
        GmAbilitiesChanged {
            entity_id: GM,
            player_id: PLAYER_ID,
            change: GmAbilityChange::Reset,
            added: vec![],
            removed: vec![TRAINED[0], TRAINED[1], QUEST],
            training_points: 3,
        },
    )
    .await;

    assert_eq!(known(&mgr), STARTERS.to_vec());
    let progress = &mgr.get_entity(GM).unwrap().tree_progress;
    assert!(progress.trained_abilities.is_empty());
    assert_eq!(
        (progress.tree_points_spent, progress.training_points),
        (0, 3)
    );
    assert_eq!(count(&frames, method_idx::ON_KNOWN_ABILITIES_UPDATE), 1);
    assert_eq!(
        count(&frames, method_idx::ON_PLAYER_COMMUNICATION),
        1,
        "the GM's result line"
    );
    assert!(
        frames.len() >= 3,
        "known update, point counter and feedback: {:?}",
        frames.iter().map(|(m, _)| m).collect::<Vec<_>>()
    );
}

/// **Guard: give-all is one burst.** Two ids join the known set with one
/// `onKnownAbilitiesUpdate`, not one per id, and the tree progress is
/// untouched (a GM grant is not a purchase).
#[tokio::test]
async fn gm_give_all_mirror_adds_every_id_in_one_burst() {
    let mut mgr = fixture();
    let frames = deliver(
        &mut mgr,
        GmAbilitiesChanged {
            entity_id: GM,
            player_id: PLAYER_ID,
            change: GmAbilityChange::GrantAll,
            added: vec![700, 701],
            removed: vec![],
            training_points: 1,
        },
    )
    .await;

    let k = known(&mgr);
    assert!(k.contains(&700) && k.contains(&701), "{k:?}");
    assert_eq!(count(&frames, method_idx::ON_KNOWN_ABILITIES_UPDATE), 1);
    let progress = &mgr.get_entity(GM).unwrap().tree_progress;
    assert_eq!(progress.trained_abilities, TRAINED.to_vec());
    assert_eq!(progress.tree_points_spent, 2);
}

/// The entity id now plays another character: nothing changes, nothing is
/// sent.
#[tokio::test]
async fn gm_bulk_mirror_for_another_character_is_ignored() {
    let mut mgr = fixture();
    let before = known(&mgr);
    let frames = deliver(
        &mut mgr,
        GmAbilitiesChanged {
            entity_id: GM,
            player_id: PLAYER_ID + 1,
            change: GmAbilityChange::Reset,
            added: vec![],
            removed: vec![QUEST],
            training_points: 3,
        },
    )
    .await;
    assert!(frames.is_empty());
    assert_eq!(known(&mgr), before);
}
