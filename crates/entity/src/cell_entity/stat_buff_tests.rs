//! The stat-buff ledger's arithmetic: the stat-keyed replace rule, the
//! bound widening and its exact reversal.

use std::time::{Duration, Instant};

use cimmeria_common::{EntityId, SpaceId, Vector3};

use super::stat_buff::{shift_stat_widening, unshift_stat, StatBuffSpec, StatShift};
use super::CellEntity;
use crate::stats::{ArchetypeStatValues, COORDINATION, ENGAGEMENT, HEALTH};

/// A player-like entity with the archetype's primary attributes, which sit
/// at `cur == max` (Coordination 10/10).
fn entity() -> CellEntity {
    let mut e = CellEntity::new(EntityId(1), SpaceId(1), Vector3::zero());
    e.stats.apply_archetype(&ArchetypeStatValues {
        coordination: 10,
        engagement: 12,
        fortitude: 10,
        morale: 10,
        perception: 10,
        intelligence: 10,
        health: 500,
        focus: 300,
        health_per_level: 0,
        focus_per_level: 0,
    });
    e
}

fn spec(stat_id: i32, delta: i32, effect_id: i32) -> StatBuffSpec {
    StatBuffSpec {
        stat_id,
        delta,
        effect_id,
        ability_id: effect_id + 10_000,
        invoker_id: 1,
        effect_flags: 2,
        duration_secs: 3600.0,
    }
}

fn cur_max(e: &CellEntity, stat: i32) -> (i32, i32) {
    let s = e.stats.get(stat).unwrap();
    (s.cur, s.max)
}

#[test]
fn a_buff_on_a_stat_at_max_raises_cur_and_max_together() {
    let mut e = entity();
    let now = Instant::now();
    let out = e.apply_stat_buff(spec(COORDINATION, 5, 3950), now).unwrap();
    assert_eq!(
        cur_max(&e, COORDINATION),
        (15, 15),
        "+5 must not clamp at max"
    );
    assert_eq!(
        out.applied.shift,
        StatShift {
            cur: 5,
            min: 0,
            max: 5
        }
    );
    assert_eq!(out.applied.expires_at, now + Duration::from_secs(3600));
    assert!(!out.applied.timer_sent);
    assert!(out.replaced.is_none());
    assert!(
        e.stats.get(COORDINATION).unwrap().dirty,
        "the client must hear of it"
    );
}

#[test]
fn removing_a_buff_restores_cur_and_max_exactly() {
    let mut e = entity();
    e.apply_stat_buff(spec(COORDINATION, 5, 3950), Instant::now());
    let removed = e.remove_stat_buffs_where(|b| b.effect_id == 3950);
    assert_eq!(removed.len(), 1);
    assert_eq!(cur_max(&e, COORDINATION), (10, 10), "no permanent stat");
    assert!(e.stat_buffs.buffs.is_empty());
    assert_eq!(e.stat_buffs.pending_timer_clears, vec![(3950, 1)]);
}

#[test]
fn a_higher_tier_on_the_same_stat_replaces_rather_than_stacks() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_stat_buff(spec(COORDINATION, 5, 3950), now);
    let out = e.apply_stat_buff(spec(COORDINATION, 7, 3956), now).unwrap();
    assert_eq!(
        cur_max(&e, COORDINATION),
        (17, 17),
        "+7 replaces +5, not +12"
    );
    assert_eq!(out.replaced.as_ref().map(|b| b.effect_id), Some(3950));
    assert_eq!(e.stat_buffs.buffs.len(), 1, "one buff per stat");
    assert_eq!(
        e.stat_buffs.pending_timer_clears,
        vec![(3950, 1)],
        "the replaced effect's icon must be cleared"
    );
    e.remove_stat_buffs_where(|_| true);
    assert_eq!(cur_max(&e, COORDINATION), (10, 10));
}

#[test]
fn a_lower_tier_on_the_same_stat_also_replaces() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_stat_buff(spec(COORDINATION, 10, 3979), now);
    e.apply_stat_buff(spec(COORDINATION, 5, 3950), now);
    assert_eq!(
        cur_max(&e, COORDINATION),
        (15, 15),
        "the last stim applied wins"
    );
}

#[test]
fn different_stats_do_not_collide() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_stat_buff(spec(COORDINATION, 5, 3950), now);
    let out = e.apply_stat_buff(spec(ENGAGEMENT, 3, 3955), now).unwrap();
    assert!(out.replaced.is_none());
    assert_eq!(cur_max(&e, COORDINATION), (15, 15));
    assert_eq!(cur_max(&e, ENGAGEMENT), (15, 15));
    assert_eq!(e.stat_buffs.buffs.len(), 2);
    assert!(e.stat_buffs.pending_timer_clears.is_empty());
}

#[test]
fn reapplying_the_same_effect_refreshes_and_owes_no_clear() {
    let mut e = entity();
    let t0 = Instant::now();
    e.apply_stat_buff(spec(COORDINATION, 5, 3950), t0);
    let t1 = t0 + Duration::from_secs(600);
    let out = e.apply_stat_buff(spec(COORDINATION, 5, 3950), t1).unwrap();
    assert_eq!(cur_max(&e, COORDINATION), (15, 15));
    assert_eq!(out.applied.expires_at, t1 + Duration::from_secs(3600));
    assert!(
        e.stat_buffs.pending_timer_clears.is_empty(),
        "the new start timer supersedes the old one"
    );
}

#[test]
fn an_effect_timer_clears_only_after_its_last_stat_comes_off() {
    let mut e = entity();
    let now = Instant::now();
    // One effect moving two stats.
    e.apply_stat_buff(spec(COORDINATION, 5, 7000), now);
    e.apply_stat_buff(spec(ENGAGEMENT, 5, 7000), now);
    e.remove_stat_buffs_where(|b| b.stat_id == COORDINATION);
    assert!(e.stat_buffs.pending_timer_clears.is_empty());
    e.remove_stat_buffs_where(|b| b.stat_id == ENGAGEMENT);
    assert_eq!(e.stat_buffs.pending_timer_clears, vec![(7000, 1)]);
}

#[test]
fn a_debuff_widens_min_and_restores_it() {
    let mut e = entity();
    let s = e.stats.get_mut(COORDINATION).unwrap();
    s.update(0, 3, 10);
    let shift = shift_stat_widening(&mut e.stats, COORDINATION, -5).unwrap();
    let s = e.stats.get(COORDINATION).unwrap();
    assert_eq!((s.min, s.cur, s.max), (-2, -2, 10));
    unshift_stat(&mut e.stats, COORDINATION, shift).unwrap();
    let s = e.stats.get(COORDINATION).unwrap();
    assert_eq!((s.min, s.cur, s.max), (0, 3, 10));
}

#[test]
fn a_buff_below_max_moves_cur_only() {
    let mut e = entity();
    e.stats.get_mut(HEALTH).unwrap().update(0, 100, 500);
    let shift = shift_stat_widening(&mut e.stats, HEALTH, 50).unwrap();
    assert_eq!(
        shift,
        StatShift {
            cur: 50,
            min: 0,
            max: 0
        }
    );
    assert_eq!(cur_max(&e, HEALTH), (150, 500));
}

#[test]
fn a_missing_stat_changes_nothing() {
    let mut e = entity();
    assert!(e
        .apply_stat_buff(spec(9_999, 5, 1), Instant::now())
        .is_none());
    assert!(e.stat_buffs.is_idle());
}
