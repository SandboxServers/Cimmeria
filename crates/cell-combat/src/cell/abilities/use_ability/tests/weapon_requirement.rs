//! The player weapon-moniker requirement (Class Start v6 CS-07, OD-CS11)
//! and the removal of the 592 → active-weapon redirect.
//!
//! Ability and item ids are the seeded ones, with their seeded monikers:
//! 592 Pistol Shot {ITEM_Pistol}, 598 Quick Burst {ITEM_Automatic_Weapon},
//! 1984 Staff Swing {ITEM_Staff}, 1639 Ribbon Device:Destruction Beam
//! {ITEM_RibbonDevice}; item 55 SI 3 9mm Pistol, 21 SGHC 6 SMG, 3260 SK37
//! LMG (ITEM_LightMG only), 2797 Serpent Staff, 4565 Serpent Ribbon Device.

use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::super::weapon_requirement::{WRONG_WEAPON_TEXT, WRONG_WEAPON_TYPE_ERROR_CODE};
use super::warmup::{calls, ON_TIMER_UPDATE};
use super::*;

const ITEM_PISTOL: i64 = 2_445_422_768;
const ITEM_AUTOMATIC_WEAPON: i64 = 3_175_425_141;
const ITEM_SMG: i64 = 728_213_066;
const ITEM_LIGHT_MG: i64 = 1_115_110_575;
const ITEM_STAFF: i64 = 1_383_013_887;
const ITEM_RIBBON_DEVICE: i64 = 4_193_235_610;
const CATEGORY_WEAPONS: i64 = 3_901_383_057;

const PISTOL_SHOT: i32 = 592;
const QUICK_BURST: i32 = 598;
const STAFF_SWING: i32 = 1984;
const DESTRUCTION_BEAM: i32 = 1639;
const SMG_AUTO_ATTACK: i32 = 559;

const PISTOL: i32 = 55;
const SGHC_SMG: i32 = 21;
const SK37_LMG: i32 = 3260;
const SERPENT_STAFF: i32 = 2797;
const RIBBON_DEVICE: i32 = 4565;

const PLAYER: u32 = 1;
const TARGET: u32 = 2;
const CLIP: i32 = 30;

/// The seeded `item_monikers` of an ability.
fn requirement(ability_id: i32) -> Vec<i64> {
    match ability_id {
        PISTOL_SHOT => vec![ITEM_PISTOL],
        QUICK_BURST | SMG_AUTO_ATTACK => vec![ITEM_AUTOMATIC_WEAPON],
        STAFF_SWING => vec![ITEM_STAFF],
        DESTRUCTION_BEAM => vec![ITEM_RIBBON_DEVICE],
        _ => vec![],
    }
}

/// A one-ammo weapon ability with its seeded requirement.
fn weapon_ability(ability_id: i32) -> AbilityDef {
    AbilityDef {
        item_monikers: requirement(ability_id),
        ..make_ability(ability_id, 1, 30)
    }
}

/// Player 1 (weapon drawn) knowing `ability_id`, a hostile NPC 2 three
/// metres away, the seeded monikers of every starter weapon, and `item` (if
/// any) in the active bandolier slot with a full clip.
fn scene(ability_id: i32, item: Option<i32>) -> SpaceManager {
    let mut mgr = make_mgr();
    make_player(&mut mgr, PLAYER, [0.0; 3]);
    mgr.create_entity(TARGET, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(TARGET).unwrap().faction = crate::cell::combat::HOSTILE_FACTION;
    for (id, monikers) in [
        (PISTOL, vec![ITEM_PISTOL, CATEGORY_WEAPONS]),
        (
            SGHC_SMG,
            vec![ITEM_AUTOMATIC_WEAPON, ITEM_SMG, CATEGORY_WEAPONS],
        ),
        (SK37_LMG, vec![ITEM_LIGHT_MG, CATEGORY_WEAPONS]),
        (SERPENT_STAFF, vec![ITEM_STAFF, CATEGORY_WEAPONS]),
        (RIBBON_DEVICE, vec![ITEM_RIBBON_DEVICE, CATEGORY_WEAPONS]),
    ] {
        mgr.item_monikers.insert(id, monikers);
    }
    mgr.ability_defs
        .insert(ability_id, weapon_ability(ability_id));
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.abilities.add_ability(ability_id);
    p.weapon_holstered = false;
    p.active_bandolier_slot = 0;
    if let Some(item_id) = item {
        p.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id,
                clip_size: CLIP,
                default_ammo_type: 2,
                current_ammo: CLIP,
                cur_ammo_type: 2,
            },
        );
    }
    mgr
}

/// The exact `onErrorCode(ERRORCODE_SYSTEM_Ability, ability, 63)` args.
fn wrong_weapon_error(ability_id: i32) -> Vec<u8> {
    let mut err = vec![0u8];
    err.extend_from_slice(&ability_id.to_le_bytes());
    err.extend_from_slice(&WRONG_WEAPON_TYPE_ERROR_CODE.to_le_bytes());
    err
}

