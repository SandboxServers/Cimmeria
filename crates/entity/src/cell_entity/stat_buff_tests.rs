//! The timed effect ledger's arithmetic: per-source refresh and stacking,
//! the stimpacks' stat-keyed replace rule, the bound widening and its exact
//! reversal.

use std::time::{Duration, Instant};

use cimmeria_common::{EntityId, SpaceId, Vector3};

use super::stat_buff::{
    shift_stat_widening, unshift_stat, StatShift, TimedEffectSpec, TimedStacking,
};
use super::CellEntity;
use crate::stats::{
    ArchetypeStatValues, ACCURACY, COORDINATION, DEFENSE, ENGAGEMENT, HEALTH, RESPONSE,
};

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

/// A stimpack effect: one stat, an hour, the stat-keyed rule.
fn stim(stat_id: i32, delta: i32, effect_id: i32) -> TimedEffectSpec {
    TimedEffectSpec {
        effect_id,
        ability_id: effect_id + 10_000,
        invoker_id: 1,
        effect_flags: 2,
        moniker_ids: vec![],
        stats: vec![(stat_id, delta)],
        absorb: Vec::new(),
        duration_secs: Some(3600.0),
        stacking: TimedStacking::ReplaceSameStat,
        invoker_identity: Default::default(),
    }
}

/// An ability buff (Aim, effect 700): per source, 15 s.
fn aim(invoker_id: u32) -> TimedEffectSpec {
    TimedEffectSpec {
        effect_id: 700,
        ability_id: 637,
        invoker_id,
        effect_flags: 21,
        moniker_ids: vec![3_212_632_871],
        stats: vec![(ACCURACY, 200)],
        absorb: Vec::new(),
        duration_secs: Some(15.0),
        stacking: TimedStacking::PerSource,
        invoker_identity: Default::default(),
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
    let out = e
        .apply_timed_effect(stim(COORDINATION, 5, 3950), now)
        .unwrap();
    assert_eq!(
        cur_max(&e, COORDINATION),
        (15, 15),
        "+5 must not clamp at max"
    );
    assert_eq!(
        out.applied.stats[0].shift,
        StatShift {
            cur: 5,
            min: 0,
            max: 5
        }
    );
    assert_eq!(
        out.applied.expires_at,
        Some(now + Duration::from_secs(3600))
    );
    assert!(!out.applied.timer_sent);
    assert!(out.replaced.is_empty());
    assert!(
        e.stats.get(COORDINATION).unwrap().dirty,
        "the client must hear of it"
    );
}

#[test]
fn removing_an_entry_restores_cur_and_max_exactly() {
    let mut e = entity();
    e.apply_timed_effect(stim(COORDINATION, 5, 3950), Instant::now());
    let removed = e.remove_timed_effects_where(|b| b.effect_id == 3950);
    assert_eq!(removed.len(), 1);
    assert_eq!(cur_max(&e, COORDINATION), (10, 10), "no permanent stat");
    assert!(e.stat_buffs.entries.is_empty());
    assert_eq!(e.stat_buffs.pending_timer_clears, vec![(3950, 1)]);
}

#[test]
fn a_higher_stim_tier_on_the_same_stat_replaces_rather_than_stacks() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_timed_effect(stim(COORDINATION, 5, 3950), now);
    let out = e
        .apply_timed_effect(stim(COORDINATION, 7, 3956), now)
        .unwrap();
    assert_eq!(
        cur_max(&e, COORDINATION),
        (17, 17),
        "+7 replaces +5, not +12"
    );
    assert_eq!(
        out.replaced.iter().map(|b| b.effect_id).collect::<Vec<_>>(),
        vec![3950]
    );
    assert_eq!(e.stat_buffs.entries.len(), 1, "one stim per stat");
    assert_eq!(
        e.stat_buffs.pending_timer_clears,
        vec![(3950, 1)],
        "the replaced effect's icon must be cleared"
    );
    e.remove_timed_effects_where(|_| true);
    assert_eq!(cur_max(&e, COORDINATION), (10, 10));
}

#[test]
fn a_lower_stim_tier_on_the_same_stat_also_replaces() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_timed_effect(stim(COORDINATION, 10, 3979), now);
    e.apply_timed_effect(stim(COORDINATION, 5, 3950), now);
    assert_eq!(
        cur_max(&e, COORDINATION),
        (15, 15),
        "the last stim applied wins"
    );
}

