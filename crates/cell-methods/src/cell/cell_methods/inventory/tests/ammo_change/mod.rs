//! `requestAmmoChange` (cell method 42) through the inventory dispatcher.
//!
//! The wire `ItemId` is the weapon's **instance** id (#534), so every
//! request here names `INSTANCE` (4242), never the design id `DESIGN` (42).
//! A refusal sends exactly one `CHAN_FEEDBACK` line and nothing else: no
//! `BandolierAmmoUpdate`, no `onEntityProperty` (AM-03).

mod live_db;

use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_wire::cell::chat::CHAN_FEEDBACK;
use tokio::sync::mpsc;
use tracing::Level;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::WeaponDef;
use crate::test_support::LogCapture;

use super::super::constants::GENERICPROPERTY_AMMO_TYPE_ID;
use super::super::dispatch::dispatch;
use super::super::REQUEST_AMMO_CHANGE;
use super::make_test_space_mgr;

const ENTITY: u32 = 1;
const ACCOUNT: u32 = 7;
const PLAYER: i32 = 100;
const DESIGN: i32 = 42;
const INSTANCE: i32 = 4242;
const ON_PLAYER_COMMUNICATION: u16 =
    cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;

fn weapon(instance_id: i32, item_id: i32) -> BandolierItem {
    BandolierItem {
        instance_id,
        item_id,
        clip_size: 30,
        default_ammo_type: 1,
        current_ammo: 20,
        cur_ammo_type: 1,
    }
}

fn weapon_def(allowed: Vec<i32>) -> WeaponDef {
    WeaponDef {
        clip_size: 30,
        default_ammo_type: 1,
        allowed_ammo_types: allowed,
        holster_animation_duration: std::time::Duration::from_millis(600),
    }
}

/// A player (entity 1) holding weapon `DESIGN` / `INSTANCE` in active slot
/// 0, with a cached `WeaponDef` allowing types 1, 3 and 5.
fn player_with_weapon() -> SpaceManager {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(ENTITY, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.item_defs.insert(DESIGN, weapon_def(vec![1, 3, 5]));
    let e = mgr.get_entity_mut(ENTITY).unwrap();
    e.is_player = true;
    e.account_id = Some(ACCOUNT);
    e.player_id = Some(PLAYER);
    e.bandolier_items.insert(0, weapon(INSTANCE, DESIGN));
    e.active_bandolier_slot = 0;
    mgr
}

fn request(item_id: i32, ammo_type: i32) -> Vec<u8> {
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&item_id.to_le_bytes());
    args.extend_from_slice(&ammo_type.to_le_bytes());
    args
}

async fn send(mgr: &mut SpaceManager, args: &[u8]) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(16);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    assert!(
        dispatch(ENTITY, REQUEST_AMMO_CHANGE, args, &tx, mgr, &engine).await,
        "REQUEST_AMMO_CHANGE should be claimed"
    );
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

/// The refusal's only message: one `onPlayerCommunication` to the sender on
/// `CHAN_FEEDBACK`. Returns its args.
fn only_feedback_line(msgs: &[CellToBaseMsg]) -> Vec<u8> {
    match msgs {
        [CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        }] => {
            assert_eq!(*entity_id, ENTITY);
            assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);
            args.clone()
        }
        other => panic!(
            "a refusal must send exactly one feedback line and no \
             BandolierAmmoUpdate / onEntityProperty; got {other:?}"
        ),
    }
}

/// The feedback line's text, decoded from `onPlayerCommunication` args
/// (WSTRING speaker, u8 flags, u8 channel, WSTRING text).
fn feedback_text(args: &[u8]) -> String {
    let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
    let at = 4 + speaker_len * 2;
    assert_eq!(args[at + 1], CHAN_FEEDBACK, "channel must be CHAN_FEEDBACK");
    let text_len = u32::from_le_bytes(args[at + 2..at + 6].try_into().unwrap()) as usize;
    let units: Vec<u16> = args[at + 6..at + 6 + text_len * 2]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    String::from_utf16(&units).unwrap()
}

fn assert_slot_untouched(mgr: &SpaceManager, slot: i32) {
    let e = mgr.get_entity(ENTITY).unwrap();
    assert_eq!(e.bandolier_items[&slot].cur_ammo_type, 1, "slot unchanged");
    assert!(
        !e.bandolier_ammo_dirty.contains(&slot),
        "nothing may be queued for persistence"
    );
}

