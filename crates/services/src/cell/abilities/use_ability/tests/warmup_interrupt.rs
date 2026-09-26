//! AT-10: what interrupts a warmup, and what an interrupt sends.
//!
//! One test per trigger. Each proves the cast never fires (no hit even
//! after the warmup would have expired) and, where it matters, what the
//! client is told. Fixture and helpers are in `warmup.rs`.

use std::time::Instant;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::abilities::{serialize_timer_update, AF_CHANNEL_ALLOWS_MOVEMENT};
use cimmeria_entity::cell_entity::BandolierItem;

use super::warmup::{
    after_warmup, calls, effect_results, sequences, warmup_mgr, ON_TIMER_UPDATE, SEQ_INTERRUPT,
    WARMUP_ABILITY,
};
use super::*;
use crate::cell::abilities::resolve_warmups;

/// Launch ability 50 from player 1 at NPC 2 and discard the launch burst.
async fn launch(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) {
    assert!(handle_use_ability(1, WARMUP_ABILITY, 2, tx, mgr).await);
    drain(rx);
    assert!(mgr.get_entity(1).unwrap().pending_cast.is_some());
}

/// The cast is gone and stays gone: no hit even once the warmup would have
/// expired.
async fn assert_never_fires(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) {
    let engine = ChainEngine::new();
    assert!(mgr.get_entity(1).unwrap().pending_cast.is_none());
    assert_eq!(resolve_warmups(after_warmup(), tx, mgr, &engine).await, 0);
    assert_eq!(effect_results(&drain(rx), 1), 0, "an interrupted cast hit");
}

/// Moving off the spot interrupts the warmup (the channel rule, AT-10
/// deviation from python, which had no movement interrupt). Pins the whole
/// interrupt burst byte for byte: the python zeroed `AbilityWarmup` timer,
/// the zeroed cooldown timer that matches the refund, then
/// `Ability_Interrupt` with the launch's `InstanceId`. The cooldown is
/// refunded (python `interruptAbility`).
#[tokio::test]
async fn moving_interrupts_the_warmup_and_refunds_the_cooldown() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;
    let effect_seq = mgr
        .get_entity(1)
        .unwrap()
        .pending_cast
        .as_ref()
        .unwrap()
        .effect_seq;

    mgr.get_entity_mut(1).unwrap().position.x += 1.0;
    resolve_warmups(Instant::now(), &tx, &mut mgr, &engine).await;
    let c = calls(&drain(&mut rx));

    let mut interrupt = Vec::new();
    interrupt.extend_from_slice(&SEQ_INTERRUPT.to_le_bytes());
    interrupt.extend_from_slice(&1i32.to_le_bytes());
    interrupt.extend_from_slice(&2i32.to_le_bytes());
    interrupt.push(1);
    interrupt.extend_from_slice(&0.0f32.to_le_bytes());
    interrupt.extend_from_slice(&0u32.to_le_bytes());
    interrupt.push(0);
    interrupt.extend_from_slice(&effect_seq.to_le_bytes());
    assert_eq!(
        c,
        vec![
            (
                1,
                ON_TIMER_UPDATE,
                serialize_timer_update(WARMUP_ABILITY, 1, 1, 0, 0.0, 0.0)
            ),
            (
                1,
                ON_TIMER_UPDATE,
                serialize_timer_update(WARMUP_ABILITY, 2, 1, 0, 0.0, 0.0)
            ),
            (1, method_idx::ON_SEQUENCE, interrupt),
        ]
    );
    assert!(
        !mgr.get_entity(1)
            .unwrap()
            .abilities
            .is_on_cooldown(WARMUP_ABILITY),
        "the interrupt refunds the cooldown"
    );
    assert!(mgr.pending_casts.is_empty());
    assert_never_fires(&mut mgr, &tx, &mut rx).await;
}

/// Standing still, or an ability flagged to allow movement, does not
/// interrupt: the movement rule has the channel's exemption.
#[tokio::test]
async fn movement_allowed_flag_keeps_the_warmup() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    mgr.ability_defs.get_mut(&WARMUP_ABILITY).unwrap().flags |= AF_CHANNEL_ALLOWS_MOVEMENT;
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;

    mgr.get_entity_mut(1).unwrap().position.x += 1.0;
    resolve_warmups(Instant::now(), &tx, &mut mgr, &engine).await;
    assert!(mgr.get_entity(1).unwrap().pending_cast.is_some());
    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    assert_eq!(effect_results(&drain(&mut rx), 1), 1);
}