/// Press `ability_id` at the hostile NPC; return whether it committed and
/// the messages sent.
async fn press(mgr: &mut SpaceManager, ability_id: i32) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let committed = handle_use_ability(PLAYER, ability_id, TARGET as i32, &tx, mgr).await;
    (committed, drain(&mut rx))
}

/// Assert a refused press: no commit, no cooldown, no ammo spent, no timer,
/// and the exact WrongWeaponType `onErrorCode` plus the feedback line.
fn assert_wrong_weapon(
    mgr: &SpaceManager,
    ability_id: i32,
    committed: bool,
    msgs: &[CellToBaseMsg],
) {
    assert!(
        !committed,
        "{ability_id}: the wrong weapon must refuse the cast"
    );
    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(
        !p.abilities.is_on_cooldown(ability_id),
        "{ability_id}: a refusal charges no cooldown"
    );
    if let Some(b) = p.bandolier_items.get(&0) {
        assert_eq!(
            b.current_ammo, CLIP,
            "{ability_id}: a refusal spends no ammo"
        );
    }
    let sent = calls(msgs);
    assert!(
        !sent.iter().any(|(_, m, _)| *m == ON_TIMER_UPDATE),
        "{ability_id}: no cooldown timer is sent: {sent:?}"
    );
    let errors: Vec<&Vec<u8>> = sent
        .iter()
        .filter(|(e, m, _)| *e == PLAYER && *m == method_idx::ON_ERROR_CODE)
        .map(|(_, _, a)| a)
        .collect();
    assert_eq!(
        errors,
        vec![&wrong_weapon_error(ability_id)],
        "{ability_id}: exactly one onErrorCode(0, ability, 63)"
    );
    let line = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, WRONG_WEAPON_TEXT);
    assert!(
        sent.iter().any(|(e, m, a)| *e == PLAYER
            && *m == method_idx::ON_PLAYER_COMMUNICATION
            && *a == line),
        "{ability_id}: the feedback line is sent so the first press is visible: {sent:?}"
    );
}

/// Assert a committed press that fired `ability_id` itself.
fn assert_fires(mgr: &SpaceManager, ability_id: i32, committed: bool, msgs: &[CellToBaseMsg]) {
    assert!(committed, "{ability_id}: the right weapon must let it fire");
    assert!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .abilities
            .is_on_cooldown(ability_id),
        "{ability_id}: the ability the client sent is the one that fired"
    );
    assert!(
        !calls(msgs).iter().any(
            |(_, m, a)| *m == method_idx::ON_ERROR_CODE && *a == wrong_weapon_error(ability_id)
        ),
        "{ability_id}: no WrongWeaponType"
    );
}

/// The starter cases that hold: 592 + 55, 598 + 21, 1984 + 2797,
/// 1639 + 4565 all fire as themselves.
#[tokio::test]
async fn starter_abilities_fire_with_a_weapon_carrying_their_moniker() {
    for (ability, item) in [
        (PISTOL_SHOT, PISTOL),
        (QUICK_BURST, SGHC_SMG),
        (STAFF_SWING, SERPENT_STAFF),
        (DESTRUCTION_BEAM, RIBBON_DEVICE),
    ] {
        let mut mgr = scene(ability, Some(item));
        let (committed, msgs) = press(&mut mgr, ability).await;
        assert_fires(&mgr, ability, committed, &msgs);
    }
}

/// **Regression guard (OD-CS11).** 598 Quick Burst needs
/// ITEM_Automatic_Weapon; the SK37 LMG (3260) carries ITEM_LightMG only, so
/// the press is refused with WrongWeaponType and charges nothing. Without
/// the gate the cast commits.
#[tokio::test]
async fn quick_burst_with_the_sk37_lmg_is_refused_with_wrong_weapon_type() {
    let mut mgr = scene(QUICK_BURST, Some(SK37_LMG));
    let (committed, msgs) = press(&mut mgr, QUICK_BURST).await;
    assert_wrong_weapon(&mgr, QUICK_BURST, committed, &msgs);
}

/// **Regression guard (redirect removed).** 592 with the SGHC 6 (item 21)
/// active is refused with WrongWeaponType. It is not redirected to the
/// SMG's 559, and neither 592 nor 559 enters cooldown. With the #495
/// redirect back, 559 fires and its cooldown starts.
#[tokio::test]
async fn pistol_shot_with_the_smg_is_refused_not_redirected() {
    let mut mgr = scene(PISTOL_SHOT, Some(SGHC_SMG));
    mgr.item_event_set_abilities.insert(
        (SGHC_SMG, crate::cell::spawner::EVENT_ITEM_RANGED),
        SMG_AUTO_ATTACK,
    );
    mgr.ability_defs
        .insert(SMG_AUTO_ATTACK, weapon_ability(SMG_AUTO_ATTACK));
    let (committed, msgs) = press(&mut mgr, PISTOL_SHOT).await;
    assert_wrong_weapon(&mgr, PISTOL_SHOT, committed, &msgs);
    assert!(
        !mgr.get_entity(PLAYER)
            .unwrap()
            .abilities
            .is_on_cooldown(SMG_AUTO_ATTACK),
        "592 never becomes the active weapon's attack"
    );
}