/// An allowed type swaps the slot, persists it by instance PK, and (active
/// slot) refreshes the client's indicator with `onEntityProperty`.
#[tokio::test]
async fn request_ammo_change_updates_slot_and_sends_property() {
    let mut mgr = player_with_weapon();
    let msgs = send(&mut mgr, &request(INSTANCE, 3)).await;

    let e = mgr.get_entity(ENTITY).unwrap();
    assert_eq!(e.bandolier_items[&0].cur_ammo_type, 3);
    assert!(
        !e.bandolier_ammo_dirty.contains(&0),
        "dirty flag should be drained immediately"
    );
    match msgs.as_slice() {
        [CellToBaseMsg::BandolierAmmoUpdate {
            player_id,
            slot_id,
            expected_instance_id,
            current_ammo,
            cur_ammo_type,
        }, CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        }] => {
            assert_eq!(*player_id, PLAYER);
            assert_eq!(*slot_id, 0);
            assert_eq!(*expected_instance_id, INSTANCE);
            assert_eq!(*current_ammo, 20);
            assert_eq!(*cur_ammo_type, 3);
            assert_eq!(*entity_id, ENTITY);
            assert_eq!(
                *method_index,
                crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY
            );
            // Byte-exact: INT32 propId 3, INT32 value 3.
            let mut want = GENERICPROPERTY_AMMO_TYPE_ID.to_le_bytes().to_vec();
            want.extend_from_slice(&3i32.to_le_bytes());
            assert_eq!(args, &want);
        }
        other => panic!("expected BandolierAmmoUpdate then onEntityProperty, got {other:?}"),
    }
}

/// #534: the wire value is the instance id. Two slots hold the same design
/// with different instances; naming the second instance swaps only that
/// slot. Matching on the design id again would refuse this as ambiguous.
#[tokio::test]
async fn request_ammo_change_keys_the_slot_on_the_instance_id() {
    let mut mgr = player_with_weapon();
    mgr.get_entity_mut(ENTITY)
        .unwrap()
        .bandolier_items
        .insert(1, weapon(4343, DESIGN));

    let msgs = send(&mut mgr, &request(4343, 5)).await;

    let e = mgr.get_entity(ENTITY).unwrap();
    assert_eq!(e.bandolier_items[&1].cur_ammo_type, 5, "slot 1 swapped");
    assert_eq!(e.bandolier_items[&0].cur_ammo_type, 1, "slot 0 untouched");
    // Slot 1 is not active: persist only, no indicator refresh.
    assert!(
        matches!(
            msgs.as_slice(),
            [CellToBaseMsg::BandolierAmmoUpdate {
                slot_id: 1,
                expected_instance_id: 4343,
                cur_ammo_type: 5,
                ..
            }]
        ),
        "{msgs:?}"
    );
}

/// The design id is not a slot key: a request naming `DESIGN` (42) finds no
/// instance and is refused, not swapped.
#[tokio::test]
async fn request_ammo_change_refuses_a_design_id() {
    let capture = LogCapture::install();
    let mut mgr = player_with_weapon();
    let msgs = send(&mut mgr, &request(DESIGN, 3)).await;

    assert_eq!(
        feedback_text(&only_feedback_line(&msgs)),
        "That weapon is not in your bandolier."
    );
    assert_slot_untouched(&mgr, 0);
    assert!(capture
        .find_event(
            Level::WARN,
            "requestAmmoChange refused",
            "item_not_in_bandolier"
        )
        .is_some());
}

/// Non-positive types are refused before any lookup: 0 is the "no choice"
/// sentinel and the DB column has `CHECK (cur_ammo_type >= 0)`.
#[tokio::test]
async fn request_ammo_change_rejects_non_positive() {
    for bad in [0i32, -1, -42] {
        let mut mgr = player_with_weapon();
        let msgs = send(&mut mgr, &request(INSTANCE, bad)).await;
        assert_eq!(
            feedback_text(&only_feedback_line(&msgs)),
            "That weapon cannot use that ammo type.",
            "ammo_type {bad}"
        );
        assert_slot_untouched(&mgr, 0);
    }
}

