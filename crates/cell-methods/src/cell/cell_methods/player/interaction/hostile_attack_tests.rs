//! Right-click on a live hostile with the active weapon (CS-07 review
//! finding 2): the RANGED binding fires; a weapon with no RANGED binding
//! fires nothing and says so, where the old `592 Pistol Shot` fallback was
//! refused by the weapon requirement with a misleading line.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::hostile_attack::NO_RANGED_ATTACK_TEXT;
use crate::cell::cell_methods::player::INTERACT;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

const PLAYER: u32 = 1;
const ITEM_PISTOL: i64 = 2_445_422_768;
const ITEM_RIFLE: i64 = 2_882_868_408;
const ITEM_BLADE: i64 = 830_336_901;
const PISTOL_SHOT: i32 = 592;
const RIFLE_AUTO_ATTACK: i32 = 581;
/// SR1 .50-Cal Rifle: an ITEM_Rifle sniper rifle with a clip.
const SNIPER_RIFLE: i32 = 3287;
/// A blade: no RANGED binding.
const COMBAT_KNIFE: i32 = 9_901;

const ON_TARGET_UPDATE: u16 = 16;

/// Player 1 at the origin knowing 592 (with its seeded ITEM_Pistol
/// requirement), `item` drawn in the active slot with a full clip, and a
/// live hostile NPC `distance` metres away. Returns the manager and the NPC.
fn scene(item_id: i32, item_monikers: Vec<i64>, distance: f32) -> (SpaceManager, u32) {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [distance, 0.0, 0.0], [0.0; 3])
        .unwrap();
    {
        let n = mgr.get_entity_mut(npc).unwrap();
        n.faction = crate::cell::combat::HOSTILE_FACTION;
        n.clear_all_state_flags();
    }
    mgr.item_monikers.insert(item_id, item_monikers);
    mgr.ability_defs.insert(
        PISTOL_SHOT,
        AbilityDef {
            ability_id: PISTOL_SHOT,
            name: "Pistol Shot".into(),
            cooldown: 0.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0.0,
            max_range: 30.0,
            target_type_id: 2,
            effect_ids: vec![],
            moniker_ids: vec![],
            item_monikers: vec![ITEM_PISTOL],
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
            type_id: Default::default(),
            passive: false,
        },
    );
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(4242);
    p.abilities.add_ability(PISTOL_SHOT);
    p.weapon_holstered = false;
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 1,
            item_id,
            clip_size: 30,
            default_ammo_type: 0,
            current_ammo: 30,
            cur_ammo_type: 0,
        },
    );
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    (mgr, npc)
}

/// Right-click `npc`; return every method sent to the player.
async fn right_click(mgr: &mut SpaceManager, npc: u32) -> Vec<(u16, Vec<u8>)> {
    let (tx, mut rx) = mpsc::channel(256);
    let args = (npc as i32).to_le_bytes();
    assert!(super::dispatch(PLAYER, INTERACT, &args, &tx, mgr, &ChainEngine::new()).await);
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: PLAYER,
            method_index,
            args,
        } = msg
        {
            out.push((method_index, args));
        }
    }
    out
}

/// **Regression guard.** A blade has no RANGED binding: right-click targets
/// the mob, fires nothing, charges nothing and sends exactly the
/// no-ranged-attack line. With the old 592 fallback back, the press would
/// be refused with `onErrorCode(.., 63)` and "You need a different weapon".
#[tokio::test]
async fn right_click_with_no_ranged_binding_says_so_and_fires_nothing() {
    let (mut mgr, npc) = scene(COMBAT_KNIFE, vec![ITEM_BLADE], 2.0);
    let sent = right_click(&mut mgr, npc).await;

    assert!(
        sent.iter().any(|(m, _)| *m == ON_TARGET_UPDATE),
        "the mob is still targeted: {sent:?}"
    );
    let line = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, NO_RANGED_ATTACK_TEXT);
    let lines: Vec<_> = sent
        .iter()
        .filter(|(m, _)| *m == crate::mercury::method_idx::ON_PLAYER_COMMUNICATION)
        .collect();
    assert_eq!(lines.len(), 1, "one feedback line: {sent:?}");
    assert_eq!(lines[0].1, line, "it names the real reason");
    assert!(
        !sent
            .iter()
            .any(|(m, _)| *m == crate::mercury::method_idx::ON_ERROR_CODE),
        "no WrongWeaponType refusal: nothing was cast"
    );
    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(!p.abilities.is_on_cooldown(PISTOL_SHOT), "592 is not tried");
    assert_eq!(p.active_ammo(), 30, "no round is spent");
    assert_eq!(p.current_target_id, Some(npc as i32));
}

