//! Tests for the pulsing lifecycle — registration, channel
//! cancellation / movement-interrupt, and the per-cell pulse tick.
//!
//! Kept in one module because the test harness (`make_mgr`,
//! `make_dot_effect`) is shared across all three seams, and several
//! scenarios cut across them (e.g. a cancel-channel test first registers
//! via `register_active_effect`).

use super::{
    cancel_channels_from_attacker, channel_interrupt_on_movement_tick, effect_pulse_tick,
    register_active_effect, MAX_CHANNEL_DURATION_SECS,
};

use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::HEALTH;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "W", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        if let Some(s) = e.stats.get_mut(HEALTH) {
            s.update(0, 100, 100);
        }
    }
    if let Some(e) = mgr.get_entity_mut(2) {
        if let Some(s) = e.stats.get_mut(HEALTH) {
            s.update(0, 100, 100);
        }
    }
    mgr
}

fn make_dot_effect(pulse_count: i32, pulse_secs: f32, dmg: i32) -> EffectDef {
    let mut params = HashMap::new();
    params.insert("HealthDamage".to_string(), dmg.to_string());
    EffectDef {
        effect_id: 7777,
        ability_id: 1234,
        pulse_count,
        pulse_duration: pulse_secs,
        params,
        ..Default::default()
    }
}

#[tokio::test]
async fn register_pulsing_effect_stores_remaining_pulse_count_minus_one() {
    let mut mgr = make_mgr();
    let effect = make_dot_effect(5, 1.0, 10);
    let (tx, _rx) = mpsc::channel(64);
    let now = Instant::now();
    let registered = register_active_effect(&mut mgr, 2, 1, &effect, now, &tx).await;
    assert!(registered);
    let active = &mgr.get_entity(2).unwrap().active_effects;
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].remaining_pulses, 4);
    assert_eq!(active[0].total_pulses, 5);
}

#[tokio::test]
async fn register_single_shot_effect_does_not_register() {
    let mut mgr = make_mgr();
    let effect = make_dot_effect(1, 0.0, 10);
    let (tx, _rx) = mpsc::channel(64);
    let registered = register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
    assert!(!registered);
    assert!(mgr.get_entity(2).unwrap().active_effects.is_empty());
}

#[tokio::test]
async fn register_channelled_effect_uses_time_based_safety_cap() {
    // pulse_count=0 → register with ceil(MAX_CHANNEL_DURATION_SECS /
    // pulse_duration) pulses. At 0.5s/pulse over 30s: 60 pulses.
    let mut mgr = make_mgr();
    let effect = make_dot_effect(0, 0.5, 10);
    let (tx, _rx) = mpsc::channel(64);
    let registered = register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
    assert!(
        registered,
        "channelled effect now registers with safety cap"
    );
    let active = &mgr.get_entity(2).unwrap().active_effects;
    assert_eq!(active.len(), 1);
    let expected_cap = (MAX_CHANNEL_DURATION_SECS / 0.5).ceil() as i32;
    assert_eq!(active[0].total_pulses, expected_cap);
    assert_eq!(active[0].remaining_pulses, expected_cap - 1);
}

#[tokio::test]
async fn refresh_preserves_higher_remaining_pulses() {
    // Clara G7: a refresh must NOT shorten the existing instance.
    // 10-pulse DoT at 8 remaining, refreshed by a 5-pulse cast →
    // should stay at 8 remaining, not drop to 4.
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    let mut long_effect = make_dot_effect(10, 0.5, 5);
    long_effect.effect_id = 1234;
    register_active_effect(&mut mgr, 2, 1, &long_effect, Instant::now(), &tx).await;
    // Burn no pulses: remaining = 9 (10 - 1 already fired).
    let remaining_before = mgr.get_entity(2).unwrap().active_effects[0].remaining_pulses;
    assert_eq!(remaining_before, 9);

    // Refresh with a shorter 5-pulse cast of the SAME effect.
    let mut short_effect = make_dot_effect(5, 0.5, 5);
    short_effect.effect_id = 1234;
    register_active_effect(&mut mgr, 2, 1, &short_effect, Instant::now(), &tx).await;

    let remaining_after = mgr.get_entity(2).unwrap().active_effects[0].remaining_pulses;
    assert_eq!(
        remaining_after, 9,
        "refresh must preserve higher remaining_pulses (was 9, refresh proposed 4)"
    );
}

