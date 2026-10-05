//! AB-N2: what `GmAbilitiesChanged` (the `gmResetAbilities` /
//! `gmGiveAllAbilities` mirror) does to the cell and the client.
//!
//! Bug shapes: a reset that leaves the cell knowing removed abilities or
//! the old tree spend (the trainer gates would then disagree with the row);
//! a give-all that sends one hotbar update per ability instead of one; and a
//! change mirrored onto whoever inherited a recycled entity id.

use super::*;
use crate::cell::messages::{CellToBaseMsg, GmAbilitiesChanged, GmAbilityChange, GmAbilitySource};
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
            source: GmAbilitySource::Command,
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
            source: GmAbilitySource::Command,
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

/// DA-02 review F4: a grant from the Debug Area granter NPC leads its
/// result line with the NPC, not with a command the GM never typed.
/// Revert proof: label every source as its command and the line says
/// "gmGiveAllAbilities".
#[tokio::test]
async fn an_npc_granter_grant_is_labelled_as_the_granter() {
    let utf16 = |s: &str| -> Vec<u8> { s.encode_utf16().flat_map(u16::to_le_bytes).collect() };
    let says = |frames: &[(u16, Vec<u8>)], text: &str| {
        let needle = utf16(text);
        frames.iter().any(|(m, a)| {
            *m == method_idx::ON_PLAYER_COMMUNICATION
                && a.windows(needle.len()).any(|w| w == needle)
        })
    };
    let mut mgr = fixture();
    let frames = deliver(
        &mut mgr,
        GmAbilitiesChanged {
            entity_id: GM,
            player_id: PLAYER_ID,
            change: GmAbilityChange::GrantAll,
            source: GmAbilitySource::NpcGranter,
            added: vec![700, 701],
            removed: vec![],
            training_points: 1,
        },
    )
    .await;
    assert!(says(&frames, "Ability granter: granted 2 abilities"));
    assert!(!says(&frames, "gmGiveAllAbilities"));
}

/// **Guard: a reset keeps the equipped weapon's abilities.** The row holds
/// the pistol shot (the player knew it before equipping, so the weapon
/// never tagged it), the reset removes it from the row, and the active
/// slot's grant puts it back, tagged so the next unequip revokes it. On
/// revert the player is left unable to fire the equipped pistol.
#[tokio::test]
async fn gm_reset_mirror_regrants_the_equipped_weapons_abilities() {
    use cimmeria_entity::cell_entity::BandolierItem;
    // Sentinel item and ability ids: the binding is the fixture's own.
    const ITEM: i32 = 0x7032_0A01;
    const SHOT: i32 = 0x7032_0A02;
    let mut mgr = fixture();
    mgr.item_event_set_abilities
        .insert((ITEM, crate::cell::spawner::EVENT_ITEM_RANGED), SHOT);
    let p = mgr.get_entity_mut(GM).unwrap();
    p.abilities.add_ability(SHOT);
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 0,
            item_id: ITEM,
            clip_size: 12,
            default_ammo_type: 1,
            current_ammo: 12,
            cur_ammo_type: 1,
        },
    );
    p.active_bandolier_slot = 0;

    let frames = deliver(
        &mut mgr,
        GmAbilitiesChanged {
            entity_id: GM,
            player_id: PLAYER_ID,
            change: GmAbilityChange::Reset,
            source: GmAbilitySource::Command,
            added: vec![],
            removed: vec![TRAINED[0], TRAINED[1], QUEST, SHOT],
            training_points: 3,
        },
    )
    .await;

    let p = mgr.get_entity(GM).unwrap();
    assert!(p.abilities.has_ability(SHOT), "the pistol shot is back");
    assert_eq!(p.abilities.weapon_granted_ability_ids(), vec![SHOT]);
    assert!(
        !p.abilities.has_ability(QUEST),
        "the rest of the reset stands"
    );
    let update = frames
        .iter()
        .find(|(m, _)| *m == method_idx::ON_KNOWN_ABILITIES_UPDATE)
        .expect("one known-abilities update");
    assert!(
        update
            .1
            .windows(4)
            .any(|w| w == SHOT.to_le_bytes().as_slice()),
        "the client's known list carries the shot"
    );
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
            source: GmAbilitySource::Command,
            added: vec![],
            removed: vec![QUEST],
            training_points: 3,
        },
    )
    .await;
    assert!(frames.is_empty());
    assert_eq!(known(&mgr), before);
}