/// 592 with the pistol fires 592 itself, even when the pistol also binds
/// its own ranged attack: there is no redirect in either direction.
#[tokio::test]
async fn pistol_shot_with_the_pistol_fires_pistol_shot_itself() {
    const PISTOL_AUTO_ATTACK: i32 = 579;
    let mut mgr = scene(PISTOL_SHOT, Some(PISTOL));
    mgr.item_event_set_abilities.insert(
        (PISTOL, crate::cell::spawner::EVENT_ITEM_RANGED),
        PISTOL_AUTO_ATTACK,
    );
    mgr.ability_defs
        .insert(PISTOL_AUTO_ATTACK, weapon_ability(PISTOL_AUTO_ATTACK));
    let (committed, msgs) = press(&mut mgr, PISTOL_SHOT).await;
    assert_fires(&mgr, PISTOL_SHOT, committed, &msgs);
    assert!(!mgr
        .get_entity(PLAYER)
        .unwrap()
        .abilities
        .is_on_cooldown(PISTOL_AUTO_ATTACK));
}

/// An empty active slot fails any requirement: 592 bare-handed is refused.
#[tokio::test]
async fn a_requirement_with_no_weapon_is_refused() {
    let mut mgr = scene(PISTOL_SHOT, None);
    let (committed, msgs) = press(&mut mgr, PISTOL_SHOT).await;
    assert_wrong_weapon(&mgr, PISTOL_SHOT, committed, &msgs);
}

/// No requirement, no change: an ability with an empty `item_monikers`
/// fires with no weapon and with any weapon.
#[tokio::test]
async fn an_ability_with_no_requirement_is_unchanged() {
    const NO_REQUIREMENT: i32 = 4321;
    for item in [None, Some(SK37_LMG)] {
        let mut mgr = scene(NO_REQUIREMENT, item);
        mgr.ability_defs
            .insert(NO_REQUIREMENT, make_ability(NO_REQUIREMENT, 0, 30));
        let (committed, msgs) = press(&mut mgr, NO_REQUIREMENT).await;
        assert_fires(&mgr, NO_REQUIREMENT, committed, &msgs);
    }
}

/// A requirement is met by any one shared moniker: 598 also fires with an
/// item that carries ITEM_Automatic_Weapon among others.
#[tokio::test]
async fn one_shared_moniker_is_enough() {
    const OTHER_SMG: i32 = 9001;
    let mut mgr = scene(QUICK_BURST, Some(OTHER_SMG));
    mgr.item_monikers.insert(
        OTHER_SMG,
        vec![CATEGORY_WEAPONS, ITEM_SMG, ITEM_AUTOMATIC_WEAPON],
    );
    let (committed, msgs) = press(&mut mgr, QUICK_BURST).await;
    assert_fires(&mgr, QUICK_BURST, committed, &msgs);
}

/// **NPC casts are unchanged.** An NPC has no bandolier, and its 592 still
/// fires though 592 requires a pistol.
#[tokio::test]
async fn an_npc_cast_is_not_checked() {
    use super::sequence::{scene as npc_scene, NPC, SHOOTER};
    let mut mgr = npc_scene();
    mgr.get_entity_mut(NPC)
        .unwrap()
        .abilities
        .add_ability(PISTOL_SHOT);
    mgr.ability_defs.insert(
        PISTOL_SHOT,
        AbilityDef {
            item_monikers: vec![ITEM_PISTOL],
            ..make_ability(PISTOL_SHOT, 0, 40)
        },
    );
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        handle_use_ability(NPC, PISTOL_SHOT, SHOOTER as i32, &tx, &mut mgr).await,
        "an NPC's 592 fires without a weapon"
    );
    assert!(mgr
        .get_entity(NPC)
        .unwrap()
        .abilities
        .is_on_cooldown(PISTOL_SHOT));
}

/// An auto-cycle loop armed on the refused ability is stopped, so the tick
/// does not repeat the refusal every cooldown.
#[tokio::test]
async fn a_refusal_stops_a_loop_armed_on_that_ability() {
    let mut mgr = scene(QUICK_BURST, Some(SK37_LMG));
    {
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        p.abilities.auto_cycle = true;
        p.abilities.auto_cycle_ability_id = Some(QUICK_BURST);
    }
    let (committed, msgs) = press(&mut mgr, QUICK_BURST).await;
    assert_wrong_weapon(&mgr, QUICK_BURST, committed, &msgs);
    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(!p.abilities.auto_cycle, "the loop is stopped");
    assert_eq!(p.abilities.auto_cycle_ability_id, None);
}