#[test]
fn stims_on_different_stats_do_not_collide() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_timed_effect(stim(COORDINATION, 5, 3950), now);
    let out = e
        .apply_timed_effect(stim(ENGAGEMENT, 3, 3955), now)
        .unwrap();
    assert!(out.replaced.is_empty());
    assert_eq!(cur_max(&e, COORDINATION), (15, 15));
    assert_eq!(cur_max(&e, ENGAGEMENT), (15, 15));
    assert_eq!(e.stat_buffs.entries.len(), 2);
    assert!(e.stat_buffs.pending_timer_clears.is_empty());
}

#[test]
fn reapplying_the_same_effect_refreshes_and_owes_no_clear() {
    let mut e = entity();
    let t0 = Instant::now();
    e.apply_timed_effect(stim(COORDINATION, 5, 3950), t0);
    let t1 = t0 + Duration::from_secs(600);
    let out = e
        .apply_timed_effect(stim(COORDINATION, 5, 3950), t1)
        .unwrap();
    assert_eq!(cur_max(&e, COORDINATION), (15, 15));
    assert_eq!(out.applied.expires_at, Some(t1 + Duration::from_secs(3600)));
    assert!(
        e.stat_buffs.pending_timer_clears.is_empty(),
        "the new start timer supersedes the old one"
    );
}

/// The contract's refresh rule: the same effect from the same invoker
/// refreshes, it never stacks. On a revert to stat-blind stacking Accuracy
/// would read 400.
#[test]
fn the_same_source_refreshes_an_ability_buff() {
    let mut e = entity();
    let t0 = Instant::now();
    e.apply_timed_effect(aim(1), t0);
    let t1 = t0 + Duration::from_secs(10);
    let out = e.apply_timed_effect(aim(1), t1).unwrap();
    assert_eq!(
        e.stats.get(ACCURACY).unwrap().cur,
        200,
        "refreshed, not +400"
    );
    assert_eq!(out.replaced.len(), 1);
    assert_eq!(out.applied.expires_at, Some(t1 + Duration::from_secs(15)));
    assert_eq!(e.stat_buffs.entries.len(), 1);
    assert!(e.stat_buffs.pending_timer_clears.is_empty());
}

/// Different invokers stack, and each comes off on its own. On a revert to
/// the stat-keyed rule the second Aim would replace the first (200, not 400).
#[test]
fn different_sources_stack_and_expire_independently() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_timed_effect(aim(1), now);
    let out = e.apply_timed_effect(aim(2), now).unwrap();
    assert!(out.replaced.is_empty());
    assert_eq!(e.stats.get(ACCURACY).unwrap().cur, 400);
    e.stat_buffs
        .entries
        .iter_mut()
        .for_each(|b| b.timer_sent = true);
    e.remove_timed_effects_where(|b| b.invoker_id == 1);
    assert_eq!(e.stats.get(ACCURACY).unwrap().cur, 200);
    // One icon per effect: the shared icon stays, and is re-sent with the
    // remaining entry's expiry instead of being cleared.
    assert!(e.stat_buffs.pending_timer_clears.is_empty());
    assert!(!e.stat_buffs.entries[0].timer_sent);
    e.remove_timed_effects_where(|_| true);
    assert_eq!(e.stat_buffs.pending_timer_clears, vec![(700, 2)]);
}

/// Expiry reverts exactly the applied delta, even when something else moved
/// the stat meanwhile.
#[test]
fn removal_reverts_exactly_what_was_applied() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_timed_effect(aim(1), now);
    // Another system moves Accuracy by +50 while Aim is up.
    e.stats.get_mut(ACCURACY).unwrap().change(50);
    e.remove_timed_effects_where(|b| b.effect_id == 700);
    assert_eq!(
        e.stats.get(ACCURACY).unwrap().cur,
        50,
        "only Aim's +200 comes off"
    );
}

#[test]
fn one_entry_moves_several_stats_and_restores_them_together() {
    let mut e = entity();
    let spec = TimedEffectSpec {
        effect_id: 1980,
        ability_id: 1630,
        invoker_id: 9,
        effect_flags: 6,
        moniker_ids: vec![],
        stats: vec![(ACCURACY, -200), (DEFENSE, -200)],
        absorb: Vec::new(),
        duration_secs: Some(15.0),
        stacking: TimedStacking::PerSource,
        invoker_identity: Default::default(),
    };
    e.apply_timed_effect(spec, Instant::now()).unwrap();
    assert_eq!(e.stats.get(ACCURACY).unwrap().cur, -200);
    let d = e.stats.get(DEFENSE).unwrap();
    assert_eq!(
        (d.min, d.cur, d.max),
        (-200, -200, 0),
        "min widens for a debuff"
    );
    e.remove_timed_effects_where(|_| true);
    let d = e.stats.get(DEFENSE).unwrap();
    assert_eq!((d.min, d.cur, d.max), (0, 0, 0));
    assert_eq!(e.stats.get(ACCURACY).unwrap().cur, 0);
    assert_eq!(e.stat_buffs.pending_timer_clears, vec![(1980, 9)]);
}

