//! The stat-buff tick: the byte-exact duration timers, expiry restoring the
//! stat, the replacement's clear, the death strip, and AB-04's ability
//! entries (per-source timers, held entries, the strip seam).

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_entity::abilities::EF_CLEAR_ON_DEATH;
use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::{ACCURACY, COORDINATION, ENGAGEMENT};

use super::*;
use crate::cell::client_methods::being::ON_TIMER_UPDATE;
use crate::mercury::game_clock::{game_time_secs, init};

const PLAYER: u32 = 1;

fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(PLAYER, "W", [0.0; 3], [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(PLAYER).unwrap();
    e.is_player = true;
    e.player_id = Some(100);
    // Coordination at its archetype value, cur == max.
    e.stats.get_mut(COORDINATION).unwrap().update(0, 10, 10);
    e.stats.get_mut(ENGAGEMENT).unwrap().update(0, 12, 12);
    e.stats.clear_dirty();
    mgr
}

fn stim(stat_id: i32, delta: i32, effect_id: i32, flags: u32) -> TimedEffectSpec {
    TimedEffectSpec {
        effect_id,
        ability_id: 2735,
        invoker_id: PLAYER,
        effect_flags: flags,
        moniker_ids: vec![],
        stats: vec![(stat_id, delta)],
        duration_secs: Some(3600.0),
        stacking: TimedStacking::ReplaceSameStat,
        invoker_identity: Default::default(),
    }
}

/// Aim (effect 700, flags 21) cast by `invoker` on `PLAYER`.
fn aim(invoker_id: u32) -> TimedEffectSpec {
    TimedEffectSpec {
        effect_id: 700,
        ability_id: 637,
        invoker_id,
        effect_flags: 21,
        moniker_ids: vec![],
        stats: vec![(ACCURACY, 200)],
        duration_secs: Some(15.0),
        stacking: TimedStacking::PerSource,
        invoker_identity: Default::default(),
    }
}

/// Every `(method_index, args)` sent to `PLAYER`'s own client.
fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u16, Vec<u8>)> {
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

fn timers(sent: &[(u16, Vec<u8>)]) -> Vec<&Vec<u8>> {
    sent.iter()
        .filter(|(m, _)| *m == ON_TIMER_UPDATE)
        .map(|(_, a)| a)
        .collect()
}

/// Past the game clock's epoch, so an absolute expiry is distinguishable
/// from a relative one.
fn settle_clock() {
    init();
    while game_time_secs() < 0.01 {
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn coordination(mgr: &SpaceManager) -> (i32, i32) {
    let s = mgr
        .get_entity(PLAYER)
        .unwrap()
        .stats
        .get(COORDINATION)
        .unwrap();
    (s.cur, s.max)
}

#[tokio::test]
async fn a_new_buff_gets_one_start_timer_with_an_absolute_expiry() {
    settle_clock();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(64);
    let now = Instant::now();
    mgr.apply_timed_effect(PLAYER, stim(COORDINATION, 5, 3950, 2), now);

    let before = game_time_secs();
    stat_buff_tick_at(now, &tx, &mut mgr).await;
    let after = game_time_secs();
    let sent = drain(&mut rx);
    let t = timers(&sent);
    assert_eq!(t.len(), 1, "one start timer: {sent:?}");
    // id 3950, TIMER_DURATION_EFFECT (5), source 1, secondary 3950,
    // TotalTime 3600.0.
    assert_eq!(
        &t[0][..17],
        &[
            0x6E, 0x0F, 0x00, 0x00, // id = 3950
            0x05, // TIMER_DURATION_EFFECT
            0x01, 0x00, 0x00, 0x00, // source = the user
            0x6E, 0x0F, 0x00, 0x00, // secondary = 3950
            0x00, 0x00, 0x61, 0x45, // 3600.0f32
        ][..]
    );
    let expiry = f32::from_le_bytes(t[0][17..21].try_into().unwrap());
    assert!(
        (before + 3600.0..=after + 3600.0).contains(&expiry),
        "BigWorldTimeComplete {expiry} is not game time [{before}, {after}] + 3600"
    );

    stat_buff_tick_at(now + Duration::from_secs(1), &tx, &mut mgr).await;
    assert!(
        timers(&drain(&mut rx)).is_empty(),
        "the start timer is sent once"
    );
}

#[tokio::test]
async fn expiry_restores_the_stat_and_clears_the_timer() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(64);
    let now = Instant::now();
    mgr.apply_timed_effect(PLAYER, stim(COORDINATION, 5, 3950, 2), now);
    stat_buff_tick_at(now, &tx, &mut mgr).await;
    let _ = drain(&mut rx);
    assert_eq!(coordination(&mgr), (15, 15));

    let still = stat_buff_tick_at(now + Duration::from_secs(3599), &tx, &mut mgr).await;
    assert_eq!(still, 0);
    assert_eq!(coordination(&mgr), (15, 15), "not before the hour");

    let expired = stat_buff_tick_at(now + Duration::from_secs(3600), &tx, &mut mgr).await;
    assert_eq!(expired, 1);
    assert_eq!(coordination(&mgr), (10, 10), "back to the archetype value");
    let sent = drain(&mut rx);
    assert_eq!(
        timers(&sent),
        vec![&vec![
            0x6E, 0x0F, 0x00, 0x00, // id = 3950
            0x05, // TIMER_DURATION_EFFECT
            0x01, 0x00, 0x00, 0x00, // source
            0x6E, 0x0F, 0x00, 0x00, // secondary
            0x00, 0x00, 0x00, 0x00, // TotalTime 0.0
            0x00, 0x00, 0x00, 0x00, // BigWorldTimeComplete 0.0: clear
        ]],
    );
    assert!(
        sent.iter()
            .any(|(m, _)| *m == crate::mercury::method_idx::ON_STAT_UPDATE),
        "the restored stat reaches the client"
    );
    assert!(mgr.get_entity(PLAYER).unwrap().stat_buffs.is_idle());
    stat_buff_tick_at(now + Duration::from_secs(7200), &tx, &mut mgr).await;
    assert!(drain(&mut rx).is_empty(), "an idle ledger sends nothing");
}

#[tokio::test]
async fn a_replacement_clears_the_old_icon_and_starts_the_new_one() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(64);
    let now = Instant::now();
    mgr.apply_timed_effect(PLAYER, stim(COORDINATION, 5, 3950, 2), now);
    stat_buff_tick_at(now, &tx, &mut mgr).await;
    let _ = drain(&mut rx);

    mgr.apply_timed_effect(PLAYER, stim(COORDINATION, 7, 3956, 2), now);
    stat_buff_tick_at(now, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    let t = timers(&sent);
    assert_eq!(t.len(), 2, "{sent:?}");
    let id = |args: &Vec<u8>| i32::from_le_bytes(args[..4].try_into().unwrap());
    let expiry = |args: &Vec<u8>| f32::from_le_bytes(args[17..21].try_into().unwrap());
    assert_eq!(
        (id(t[0]), expiry(t[0])),
        (3950, 0.0),
        "clear the Mark III icon"
    );
    assert_eq!(id(t[1]), 3956, "then start the Mark V icon");
    assert_eq!(coordination(&mgr), (17, 17));
}

#[tokio::test]
async fn two_stats_of_one_effect_share_one_timer() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(64);
    let now = Instant::now();
    let mut spec = stim(COORDINATION, 5, 7000, 2);
    spec.stats.push((ENGAGEMENT, 5));
    mgr.apply_timed_effect(PLAYER, spec, now);
    stat_buff_tick_at(now, &tx, &mut mgr).await;
    assert_eq!(timers(&drain(&mut rx)).len(), 1);
}

#[tokio::test]
async fn death_takes_off_only_clear_on_death_buffs() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(64);
    let now = Instant::now();
    // A stimpack row (flags 2, EF_Offline_Time_Counts) and a hypothetical
    // EF_ClearOnDeath buff on another stat.
    mgr.apply_timed_effect(PLAYER, stim(COORDINATION, 5, 3950, 2), now);
    mgr.apply_timed_effect(PLAYER, stim(ENGAGEMENT, 3, 9001, EF_CLEAR_ON_DEATH), now);
    stat_buff_tick_at(now, &tx, &mut mgr).await;
    let _ = drain(&mut rx);

    let removed = clear_stat_buffs_on_death(PLAYER, &tx, &mut mgr).await;
    assert_eq!(removed, 1);
    let e = mgr.get_entity(PLAYER).unwrap();
    assert_eq!(
        e.stats.get(ENGAGEMENT).unwrap().cur,
        12,
        "the death buff came off"
    );
    assert_eq!(
        e.stats.get(COORDINATION).unwrap().cur,
        15,
        "the stim outlasts death"
    );
    let sent = drain(&mut rx);
    let t = timers(&sent);
    assert_eq!(t.len(), 1);
    assert_eq!(i32::from_le_bytes(t[0][..4].try_into().unwrap()), 9001);
}

