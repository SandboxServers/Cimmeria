//! The stat-buff tick: the byte-exact duration timers, expiry restoring the
//! stat, the replacement's clear, and the death strip.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_entity::abilities::EF_CLEAR_ON_DEATH;
use cimmeria_entity::cell_entity::StatBuffSpec;
use cimmeria_entity::stats::{COORDINATION, ENGAGEMENT};

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

fn stim(stat_id: i32, delta: i32, effect_id: i32, flags: u32) -> StatBuffSpec {
    StatBuffSpec {
        stat_id,
        delta,
        effect_id,
        ability_id: 2735,
        invoker_id: PLAYER,
        effect_flags: flags,
        duration_secs: 3600.0,
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
    mgr.apply_stat_buff(PLAYER, stim(COORDINATION, 5, 3950, 2), now);

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
    mgr.apply_stat_buff(PLAYER, stim(COORDINATION, 5, 3950, 2), now);
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
    mgr.apply_stat_buff(PLAYER, stim(COORDINATION, 5, 3950, 2), now);
    stat_buff_tick_at(now, &tx, &mut mgr).await;
    let _ = drain(&mut rx);

    mgr.apply_stat_buff(PLAYER, stim(COORDINATION, 7, 3956, 2), now);
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
    mgr.apply_stat_buff(PLAYER, stim(COORDINATION, 5, 7000, 2), now);
    mgr.apply_stat_buff(PLAYER, stim(ENGAGEMENT, 5, 7000, 2), now);
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
    mgr.apply_stat_buff(PLAYER, stim(COORDINATION, 5, 3950, 2), now);
    mgr.apply_stat_buff(PLAYER, stim(ENGAGEMENT, 3, 9001, EF_CLEAR_ON_DEATH), now);
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