/// The caster dying interrupts the warmup (python `onDead` →
/// `interruptAbility`), from the death transition itself.
#[tokio::test]
async fn caster_death_interrupts_the_warmup() {
    let mut mgr = warmup_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;

    assert!(crate::cell::abilities::resolve_death_for_test(1, 2, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert!(
        sequences(&msgs, 1).contains(&SEQ_INTERRUPT),
        "Ability_Interrupt on death; got {msgs:?}"
    );
    assert_never_fires(&mut mgr, &tx, &mut rx).await;
}

/// Changing the active bandolier slot interrupts the warmup (python
/// `onActiveSlotChanged` → `onBandolierSlotChange` → `interruptAbility`).
/// A request for the slot already active does not.
#[tokio::test]
async fn bandolier_slot_change_interrupts_the_warmup() {
    let mut mgr = warmup_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;

    let slot_args = |wire_slot: i32| {
        let mut a = 3i32.to_le_bytes().to_vec();
        a.extend_from_slice(&wire_slot.to_le_bytes());
        a
    };
    // Wire slot 1 is server slot 0, the active one: not a change.
    crate::cell::cell_methods::inventory::handle_request_active_slot_change(
        1,
        &slot_args(1),
        &tx,
        &mut mgr,
    )
    .await;
    assert!(mgr.get_entity(1).unwrap().pending_cast.is_some());

    crate::cell::cell_methods::inventory::handle_request_active_slot_change(
        1,
        &slot_args(2),
        &tx,
        &mut mgr,
    )
    .await;
    assert!(sequences(&drain(&mut rx), 1).contains(&SEQ_INTERRUPT));
    assert_never_fires(&mut mgr, &tx, &mut rx).await;
}

/// The target dying during the warmup interrupts the cast at fire time
/// rather than hitting a corpse (AT-10 deviation: python's `afterWarmup`
/// re-checked nothing).
#[tokio::test]
async fn target_death_during_warmup_interrupts_at_fire() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;

    mgr.get_entity_mut(2)
        .unwrap()
        .set_state_flag(crate::cell::combat::BSF_DEAD);
    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    let msgs = drain(&mut rx);
    assert_eq!(effect_results(&msgs, 1), 0, "no hit on the corpse");
    assert!(sequences(&msgs, 1).contains(&SEQ_INTERRUPT));
    assert_never_fires(&mut mgr, &tx, &mut rx).await;
}

/// A target that leaves range during the warmup interrupts the cast at
/// fire time, with the launch's out-of-range error (code 42).
#[tokio::test]
async fn target_out_of_range_at_fire_interrupts_with_error_42() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;

    mgr.get_entity_mut(2).unwrap().position.x = 100.0;
    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    let msgs = drain(&mut rx);
    assert_eq!(effect_results(&msgs, 1), 0);
    let mut err = vec![0u8];
    err.extend_from_slice(&WARMUP_ABILITY.to_le_bytes());
    err.extend_from_slice(&42u16.to_le_bytes());
    assert!(
        calls(&msgs).contains(&(1, method_idx::ON_ERROR_CODE, err)),
        "onErrorCode 42; got {msgs:?}"
    );
    assert!(sequences(&msgs, 1).contains(&SEQ_INTERRUPT));
    assert_never_fires(&mut mgr, &tx, &mut rx).await;
}

/// A target that steps behind a wall during the warmup interrupts the cast
/// at fire time, with the launch's line-of-sight error (code 39).
#[tokio::test]
async fn target_behind_a_wall_at_fire_interrupts_with_error_39() {
    use crate::cell::space_manager::occluder_fixtures::corner;

    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    let sid = mgr.get_entity_space_id(1).unwrap();
    mgr.spaces.get_mut(&sid).unwrap().occluder = Some(corner());
    // The corner fixture: a wall on x 19.85-20.15 ending at z 20. From
    // (5, 10) a target at (21, 22) is in the open and one at (21, 18) is
    // two metres into the wall's shadow.
    mgr.get_entity_mut(1).unwrap().position = cimmeria_common::Vector3::new(5.0, 0.0, 10.0);
    mgr.get_entity_mut(2).unwrap().position = cimmeria_common::Vector3::new(21.0, 0.0, 22.0);
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;

    mgr.get_entity_mut(2).unwrap().position = cimmeria_common::Vector3::new(21.0, 0.0, 18.0);
    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    let msgs = drain(&mut rx);
    assert_eq!(effect_results(&msgs, 1), 0);
    let mut err = vec![0u8];
    err.extend_from_slice(&WARMUP_ABILITY.to_le_bytes());
    err.extend_from_slice(&39u16.to_le_bytes());
    assert!(
        calls(&msgs).contains(&(1, method_idx::ON_ERROR_CODE, err)),
        "onErrorCode 39; got {msgs:?}"
    );
    assert_never_fires(&mut mgr, &tx, &mut rx).await;
}