/// The AB-04 wire: an ability buff's start timer names the effect as both
/// the timer id and the SecondaryId, and the caster as the source; its
/// expiry clears with `0.0, 0.0`.
#[tokio::test]
async fn an_ability_buff_starts_and_clears_its_timer_byte_for_byte() {
    settle_clock();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(64);
    let now = Instant::now();
    mgr.apply_timed_effect(PLAYER, aim(PLAYER), now);
    let before = game_time_secs();
    flush_stat_buff_timers(PLAYER, now, &tx, &mut mgr).await;
    let after = game_time_secs();
    let sent = drain(&mut rx);
    let t = timers(&sent);
    assert_eq!(t.len(), 1, "{sent:?}");
    assert_eq!(
        &t[0][..17],
        &[
            0xBC, 0x02, 0x00, 0x00, // id = 700
            0x05, // TIMER_DURATION_EFFECT
            0x01, 0x00, 0x00, 0x00, // source = the caster
            0xBC, 0x02, 0x00, 0x00, // SecondaryId = 700
            0x00, 0x00, 0x70, 0x41, // TotalTime 15.0f32
        ][..]
    );
    let expiry = f32::from_le_bytes(t[0][17..21].try_into().unwrap());
    assert!((before + 15.0..=after + 15.0).contains(&expiry), "{expiry}");

    let expired = stat_buff_tick_at(now + Duration::from_secs(15), &tx, &mut mgr).await;
    assert_eq!(expired, 1);
    let accuracy = mgr
        .get_entity(PLAYER)
        .unwrap()
        .stats
        .get(ACCURACY)
        .unwrap()
        .cur;
    assert_eq!(accuracy, 0, "expiry reverts exactly the +200");
    assert_eq!(
        timers(&drain(&mut rx)),
        vec![&vec![
            0xBC, 0x02, 0x00, 0x00, // id = 700
            0x05, // TIMER_DURATION_EFFECT
            0x01, 0x00, 0x00, 0x00, // source
            0xBC, 0x02, 0x00, 0x00, // SecondaryId = 700
            0x00, 0x00, 0x00, 0x00, // TotalTime 0.0
            0x00, 0x00, 0x00, 0x00, // BigWorldTimeComplete 0.0: clear
        ]],
    );
}

