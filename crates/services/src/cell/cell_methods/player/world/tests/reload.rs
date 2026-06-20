//! `handle_reload` core dispatch tests: full-clip no-op, reload-slot
//! pinning, and the byte-exact Item_Reload sequence + ammo-type propId
//! wire layout. Holster/phase choreography lives in `reload_holster.rs`.

use super::super::*;
use super::make_mgr_with_player;
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::cell_entity::BandolierItem;

/// `handle_reload` is a no-op when the active slot is at full
/// clip and no reload is in flight. Pin so a refactor that
/// always starts a reload (and therefore wastes ammo on every
/// keypress) gets caught.
#[tokio::test]
async fn handle_reload_no_op_when_already_full() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 30, // full
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    let (tx, mut rx) = mpsc::channel(8);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none(),
        "no reload should be queued when full"
    );
    assert!(rx.try_recv().is_err(), "no packets should be emitted");
}

/// `handle_reload` from an empty magazine pins the slot id at
/// the time of issue. If the player swaps mid-reload, the
/// completion tick must refill THIS slot, not whatever slot is
/// active when the deadline elapses.
#[tokio::test]
async fn handle_reload_pins_reload_slot_id_to_current_active_slot() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        // Weapon already drawn so Phase A (defer-reload-for-draw)
        // doesn't kick in — this test asserts the Phase B slot
        // pin, not the Phase A defer path.
        e.weapon_holstered = false;
        e.bandolier_items.insert(
            2,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 2;
    }
    // Seed the reload AbilityDef so warmup/cooldown/event_set are read.
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    let (tx, _rx) = mpsc::channel(16);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_some(),
        "reload must arm the deadline"
    );
    assert_eq!(
        e.reload_slot_id,
        Some(2),
        "reload_slot_id must capture the active slot at issue time, not be re-read at completion"
    );
}

/// Reload-start must fire the `Item_Reload` (event 4002) sequence from
/// the player's archetype-keyed "Item handling" event set so the
/// client plays the visible reload animation. Mirrors
/// `python/cell/SGWBeing.py:863-874` (`getItemSequence(Item_Reload)` +
/// `playSequence`). Previously this site looked up the *reload
/// ability's* `event_set_id`, which is NULL in the seed — the lookup
/// short-circuited and no animation ever played in production.
///
/// Bug shape this catches: a refactor that goes back to keying off
/// `ability_defs[596].event_set_id` reintroduces the dead path.
#[tokio::test]
async fn handle_reload_sends_item_reload_sequence() {
    use crate::cell::client_methods::spawnable_entity::ON_SEQUENCE;
    use crate::cell::spawner::{EVENT_ABILITY_BEGIN, EVENT_ITEM_RELOAD};

    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        // Soldier archetype → event set 804 (the human "Item handling"
        // set per `archetype_item_event_set`).
        e.archetype_id = Some(1);
        // Weapon already drawn so Phase A (defer-reload-for-draw)
        // doesn't kick in — this test asserts the Phase B byte
        // layout of Item_Reload's ON_SEQUENCE.
        e.weapon_holstered = false;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    // Seed the sequence_map so the (804, Item_Reload) lookup finds a
    // sentinel sequence id we can recognise on the wire.
    const HUMAN_ITEM_EVENT_SET: i32 = 804;
    const ITEM_RELOAD_SEQ_ID: i32 = 1874;
    mgr.sequence_map.insert(
        (HUMAN_ITEM_EVENT_SET, EVENT_ITEM_RELOAD),
        ITEM_RELOAD_SEQ_ID,
    );
    // Seed a decoy (ability's event set → Ability_Begin) — the
    // regression we're guarding against would have sent THIS one.
    const DECOY_EVENT_SET: i32 = 7777;
    const DECOY_BEGIN_SEQ_ID: i32 = 9001;
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: Some(DECOY_EVENT_SET),
            velocity: 0.0,
        },
    );
    mgr.sequence_map
        .insert((DECOY_EVENT_SET, EVENT_ABILITY_BEGIN), DECOY_BEGIN_SEQ_ID);

    let (tx, mut rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    // ON_SEQUENCE wire layout (26 bytes — matches use_ability.rs's fire path):
    //   sequence_id   i32 LE  @ 0..4
    //   source_id     i32 LE  @ 4..8
    //   target_id     i32 LE  @ 8..12
    //   primary       u8      @ 12
    //   impact_time   f32 LE  @ 13..17
    //   nvp_count     u32 LE  @ 17..21
    //   view_type     u8      @ 21
    //   instance_id   i32 LE  @ 22..26
    let mut item_reload_count = 0;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == ON_SEQUENCE {
                assert_eq!(
                    args.len(),
                    26,
                    "ON_SEQUENCE payload must be exactly 26 bytes — any drift \
                     in the serializer would silently corrupt the kismet event \
                     frame on the wire"
                );
                let seq_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_ne!(
                    seq_id, DECOY_BEGIN_SEQ_ID,
                    "reload-start must NOT fire the reload ability's Ability_Begin \
                     (that's the dead pre-fix path)",
                );
                if seq_id != ITEM_RELOAD_SEQ_ID {
                    continue;
                }
                let source_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let primary = args[12];
                let impact_time = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);
                let nvp_count = u32::from_le_bytes([args[17], args[18], args[19], args[20]]);
                let view_type = args[21];
                let instance_id = i32::from_le_bytes([args[22], args[23], args[24], args[25]]);
                assert_eq!(
                    source_id, 1,
                    "source = entity_id (player firing the reload)"
                );
                assert_eq!(target_id, 1, "reload targets self");
                assert_eq!(primary, 1, "primary target flag set");
                assert_eq!(impact_time, 0.0, "no projectile impact time for reload");
                assert_eq!(nvp_count, 0, "no name-value pairs in payload");
                assert_eq!(
                    view_type, 0,
                    "ViewType=0 (KISMET_VIEW_Witness) — matches use_ability.rs's \
                     fire path so reload-begin animates consistently with weapon \
                     fire animations"
                );
                assert_eq!(instance_id, 0, "no effect instance for the reload sequence");
                item_reload_count += 1;
            }
        }
    }
    assert_eq!(
        item_reload_count, 1,
        "reload-start must send exactly one onSequence with the Item_Reload \
         sequence id; without it the client plays no visible reload animation. \
         Got {item_reload_count}.",
    );
}