/// A weapon warmup whose magazine starts reloading during the warmup is
/// interrupted at fire time instead of spending ammo the reload is about
/// to overwrite.
#[tokio::test]
async fn reload_during_a_weapon_warmup_interrupts_at_fire() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    mgr.ability_defs
        .get_mut(&WARMUP_ABILITY)
        .unwrap()
        .required_ammo = 1;
    if let Some(p) = mgr.get_entity_mut(1) {
        p.weapon_holstered = false;
        p.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 30,
                cur_ammo_type: 2,
            },
        );
    }
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;
    assert_eq!(
        mgr.get_entity(1).unwrap().active_ammo(),
        30,
        "ammo is spent at the fire, not at launch (python afterWarmup)"
    );

    mgr.get_entity_mut(1).unwrap().reload_complete_at = Some(after_warmup());
    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    assert_eq!(effect_results(&drain(&mut rx), 1), 0);
    assert_eq!(
        mgr.get_entity(1).unwrap().active_ammo(),
        30,
        "no ammo spent"
    );
    mgr.get_entity_mut(1).unwrap().reload_complete_at = None;
    assert_never_fires(&mut mgr, &tx, &mut rx).await;
}

/// An interrupt of the auto-cycle ability stops the loop, so the refunded
/// cooldown does not relaunch the warmup on the next tick.
#[tokio::test]
async fn interrupting_the_auto_cycle_ability_stops_the_loop() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    mgr.get_entity_mut(1).unwrap().abilities.auto_cycle = true;
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;
    assert_eq!(
        mgr.get_entity(1).unwrap().abilities.auto_cycle_ability_id,
        Some(WARMUP_ABILITY)
    );

    mgr.get_entity_mut(1).unwrap().position.x += 1.0;
    resolve_warmups(Instant::now(), &tx, &mut mgr, &engine).await;
    let abilities = &mgr.get_entity(1).unwrap().abilities;
    assert!(!abilities.auto_cycle, "the loop is cleared");
    assert_eq!(abilities.auto_cycle_ability_id, None);
}

/// A weapon swapped into the active slot through the inventory (not a
/// `requestActiveSlotChange`) interrupts the cast at fire time, so the new
/// weapon never pays for the old one's shot.
#[tokio::test]
async fn weapon_swapped_into_the_active_slot_interrupts_at_fire() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    let weapon = |instance_id| BandolierItem {
        instance_id,
        item_id: 1,
        clip_size: 30,
        default_ammo_type: 2,
        current_ammo: 30,
        cur_ammo_type: 2,
    };
    if let Some(p) = mgr.get_entity_mut(1) {
        p.weapon_holstered = false;
        p.bandolier_items.insert(0, weapon(700));
    }
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;

    mgr.get_entity_mut(1)
        .unwrap()
        .bandolier_items
        .insert(0, weapon(701));
    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    let msgs = drain(&mut rx);
    assert_eq!(
        effect_results(&msgs, 1),
        0,
        "the swapped-in weapon fired the old shot"
    );
    assert!(sequences(&msgs, 1).contains(&SEQ_INTERRUPT));
    assert_never_fires(&mut mgr, &tx, &mut rx).await;
}

/// A cast never fires in another space, even for an ability that allows
/// movement (the movement exemption does not cover a space change).
#[tokio::test]
async fn a_space_change_interrupts_even_a_movement_allowed_warmup() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    mgr.ability_defs.get_mut(&WARMUP_ABILITY).unwrap().flags |= AF_CHANNEL_ALLOWS_MOVEMENT;
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;

    let other = cimmeria_common::SpaceId(mgr.get_entity(1).unwrap().space_id.0 + 1);
    mgr.get_entity_mut(1).unwrap().space_id = other;
    resolve_warmups(Instant::now(), &tx, &mut mgr, &engine).await;
    assert!(sequences(&drain(&mut rx), 1).contains(&SEQ_INTERRUPT));
    assert_never_fires(&mut mgr, &tx, &mut rx).await;
}

/// The launch's #444 rule holds at fire time: an NPC that turned
/// non-hostile during the warmup is not hit.
#[tokio::test]
async fn a_target_turned_friendly_during_the_warmup_is_not_hit() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(256);
    launch(&mut mgr, &tx, &mut rx).await;

    mgr.get_entity_mut(2).unwrap().faction = 0;
    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    assert_eq!(effect_results(&drain(&mut rx), 1), 0);
    assert_never_fires(&mut mgr, &tx, &mut rx).await;
}