#[tokio::test]
async fn channel_interrupt_uses_planar_distance_only() {
    // Clara G8: jumping in place (Y-axis change) must NOT interrupt.
    use cimmeria_entity::abilities::AbilityDef;
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    let mut channel_effect = make_dot_effect(0, 0.5, 5);
    channel_effect.effect_id = 1300;
    channel_effect.ability_id = 1301;
    mgr.effect_defs.insert(1300, channel_effect.clone());
    mgr.ability_defs.insert(
        1301,
        AbilityDef {
            ability_id: 1301,
            name: "Test".to_string(),
            cooldown: 0.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 50,
            target_type_id: 0,
            effect_ids: vec![1300],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    register_active_effect(&mut mgr, 2, 1, &channel_effect, Instant::now(), &tx).await;

    // Move invoker UP only (jump) — Y delta = 5, but X/Z unchanged.
    if let Some(inv) = mgr.get_entity_mut(1) {
        inv.position.y = 5.0;
    }
    let cancelled = channel_interrupt_on_movement_tick(&tx, &mut mgr).await;
    assert_eq!(
        cancelled, 0,
        "vertical-only movement must not interrupt — planar distance only"
    );
}

#[tokio::test]
async fn channelled_safety_cap_scales_to_pulse_duration() {
    // Regression for Clara G2: a 0.1s-pulse channel must still get
    // ~30 s of wallclock cap, not the ~6 s that a fixed pulse-count
    // would have given it.
    let mut mgr = make_mgr();
    let effect = make_dot_effect(0, 0.1, 5);
    let (tx, _rx) = mpsc::channel(64);
    register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
    let total = mgr.get_entity(2).unwrap().active_effects[0].total_pulses;
    // At 0.1s × 300 pulses = 30 s — close to MAX_CHANNEL_DURATION_SECS.
    assert!(
        total >= 300,
        "0.1s channel must cap by time (~300 pulses), got {total}"
    );
}

#[tokio::test]
async fn cancel_channels_from_attacker_drops_channeled_keeps_finite_dot() {
    // Phase J: when an attacker fires a different ability, channels
    // get cancelled but finite DoTs from the same attacker keep ticking.
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    let now = Instant::now();
    // Effect 1: pulse_count=0 → channelled
    let mut channel_effect = make_dot_effect(0, 0.5, 10);
    channel_effect.effect_id = 9001;
    channel_effect.ability_id = 100;
    mgr.effect_defs.insert(9001, channel_effect.clone());
    // Effect 2: pulse_count=5 → finite DoT
    let mut dot_effect = make_dot_effect(5, 1.0, 10);
    dot_effect.effect_id = 9002;
    dot_effect.ability_id = 200;
    mgr.effect_defs.insert(9002, dot_effect.clone());

    register_active_effect(&mut mgr, 2, 1, &channel_effect, now, &tx).await;
    register_active_effect(&mut mgr, 2, 1, &dot_effect, now, &tx).await;
    assert_eq!(mgr.get_entity(2).unwrap().active_effects.len(), 2);

    // Attacker fires a totally different ability (id 300). Channel
    // should die; DoT should survive.
    let cancelled = cancel_channels_from_attacker(1, Some(300), &tx, &mut mgr).await;
    assert_eq!(cancelled, 1, "exactly one channel cancelled");
    let active = &mgr.get_entity(2).unwrap().active_effects;
    assert_eq!(active.len(), 1, "DoT survived");
    assert_eq!(active[0].effect_id, 9002, "finite DoT kept");
}

#[tokio::test]
async fn cancel_channels_from_attacker_with_keep_ability_id_skips_matching() {
    // Phase J: same-ability re-fire keeps the channel alive (refresh path).
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    let mut channel_effect = make_dot_effect(0, 0.5, 10);
    channel_effect.effect_id = 9003;
    channel_effect.ability_id = 100;
    mgr.effect_defs.insert(9003, channel_effect.clone());
    register_active_effect(&mut mgr, 2, 1, &channel_effect, Instant::now(), &tx).await;

    // Re-fire the SAME ability — cancel sweep should keep this channel.
    let cancelled = cancel_channels_from_attacker(1, Some(100), &tx, &mut mgr).await;
    assert_eq!(cancelled, 0, "same-ability re-fire keeps channel");
    assert_eq!(mgr.get_entity(2).unwrap().active_effects.len(), 1);
}

#[tokio::test]
async fn channel_interrupt_fires_when_invoker_moves_past_threshold() {
    // Phase K: invoker at origin starts a channel; tick fires while
    // they're still at origin → no interrupt. Move them past 0.5m →
    // tick interrupts.
    use cimmeria_entity::abilities::AbilityDef;
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    let mut channel_effect = make_dot_effect(0, 0.5, 5);
    channel_effect.effect_id = 9100;
    channel_effect.ability_id = 500;
    mgr.effect_defs.insert(9100, channel_effect.clone());
    mgr.ability_defs.insert(
        500,
        AbilityDef {
            ability_id: 500,
            name: "TestChannel".to_string(),
            cooldown: 0.0,
            warmup: 0.0,
            flags: 0, // default = cancel-on-move
            is_ranged: true,
            min_range: 0,
            max_range: 50,
            target_type_id: 0,
            effect_ids: vec![9100],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    register_active_effect(&mut mgr, 2, 1, &channel_effect, Instant::now(), &tx).await;
    assert_eq!(mgr.get_entity(2).unwrap().active_effects.len(), 1);

    // Invoker hasn't moved → no interrupt.
    let cancelled = channel_interrupt_on_movement_tick(&tx, &mut mgr).await;
    assert_eq!(cancelled, 0, "stationary invoker keeps channel");
    assert_eq!(mgr.get_entity(2).unwrap().active_effects.len(), 1);

    // Move invoker past threshold.
    if let Some(inv) = mgr.get_entity_mut(1) {
        inv.position.x = 1.5; // > 0.5m
    }
    let cancelled = channel_interrupt_on_movement_tick(&tx, &mut mgr).await;
    assert_eq!(cancelled, 1, "moved invoker interrupted");
    assert!(mgr.get_entity(2).unwrap().active_effects.is_empty());
}

#[tokio::test]
async fn channel_interrupt_respects_af_channel_allows_movement_flag() {
    // Phase K: ability with AF_CHANNEL_ALLOWS_MOVEMENT survives a
    // movement event that would otherwise interrupt.
    use cimmeria_entity::abilities::{AbilityDef, AF_CHANNEL_ALLOWS_MOVEMENT};
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    let mut channel_effect = make_dot_effect(0, 0.5, 5);
    channel_effect.effect_id = 9101;
    channel_effect.ability_id = 501;
    mgr.effect_defs.insert(9101, channel_effect.clone());
    mgr.ability_defs.insert(
        501,
        AbilityDef {
            ability_id: 501,
            name: "MovingChannel".to_string(),
            cooldown: 0.0,
            warmup: 0.0,
            flags: AF_CHANNEL_ALLOWS_MOVEMENT,
            is_ranged: true,
            min_range: 0,
            max_range: 50,
            target_type_id: 0,
            effect_ids: vec![9101],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    register_active_effect(&mut mgr, 2, 1, &channel_effect, Instant::now(), &tx).await;
    // Move invoker far past threshold.
    if let Some(inv) = mgr.get_entity_mut(1) {
        inv.position.x = 50.0;
    }
    let cancelled = channel_interrupt_on_movement_tick(&tx, &mut mgr).await;
    assert_eq!(cancelled, 0, "AF_CHANNEL_ALLOWS_MOVEMENT exempts channel");
    assert_eq!(
        mgr.get_entity(2).unwrap().active_effects.len(),
        1,
        "channel survives movement"
    );
}

#[tokio::test]
async fn channel_interrupt_below_threshold_does_not_fire() {
    // Phase K: small position jitter (< 0.5m) doesn't interrupt.
    use cimmeria_entity::abilities::AbilityDef;
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    let mut channel_effect = make_dot_effect(0, 0.5, 5);
    channel_effect.effect_id = 9102;
    channel_effect.ability_id = 502;
    mgr.effect_defs.insert(9102, channel_effect.clone());
    mgr.ability_defs.insert(
        502,
        AbilityDef {
            ability_id: 502,
            name: "TestChannel2".to_string(),
            cooldown: 0.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 50,
            target_type_id: 0,
            effect_ids: vec![9102],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    register_active_effect(&mut mgr, 2, 1, &channel_effect, Instant::now(), &tx).await;
    // Move invoker BUT only 0.3m (under threshold of 0.5m)
    if let Some(inv) = mgr.get_entity_mut(1) {
        inv.position.x = 0.3;
    }
    let cancelled = channel_interrupt_on_movement_tick(&tx, &mut mgr).await;
    assert_eq!(cancelled, 0, "below-threshold jitter must not interrupt");
    assert_eq!(mgr.get_entity(2).unwrap().active_effects.len(), 1);
}

#[tokio::test]
async fn finite_dot_does_not_interrupt_on_invoker_movement() {
    // Phase K: only channels (pulse_count == 0) carry the position
    // anchor. Finite DoTs from the same invoker keep ticking when
    // the invoker moves.
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    let mut dot_effect = make_dot_effect(5, 1.0, 10);
    dot_effect.effect_id = 9103;
    dot_effect.ability_id = 503;
    mgr.effect_defs.insert(9103, dot_effect.clone());
    register_active_effect(&mut mgr, 2, 1, &dot_effect, Instant::now(), &tx).await;
    if let Some(inv) = mgr.get_entity_mut(1) {
        inv.position.x = 50.0;
    }
    let cancelled = channel_interrupt_on_movement_tick(&tx, &mut mgr).await;
    assert_eq!(cancelled, 0, "finite DoTs are immune to movement interrupt");
}

#[tokio::test]
async fn cancel_channels_from_attacker_no_keep_cancels_all() {
    // Phase J: cancel-all path (called from death-transition) drops
    // every channel regardless of ability.
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    let mut channel_effect = make_dot_effect(0, 0.5, 10);
    channel_effect.effect_id = 9004;
    channel_effect.ability_id = 100;
    mgr.effect_defs.insert(9004, channel_effect.clone());
    register_active_effect(&mut mgr, 2, 1, &channel_effect, Instant::now(), &tx).await;

    let cancelled = cancel_channels_from_attacker(1, None, &tx, &mut mgr).await;
    assert_eq!(cancelled, 1);
    assert!(mgr.get_entity(2).unwrap().active_effects.is_empty());
}

#[tokio::test]
async fn register_same_invoker_refreshes_existing_instance() {
    // Phase E stacking semantics: same source re-cast = refresh duration.
    let mut mgr = make_mgr();
    let effect = make_dot_effect(5, 1.0, 10);
    let (tx, _rx) = mpsc::channel(64);
    register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
    // Burn a few pulses so we can detect a refresh.
    if let Some(t) = mgr.get_entity_mut(2) {
        if let Some(inst) = t.active_effects.first_mut() {
            inst.remaining_pulses = 1;
        }
    }
    register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
    let active = &mgr.get_entity(2).unwrap().active_effects;
    assert_eq!(active.len(), 1, "same-source re-cast must NOT stack");
    assert_eq!(
        active[0].remaining_pulses, 4,
        "remaining_pulses refreshed to 4 (5 total - 1 just fired)"
    );
}

#[tokio::test]
async fn register_different_invoker_stacks_separate_instance() {
    // Phase E stacking semantics: different source = stack.
    let mut mgr = make_mgr();
    let effect = make_dot_effect(5, 1.0, 10);
    let (tx, _rx) = mpsc::channel(64);
    register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
    // A different invoker (id 99) lands the same effect_id.
    register_active_effect(&mut mgr, 2, 99, &effect, Instant::now(), &tx).await;
    let active = &mgr.get_entity(2).unwrap().active_effects;
    assert_eq!(active.len(), 2, "different invokers must stack");
    let invokers: Vec<u32> = active.iter().map(|i| i.invoker_id).collect();
    assert!(invokers.contains(&1));
    assert!(invokers.contains(&99));
}

#[tokio::test]
async fn pulse_tick_fires_due_pulse_and_decrements_remaining() {
    let mut mgr = make_mgr();
    let effect = make_dot_effect(5, 1.0, 10);
    mgr.effect_defs.insert(effect.effect_id, effect.clone());
    let past = Instant::now() - Duration::from_secs(2);
    let (tx, _rx) = mpsc::channel(64);
    register_active_effect(&mut mgr, 2, 1, &effect, past, &tx).await;
    if let Some(t) = mgr.get_entity_mut(2) {
        if let Some(inst) = t.active_effects.first_mut() {
            inst.next_pulse_at = past;
        }
    }
    let hp_before = mgr.get_entity(2).unwrap().stats.get(HEALTH).unwrap().cur;
    effect_pulse_tick(&tx, &mut mgr).await;
    let hp_after = mgr.get_entity(2).unwrap().stats.get(HEALTH).unwrap().cur;
    assert_eq!(hp_after, hp_before - 10, "DoT pulse should subtract 10");
    let active = &mgr.get_entity(2).unwrap().active_effects;
    assert_eq!(active[0].remaining_pulses, 3);
}

#[tokio::test]
async fn pulse_tick_removes_instance_when_remaining_hits_zero() {
    let mut mgr = make_mgr();
    let mut effect = make_dot_effect(2, 1.0, 10);
    effect.effect_id = 8888;
    mgr.effect_defs.insert(effect.effect_id, effect.clone());
    let (tx, _rx) = mpsc::channel(64);
    register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
    if let Some(t) = mgr.get_entity_mut(2) {
        if let Some(inst) = t.active_effects.first_mut() {
            inst.next_pulse_at = Instant::now() - Duration::from_secs(2);
            inst.remaining_pulses = 1;
        }
    }
    effect_pulse_tick(&tx, &mut mgr).await;
    let active = &mgr.get_entity(2).unwrap().active_effects;
    assert!(
        active.is_empty(),
        "instance removed after its last pulse fires"
    );
}

#[tokio::test]
async fn pulse_tick_skips_pulse_on_dead_target() {
    let mut mgr = make_mgr();
    let effect = make_dot_effect(5, 1.0, 10);
    mgr.effect_defs.insert(effect.effect_id, effect.clone());
    let (tx, _rx) = mpsc::channel(64);
    register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
    if let Some(t) = mgr.get_entity_mut(2) {
        if let Some(s) = t.stats.get_mut(HEALTH) {
            s.update(0, 0, 100);
        }
        if let Some(inst) = t.active_effects.first_mut() {
            inst.next_pulse_at = Instant::now() - Duration::from_secs(2);
        }
    }
    effect_pulse_tick(&tx, &mut mgr).await;
    let hp = mgr.get_entity(2).unwrap().stats.get(HEALTH).unwrap().cur;
    assert_eq!(hp, 0);
    let remaining = mgr.get_entity(2).unwrap().active_effects[0].remaining_pulses;
    assert_eq!(remaining, 3);
}