/// Reload-start must emit the ammo-type update under
/// `GENERICPROPERTY_AmmoTypeId` (propId 3), NOT under propId 7
/// (`GENERICPROPERTY_AccessLevel`).
///
/// Bug shape this catches (issue #168): the historical handler used
/// a hardcoded `7i32` for the propId arg of `onEntityProperty`,
/// which made the post-reload property update land on the client's
/// `setAccessLevel` slot with the ammo type as the value. The HUD's
/// ammo-type indicator still updated in practice because the
/// bandolier sync path independently emits propId 3 on cur_ammo_type
/// changes — so reverting the fix wouldn't break the visible HUD,
/// but it would re-plant a stray `setAccessLevel(<ammo_type>)` on
/// the wire. A refactor that goes back to `7i32.to_le_bytes()` here
/// would fail this guard.
#[tokio::test]
async fn handle_reload_emits_ammo_type_under_correct_propid() {
    use crate::cell::cell_methods::inventory::GENERICPROPERTY_AMMO_TYPE_ID;
    use crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY;
    use crate::cell::spawner::EVENT_ITEM_RELOAD;

    const AMMO_TYPE: i32 = 42;

    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_holstered = false;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: AMMO_TYPE,
                current_ammo: 0,
                cur_ammo_type: AMMO_TYPE,
            },
        );
        e.active_bandolier_slot = 0;
    }
    // Seed the sequence_map so Phase B doesn't bail before reaching
    // the propId emit — same shape as the sibling Item_Reload test.
    mgr.sequence_map.insert((804, EVENT_ITEM_RELOAD), 1874);

    let (tx, mut rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let mut entity_property_calls = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == ON_ENTITY_PROPERTY {
                assert_eq!(args.len(), 8, "onEntityProperty payload is 8 bytes LE");
                let prop_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let value = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                entity_property_calls.push((prop_id, value));
            }
        }
    }

    let ammo_call = entity_property_calls
        .iter()
        .find(|(_, v)| *v == AMMO_TYPE)
        .copied();
    assert_eq!(
        ammo_call,
        Some((GENERICPROPERTY_AMMO_TYPE_ID, AMMO_TYPE)),
        "ammo-type onEntityProperty must use propId {GENERICPROPERTY_AMMO_TYPE_ID} (AmmoTypeId), \
         not propId 7 (AccessLevel). All onEntityProperty calls observed: {entity_property_calls:?}",
    );
    assert!(
        !entity_property_calls
            .iter()
            .any(|(p, v)| *p == 7 && *v == AMMO_TYPE),
        "no onEntityProperty(7, <ammo_type>) — that's the pre-fix shape (issue #168). \
         All onEntityProperty calls observed: {entity_property_calls:?}",
    );
}
