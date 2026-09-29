//! Cell half of the AM-02 reload draw: the gate, the request, the answer,
//! and the completion tick's refill target (D-AM05 criteria 3a/3b).
//!
//! The base half (the stack arithmetic in the database, the double-spend
//! guard) is `ammo_reserve::requests_live_db_tests` in `cimmeria-base-methods`.

use std::time::Instant;

use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::ammo_type::{BULLET_DEFAULT, BULLET_HOLLOW_POINT};
use cimmeria_entity::cell_entity::BandolierItem;
use tokio::sync::mpsc;

use super::super::reload::{handle_reload_with, ABILITY_RELOAD_WEAPON};
use super::*;

const ENTITY: u32 = 1;
const SLOT: i32 = 0;
const INSTANCE: i32 = 4242;
const CLIP: i32 = 30;

/// A drawn player (no Phase A) holding one weapon in slot 0 with `current`
/// rounds of `ammo_type`. Each test passes its own `player_id` so the
/// process-wide infinite-ammo set never crosses tests.
fn mgr(player_id: i32, ammo_type: i32, current: i32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(ENTITY, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(ENTITY).unwrap();
    e.is_player = true;
    e.player_id = Some(player_id);
    e.weapon_holstered = false;
    e.active_bandolier_slot = SLOT;
    e.bandolier_items.insert(
        SLOT,
        BandolierItem {
            instance_id: INSTANCE,
            item_id: 1,
            clip_size: CLIP,
            default_ammo_type: BULLET_DEFAULT,
            current_ammo: current,
            cur_ammo_type: ammo_type,
        },
    );
    mgr.connect_entity(ENTITY);
    mgr.ability_defs.insert(
        ABILITY_RELOAD_WEAPON,
        AbilityDef {
            ability_id: ABILITY_RELOAD_WEAPON,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0.0,
            max_range: 0.0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    mgr
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

fn draws(sent: &[CellToBaseMsg]) -> Vec<&AmmoReserveRequest> {
    sent.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::AmmoReserve(r) => Some(r),
            _ => None,
        })
        .collect()
}

fn clip(mgr: &SpaceManager) -> i32 {
    mgr.get_entity(ENTITY).unwrap().bandolier_items[&SLOT].current_ammo
}

fn drawn_answer(
    player_id: i32,
    drawn: i32,
    result: Result<(), ReserveRefusal>,
) -> AmmoReserveAnswer {
    AmmoReserveAnswer::ReloadDrawn {
        entity_id: ENTITY,
        player_id,
        slot_id: SLOT,
        instance_id: INSTANCE,
        ammo_type: BULLET_HOLLOW_POINT,
        drawn,
        stack_after: 0,
        result,
    }
}

/// Run the completion tick's refill decision for the in-flight reload.
fn complete(mgr: &mut SpaceManager) -> i32 {
    let e = mgr.get_entity_mut(ENTITY).unwrap();
    let deadline = e.reload_complete_at.expect("a reload is in flight");
    let target = completion_target(e, SLOT, deadline, CLIP);
    e.set_slot_ammo(SLOT, target);
    e.reload_complete_at = None;
    e.reload_slot_id = None;
    clip(mgr)
}

/// Flag off: a special type reloads exactly as before the campaign — no
/// reserve request, a warmup, and a free refill to full.
#[tokio::test]
async fn flag_off_special_reload_is_free() {
    let mut mgr = mgr(9101, BULLET_HOLLOW_POINT, 18);
    let (tx, mut rx) = mpsc::channel(32);
    handle_reload_with(ENTITY, false, &tx, &mut mgr).await;
    assert!(
        draws(&drain(&mut rx)).is_empty(),
        "no reserve request with the flag off"
    );
    assert_eq!(complete(&mut mgr), CLIP);
}

/// Flag on, default ammo: unchanged, free refill.
#[tokio::test]
async fn default_ammo_reload_is_unchanged_with_flag_on() {
    let mut mgr = mgr(9102, BULLET_DEFAULT, 18);
    let (tx, mut rx) = mpsc::channel(32);
    handle_reload_with(ENTITY, true, &tx, &mut mgr).await;
    assert!(draws(&drain(&mut rx)).is_empty());
    assert_eq!(complete(&mut mgr), CLIP);
}

/// Flag on, special ammo: the reload flushes the clip, asks for the draw,
/// and starts no warmup until the base answers. A second press while the
/// request is in flight sends nothing (no double draw).
#[tokio::test]
async fn special_reload_asks_for_a_draw_once() {
    let player_id = 9103;
    let mut mgr = mgr(player_id, BULLET_HOLLOW_POINT, 18);
    let (tx, mut rx) = mpsc::channel(32);
    handle_reload_with(ENTITY, true, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert!(
        matches!(
            sent.first(),
            Some(CellToBaseMsg::BandolierAmmoUpdate {
                current_ammo: 18,
                cur_ammo_type: BULLET_HOLLOW_POINT,
                ..
            })
        ),
        "the clip is flushed ahead of the request: {sent:?}"
    );
    assert_eq!(
        draws(&sent),
        vec![&AmmoReserveRequest::ReloadDraw {
            entity_id: ENTITY,
            player_id,
            slot_id: SLOT,
            instance_id: INSTANCE,
            ammo_type: BULLET_HOLLOW_POINT,
            clip_before: 18,
        }]
    );
    assert!(mgr.get_entity(ENTITY).unwrap().reload_complete_at.is_none());
    assert_eq!(clip(&mgr), 18);

    handle_reload_with(ENTITY, true, &tx, &mut mgr).await;
    assert!(
        drain(&mut rx).is_empty(),
        "second press while in flight sends nothing"
    );
}

/// Criterion 3a: 18/30 with a 100-round stack → the base draws 12; the clip
/// reads 30 and the completion tick keeps it there.
#[tokio::test]
async fn full_draw_loads_twelve_and_the_tick_keeps_the_clip() {
    let player_id = 9104;
    let mut mgr = mgr(player_id, BULLET_HOLLOW_POINT, 18);
    let (tx, mut rx) = mpsc::channel(64);
    handle_reload_with(ENTITY, true, &tx, &mut mgr).await;
    drain(&mut rx);
    handle_reserve_answer(drawn_answer(player_id, 12, Ok(())), &tx, &mut mgr).await;
    assert_eq!(clip(&mgr), 30);
    assert!(
        mgr.get_entity(ENTITY).unwrap().reload_complete_at.is_some(),
        "the answer starts the warmup"
    );
    assert_eq!(complete(&mut mgr), 30);
}

/// Criterion 3b: 18/30 with a 5-round stack → 23/30. The tick must not top
/// the clip up to 30 for free (the regression this guards).
#[tokio::test]
async fn short_stack_loads_what_is_there_and_the_tick_adds_nothing() {
    let player_id = 9105;
    let mut mgr = mgr(player_id, BULLET_HOLLOW_POINT, 18);
    let (tx, mut rx) = mpsc::channel(64);
    handle_reload_with(ENTITY, true, &tx, &mut mgr).await;
    drain(&mut rx);
    handle_reserve_answer(drawn_answer(player_id, 5, Ok(())), &tx, &mut mgr).await;
    assert_eq!(clip(&mgr), 23);
    assert_eq!(
        complete(&mut mgr),
        23,
        "a short stack must not refill to clip_size"
    );
}

/// Empty stack: the reload is refused with `onErrorCode` (byte-exact) and a
/// feedback line; the clip is unchanged and no warmup starts. The request
/// is no longer in flight, so the next press asks again.
#[tokio::test]
async fn empty_stack_refuses_with_feedback_and_leaves_the_clip() {
    let player_id = 9106;
    let mut mgr = mgr(player_id, BULLET_HOLLOW_POINT, 18);
    let (tx, mut rx) = mpsc::channel(64);
    handle_reload_with(ENTITY, true, &tx, &mut mgr).await;
    drain(&mut rx);
    handle_reserve_answer(
        drawn_answer(player_id, 0, Err(ReserveRefusal::StackEmpty)),
        &tx,
        &mut mgr,
    )
    .await;
    let sent = drain(&mut rx);
    let error_args: Vec<&Vec<u8>> = sent
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: ENTITY,
                method_index,
                args,
            } if *method_index == crate::mercury::method_idx::ON_ERROR_CODE => Some(args),
            _ => None,
        })
        .collect();
    // ErrorSystem u8 = 0 (Ability), InstanceID i32 = 596, ErrorCode u16 = 61.
    assert_eq!(error_args, vec![&vec![0u8, 0x54, 0x02, 0, 0, 61, 0]]);
    let line = serialize_on_player_communication(
        "SYSTEM",
        0,
        CHAN_FEEDBACK,
        "You have no Hollow Point rounds left.",
    );
    let feedback = sent.iter().any(|m| {
        matches!(m, CellToBaseMsg::EntityMethodCall { method_index, args, .. }
            if *method_index == crate::mercury::method_idx::ON_PLAYER_COMMUNICATION
                && *args == line)
    });
    assert!(feedback, "a feedback line names the empty ammo: {sent:?}");
    assert_eq!(clip(&mgr), 18);
    assert!(mgr.get_entity(ENTITY).unwrap().reload_complete_at.is_none());

    handle_reload_with(ENTITY, true, &tx, &mut mgr).await;
    assert_eq!(draws(&drain(&mut rx)).len(), 1, "the next press asks again");
}