/// An armed weapon fires its RANGED binding (here the rifle's 581, which
/// its ITEM_Rifle meets).
#[tokio::test]
async fn right_click_fires_the_weapons_ranged_binding() {
    let (mut mgr, npc) = scene(SNIPER_RIFLE, vec![ITEM_RIFLE], 2.0);
    mgr.item_event_set_abilities.insert(
        (SNIPER_RIFLE, spawner::EVENT_ITEM_RANGED),
        RIFLE_AUTO_ATTACK,
    );
    mgr.ability_defs.insert(
        RIFLE_AUTO_ATTACK,
        AbilityDef {
            ability_id: RIFLE_AUTO_ATTACK,
            name: "Rifle Auto Attack".into(),
            cooldown: 3.0,
            item_monikers: vec![ITEM_RIFLE],
            ..mgr.ability_defs[&PISTOL_SHOT].clone()
        },
    );
    let sent = right_click(&mut mgr, npc).await;
    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(
        p.abilities.is_on_cooldown(RIFLE_AUTO_ATTACK),
        "the rifle's RANGED binding fires: {sent:?}"
    );
    assert_eq!(p.active_ammo(), 29);
}

/// Unarmed right-click presses 594 Strike (no requirement in the seed), not
/// 592, and gets no "no ranged attack" line, which is for an armed player.
/// The fixture's 594 has no effect, so the launch answers it with the
/// no-mechanics refusal (167) naming 594: that names the ability chosen.
#[tokio::test]
async fn unarmed_right_click_swings_strike() {
    const STRIKE: i32 = 594;
    let (mut mgr, npc) = scene(COMBAT_KNIFE, vec![ITEM_BLADE], 2.0);
    mgr.get_entity_mut(PLAYER).unwrap().bandolier_items.clear();
    mgr.ability_defs.insert(
        STRIKE,
        AbilityDef {
            ability_id: STRIKE,
            name: "Strike".into(),
            cooldown: 2.0,
            is_ranged: false,
            max_range: 5.0,
            required_ammo: 0,
            item_monikers: vec![],
            ..mgr.ability_defs[&PISTOL_SHOT].clone()
        },
    );
    let sent = right_click(&mut mgr, npc).await;
    let p = mgr.get_entity(PLAYER).unwrap();
    let pressed: Vec<i32> = sent
        .iter()
        .filter(|(m, _)| *m == crate::mercury::method_idx::ON_ERROR_CODE)
        .map(|(_, a)| i32::from_le_bytes([a[1], a[2], a[3], a[4]]))
        .collect();
    assert_eq!(
        pressed,
        vec![STRIKE],
        "right-click unarmed presses 594: {sent:?}"
    );
    assert!(!p.abilities.is_on_cooldown(PISTOL_SHOT));
    let line = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, NO_RANGED_ATTACK_TEXT);
    assert!(!sent.iter().any(|(_, a)| *a == line));
}

/// **Live-DB regression guard (seed).** Every ITEM_Rifle sniper rifle now
/// carries a RANGED binding: right-click with the seeded SR1 .50-Cal Rifle
/// (3287) fires 581 Rifle Auto Attack from the real seed rows (binding,
/// ability def, monikers, weapon range). Without the CS-07 seed rows 3287
/// has no RANGED binding and nothing fires.
#[tokio::test]
async fn sniper_rifle_right_click_fires_rifle_auto_attack_live_db() {
    let pool = require_db_or_skip!();
    let bindings = spawner::load_item_event_set_abilities(&pool)
        .await
        .expect("items_event_sets load");
    let defs = spawner::load_ability_defs(&pool)
        .await
        .expect("ability defs load");
    let monikers = spawner::load_weapon_monikers(&pool)
        .await
        .expect("item monikers load");
    let ranges = spawner::load_weapon_ranges(&pool)
        .await
        .expect("weapon ranges load");

    assert_eq!(
        bindings.get(&(SNIPER_RIFLE, spawner::EVENT_ITEM_RANGED)),
        Some(&RIFLE_AUTO_ATTACK),
        "the seed binds 581 to the sniper rifle"
    );
    let (mut mgr, npc) = scene(
        SNIPER_RIFLE,
        monikers.get(&SNIPER_RIFLE).cloned().unwrap_or_default(),
        10.0,
    );
    mgr.item_event_set_abilities = bindings;
    mgr.weapon_ranges = ranges;
    mgr.item_monikers = monikers;
    mgr.ability_defs.insert(
        RIFLE_AUTO_ATTACK,
        defs.get(&RIFLE_AUTO_ATTACK)
            .cloned()
            .expect("581 is seeded"),
    );

    let sent = right_click(&mut mgr, npc).await;
    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(
        p.abilities.is_on_cooldown(RIFLE_AUTO_ATTACK),
        "right-click with the SR1 fires 581: {sent:?}"
    );
    assert!(
        !sent
            .iter()
            .any(|(m, _)| *m == crate::mercury::method_idx::ON_ERROR_CODE),
        "581's ITEM_Rifle requirement is met by the rifle"
    );
}