#[test]
fn a_held_entry_has_no_expiry() {
    let mut e = entity();
    let mut spec = aim(1);
    spec.duration_secs = None;
    let now = Instant::now();
    let out = e.apply_timed_effect(spec, now).unwrap();
    assert_eq!(out.applied.expires_at, None);
    assert!(!out.applied.is_expired(now + Duration::from_secs(86_400)));
}

#[test]
fn an_icon_only_entry_applies_with_no_stats() {
    let mut e = entity();
    let mut spec = aim(1);
    spec.stats.clear();
    let out = e.apply_timed_effect(spec, Instant::now()).unwrap();
    assert!(out.applied.stats.is_empty());
    assert_eq!(e.stat_buffs.entries.len(), 1);
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
        .apply_timed_effect(stim(9_999, 5, 1), Instant::now())
        .is_none());
    assert!(e.stat_buffs.is_idle());
}

#[test]
fn a_partly_missing_spec_applies_the_stats_it_can_and_reports_the_rest() {
    let mut e = entity();
    let mut spec = aim(1);
    spec.stats.push((9_999, 5));
    let out = e.apply_timed_effect(spec, Instant::now()).unwrap();
    assert_eq!(out.missing_stats, vec![9_999]);
    assert_eq!(e.stats.get(ACCURACY).unwrap().cur, 200);
}

#[test]
fn the_effect_bar_counts_each_side() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_timed_effect(aim(1), now);
    e.apply_timed_effect(aim(2), now);
    let mut brace = aim(1);
    brace.effect_id = 907;
    e.apply_timed_effect(brace, now);
    let mut debuff = aim(3);
    debuff.effect_id = 903;
    debuff.effect_flags = 20;
    e.apply_timed_effect(debuff, now);
    assert_eq!(e.stat_buffs.bar_icons(true), 2, "two Aims share one icon");
    assert_eq!(e.stat_buffs.bar_icons(false), 1);
}

/// A buff and a debuff of opposite sign on one 0/0/0 stat (Heroism +50 and
/// a -100 Response debuff). Order-dependent reverts clamped the second
/// removal against the first one's restored bound.
fn heroism_and_debuff() -> (CellEntity, TimedEffectSpec, TimedEffectSpec) {
    let mut e = entity();
    e.stats.get_mut(RESPONSE).unwrap().update(0, 0, 0);
    let mut heroism = aim(1);
    heroism.effect_id = 1744;
    heroism.stats = vec![(RESPONSE, 50)];
    let mut debuff = aim(2);
    debuff.effect_id = 4309;
    debuff.effect_flags = 68;
    debuff.stats = vec![(RESPONSE, -100)];
    (e, heroism, debuff)
}

fn response(e: &CellEntity) -> (i32, i32, i32) {
    let s = e.stats.get(RESPONSE).unwrap();
    (s.min, s.cur, s.max)
}

/// **Regression guard.** The buff lapses first: Response must read -100
/// while the debuff is up, and 0/0/0 after. On revert to per-entry bound
/// shifts it read -50 (the debuff clamped at its own widened min).
#[test]
fn opposite_signs_revert_when_the_buff_lapses_first() {
    let (mut e, heroism, debuff) = heroism_and_debuff();
    let now = Instant::now();
    e.apply_timed_effect(heroism, now).unwrap();
    e.apply_timed_effect(debuff, now).unwrap();
    assert_eq!(response(&e).1, -50);
    e.remove_timed_effects_where(|b| b.effect_id == 1744);
    assert_eq!(response(&e), (-100, -100, 0), "only the debuff is left");
    e.remove_timed_effects_where(|b| b.effect_id == 4309);
    assert_eq!(response(&e), (0, 0, 0));
    assert!(e.stat_buffs.baselines.is_empty());
}

/// **Regression guard.** The debuff lapses first: Response reads +50 while
/// the buff is up, and 0/0/0 after.
#[test]
fn opposite_signs_revert_when_the_debuff_lapses_first() {
    let (mut e, heroism, debuff) = heroism_and_debuff();
    let now = Instant::now();
    e.apply_timed_effect(heroism, now).unwrap();
    e.apply_timed_effect(debuff, now).unwrap();
    e.remove_timed_effects_where(|b| b.effect_id == 4309);
    assert_eq!(response(&e), (0, 50, 50), "only the buff is left");
    e.remove_timed_effects_where(|b| b.effect_id == 1744);
    assert_eq!(response(&e), (0, 0, 0));
}