/// D-AM09: with infinite ammo on, a special reload draws nothing and the
/// clip still reaches full through the ordinary reload.
#[tokio::test]
async fn infinite_ammo_skips_the_draw_and_still_fills_the_clip() {
    let player_id = 9107;
    cimmeria_entity::ammo_infinite::set(player_id, true);
    let mut mgr = mgr(player_id, BULLET_HOLLOW_POINT, 0);
    let (tx, mut rx) = mpsc::channel(32);
    handle_reload_with(ENTITY, true, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    cimmeria_entity::ammo_infinite::set(player_id, false);
    assert!(draws(&sent).is_empty(), "infinite ammo draws nothing");
    assert_eq!(complete(&mut mgr), CLIP);
}

/// Rounds drawn while the player swapped weapons still land in the drawn
/// slot (never lost), with no warmup on the new active slot.
#[tokio::test]
async fn answer_after_a_slot_swap_still_loads_the_rounds() {
    let player_id = 9108;
    let mut mgr = mgr(player_id, BULLET_HOLLOW_POINT, 18);
    let (tx, mut rx) = mpsc::channel(64);
    handle_reload_with(ENTITY, true, &tx, &mut mgr).await;
    drain(&mut rx);
    mgr.get_entity_mut(ENTITY).unwrap().active_bandolier_slot = 1;
    handle_reserve_answer(drawn_answer(player_id, 12, Ok(())), &tx, &mut mgr).await;
    assert_eq!(clip(&mgr), 30);
    assert!(mgr.get_entity(ENTITY).unwrap().reload_complete_at.is_none());
}

/// A stale `preloaded` marker never suppresses a later free reload's refill.
#[test]
fn completion_target_ignores_a_stale_marker() {
    let mut mgr = mgr(9109, BULLET_DEFAULT, 5);
    let stale = Instant::now();
    reserve_state_mut(&mut mgr, ENTITY).unwrap().preloaded = Some((SLOT, stale));
    let e = mgr.get_entity_mut(ENTITY).unwrap();
    let later = stale + std::time::Duration::from_millis(1);
    assert_eq!(completion_target(e, SLOT, later, CLIP), CLIP);
}

#[test]
fn ammo_name_drops_the_family_prefix() {
    assert_eq!(ammo_name(BULLET_HOLLOW_POINT), "Hollow Point");
    assert_eq!(
        ammo_name(cimmeria_entity::ammo_type::BULLET_ARMOR_PIERCING),
        "Armor Piercing"
    );
}