/// **Regression guard.** The client keys an
/// effect icon by SecondaryId alone, so two casters' Aims are one icon: one
/// start carrying the later expiry (source 42), no clear when the first
/// lapses, and the clear only when the last one does. On revert (one timer
/// per entry) the first expiry cleared the icon while 42's Aim was live.
#[tokio::test]
async fn two_casters_share_one_icon_cleared_by_the_last() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(64);
    let now = Instant::now();
    mgr.apply_timed_effect(PLAYER, aim(PLAYER), now);
    mgr.apply_timed_effect(PLAYER, aim(42), now + Duration::from_secs(5));
    flush_stat_buff_timers(PLAYER, now, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    let t = timers(&sent);
    assert_eq!(t.len(), 1, "one icon for effect 700: {sent:?}");
    assert_eq!(i32::from_le_bytes(t[0][5..9].try_into().unwrap()), 42);
    assert_eq!(f32::from_le_bytes(t[0][13..17].try_into().unwrap()), 15.0);

    stat_buff_tick_at(now + Duration::from_secs(15), &tx, &mut mgr).await;
    let t: Vec<_> = timers(&drain(&mut rx)).into_iter().cloned().collect();
    assert!(
        t.iter()
            .all(|a| f32::from_le_bytes(a[17..21].try_into().unwrap()) > 0.0),
        "no clear while 42's Aim is live: {t:?}"
    );
    let accuracy = mgr
        .get_entity(PLAYER)
        .unwrap()
        .stats
        .get(ACCURACY)
        .unwrap()
        .cur;
    assert_eq!(accuracy, 200, "the second caster's Aim is still up");

    stat_buff_tick_at(now + Duration::from_secs(20), &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    let t = timers(&sent);
    assert_eq!(t.len(), 1);
    assert_eq!(
        f32::from_le_bytes(t[0][17..21].try_into().unwrap()),
        0.0,
        "the clear"
    );
}

/// A held entry (AB-08's toggles) never expires on the tick and sends no
/// start timer, but its strip clears.
#[tokio::test]
async fn a_held_entry_outlasts_the_tick_until_stripped() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(64);
    let now = Instant::now();
    let mut spec = aim(PLAYER);
    spec.duration_secs = None;
    mgr.apply_timed_effect(PLAYER, spec, now);
    let expired = stat_buff_tick_at(now + Duration::from_secs(86_400), &tx, &mut mgr).await;
    assert_eq!(expired, 0);
    assert!(
        timers(&drain(&mut rx)).is_empty(),
        "no start for a held entry"
    );

    let removed = strip_timed_effects(
        PLAYER,
        StatBuffRemoval::ToggledOff,
        |e| e.effect_id == 700,
        &tx,
        &mut mgr,
    )
    .await;
    assert_eq!(removed, 1);
    let sent = drain(&mut rx);
    assert_eq!(timers(&sent).len(), 1, "the clear");
    assert!(sent
        .iter()
        .any(|(m, _)| *m == crate::mercury::method_idx::ON_STAT_UPDATE));
}

/// Aim carries `EF_ClearOnDeath` (flags 21): a death takes it off.
#[tokio::test]
async fn death_takes_off_aim() {
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    mgr.apply_timed_effect(PLAYER, aim(PLAYER), Instant::now());
    assert_eq!(clear_stat_buffs_on_death(PLAYER, &tx, &mut mgr).await, 1);
    let accuracy = mgr
        .get_entity(PLAYER)
        .unwrap()
        .stats
        .get(ACCURACY)
        .unwrap()
        .cur;
    assert_eq!(accuracy, 0);
}