/// A positive type the weapon's whitelist does not list is refused on the
/// first press, with a visible line and `reason=not_in_allowed_types` on
/// target `ammo` carrying the correlators.
#[tokio::test]
async fn request_ammo_change_rejects_unlisted_subtype() {
    let capture = LogCapture::install();
    let mut mgr = player_with_weapon();
    let msgs = send(&mut mgr, &request(INSTANCE, 7)).await;

    assert_eq!(
        feedback_text(&only_feedback_line(&msgs)),
        "That weapon cannot use that ammo type."
    );
    assert_slot_untouched(&mgr, 0);

    let ev = capture
        .find_event(
            Level::WARN,
            "requestAmmoChange refused",
            "not_in_allowed_types",
        )
        .expect("ammo_type_change_rejected warn");
    assert_eq!(ev.target, "ammo");
    assert!(ev.has_field("event", "ammo_type_change_rejected"), "{ev:?}");
    assert!(ev.has_field("account_id", &ACCOUNT.to_string()), "{ev:?}");
    assert!(ev.has_field("player_id", &PLAYER.to_string()), "{ev:?}");
    assert!(ev.has_field("entity_id", &ENTITY.to_string()), "{ev:?}");
    assert!(ev.has_field("ammo_type", "7"), "{ev:?}");
    assert!(ev.has_field("weapon_instance_id", &INSTANCE.to_string()));
    assert!(ev.has_field("weapon_item_id", &DESIGN.to_string()));
}

/// #448 / #602, fail closed: a bandolier weapon with **no `WeaponDef`** is
/// refused, not waved through. The old fall-open let a forged request
/// persist any positive `cur_ammo_type` (here 0x7FFFFFFE).
#[tokio::test]
async fn request_ammo_change_rejects_missing_weapon_def() {
    let capture = LogCapture::install();
    let mut mgr = player_with_weapon();
    mgr.item_defs.clear();

    let msgs = send(&mut mgr, &request(INSTANCE, 0x7FFF_FFFE)).await;

    assert_eq!(
        feedback_text(&only_feedback_line(&msgs)),
        "That weapon cannot change ammo type."
    );
    assert_slot_untouched(&mgr, 0);
    assert!(capture
        .find_event(
            Level::WARN,
            "requestAmmoChange refused",
            "weapon_def_cache_miss"
        )
        .is_some());
}

/// Two slots claiming one instance id is corrupt state (it is the
/// `sgw_inventory` PK); refuse rather than guess.
#[tokio::test]
async fn request_ammo_change_rejects_a_duplicated_instance_as_ambiguous() {
    let capture = LogCapture::install();
    let mut mgr = player_with_weapon();
    mgr.get_entity_mut(ENTITY)
        .unwrap()
        .bandolier_items
        .insert(1, weapon(INSTANCE, DESIGN));

    let msgs = send(&mut mgr, &request(INSTANCE, 3)).await;

    only_feedback_line(&msgs);
    assert_slot_untouched(&mgr, 0);
    assert_slot_untouched(&mgr, 1);
    assert!(capture
        .find_event(Level::WARN, "requestAmmoChange refused", "ambiguous_slot")
        .is_some());
}

/// Wire format: the refusal line is `onPlayerCommunication("SYSTEM", 0,
/// CHAN_FEEDBACK=9, text)`, byte for byte.
#[tokio::test]
async fn request_ammo_change_refusal_line_is_byte_exact() {
    let mut mgr = player_with_weapon();
    let args = only_feedback_line(&send(&mut mgr, &request(INSTANCE, 7)).await);

    let mut want: Vec<u8> = vec![
        0x06, 0x00, 0x00, 0x00, // speaker length: 6 UTF-16 units
        b'S', 0, b'Y', 0, b'S', 0, b'T', 0, b'E', 0, b'M', 0,    // "SYSTEM"
        0x00, // speaker flags
        0x09, // CHAN_FEEDBACK
        0x26, 0x00, 0x00, 0x00, // text length: 38 UTF-16 units
    ];
    for unit in "That weapon cannot use that ammo type.".encode_utf16() {
        want.extend_from_slice(&unit.to_le_bytes());
    }
    assert_eq!(args, want);
}
