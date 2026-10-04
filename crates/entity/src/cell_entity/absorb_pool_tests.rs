//! Absorb pools on the timed effect ledger: grant, drain, settle, and the
//! release every other removal does.

use std::time::{Duration, Instant};

use cimmeria_common::{EntityId, SpaceId, Vector3};

use super::stat_buff::{TimedEffectSpec, TimedStacking};
use super::CellEntity;
use crate::stats::{ABSORB_ENERGY, ABSORB_PHYSICAL};

fn entity() -> CellEntity {
    CellEntity::new(EntityId(1), SpaceId(1), Vector3::zero())
}

/// Personal Shield's shape: 30 s, one pool per listed type.
fn shield(effect_id: i32, invoker_id: u32, pools: Vec<(i32, i32)>) -> TimedEffectSpec {
    TimedEffectSpec {
        effect_id,
        ability_id: 1013,
        invoker_id,
        effect_flags: 4,
        moniker_ids: vec![],
        stats: vec![],
        absorb: pools,
        duration_secs: Some(30.0),
        stacking: TimedStacking::PerSource,
        invoker_identity: Default::default(),
    }
}

fn absorb(e: &CellEntity, stat: i32) -> i32 {
    e.stats.get(stat).unwrap().cur
}

#[test]
fn a_shield_fills_its_absorb_stats_and_holds_its_pools() {
    let mut e = entity();
    let now = Instant::now();
    let out = e
        .apply_timed_effect(
            shield(4306, 1, vec![(ABSORB_PHYSICAL, 500), (ABSORB_ENERGY, 500)]),
            now,
        )
        .expect("a shield with no stats still applies");
    assert_eq!(absorb(&e, ABSORB_PHYSICAL), 500);
    assert_eq!(absorb(&e, ABSORB_ENERGY), 500);
    assert_eq!(out.applied.absorb.len(), 2);
    assert_eq!(out.applied.absorb_remaining(), 1000);
}

/// The drain-then-expire shape: damage takes 300 of a 500 pool, the
/// entry's expiry takes the other 200 back, and the stat ends at 0. Before
/// AB-10 nothing took a timed shield's capacity back off (B-33).
#[test]
fn a_shield_drains_then_expires_to_zero() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_timed_effect(shield(4306, 1, vec![(ABSORB_PHYSICAL, 500)]), now);
    // The damage pipeline drains the stat, as `calculate_damage` does.
    e.stats.get_mut(ABSORB_PHYSICAL).unwrap().change(-300);
    let settled = e.settle_absorb_pools();
    assert_eq!(settled.charged, vec![(4306, 1, ABSORB_PHYSICAL, 300)]);
    assert!(settled.drained.is_empty(), "200 left: the shield stays up");
    assert_eq!(e.stat_buffs.entries[0].absorb[0].remaining, 200);

    let later = now + Duration::from_secs(31);
    let gone = e.remove_timed_effects_where(|b| b.is_expired(later));
    assert_eq!(gone.len(), 1);
    assert_eq!(
        absorb(&e, ABSORB_PHYSICAL),
        0,
        "expiry takes the unspent 200 back off the stat"
    );
    assert_eq!(e.stat_buffs.pending_timer_clears, vec![(4306, 1)]);
}

#[test]
fn a_shield_drained_to_zero_comes_off_with_its_icon() {
    let mut e = entity();
    e.apply_timed_effect(
        shield(4306, 1, vec![(ABSORB_PHYSICAL, 500)]),
        Instant::now(),
    );
    e.stats.get_mut(ABSORB_PHYSICAL).unwrap().change(-500);
    let settled = e.settle_absorb_pools();
    assert_eq!(settled.drained.len(), 1, "an empty shield comes off");
    assert!(e.stat_buffs.entries.is_empty());
    assert_eq!(e.stat_buffs.pending_timer_clears, vec![(4306, 1)]);
    assert_eq!(absorb(&e, ABSORB_PHYSICAL), 0);
}

/// A multi-type shield stays up while any of its pools has capacity.
#[test]
fn a_shield_with_one_pool_left_stays_up() {
    let mut e = entity();
    e.apply_timed_effect(
        shield(4306, 1, vec![(ABSORB_PHYSICAL, 500), (ABSORB_ENERGY, 500)]),
        Instant::now(),
    );
    e.stats.get_mut(ABSORB_PHYSICAL).unwrap().change(-500);
    assert!(e.settle_absorb_pools().drained.is_empty());
    assert_eq!(e.stat_buffs.entries[0].absorb_remaining(), 500);
}

/// Two casters' shields on one stat: the older pays first.
#[test]
fn settling_charges_the_oldest_shield_first() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_timed_effect(shield(4306, 1, vec![(ABSORB_PHYSICAL, 100)]), now);
    e.apply_timed_effect(shield(4306, 2, vec![(ABSORB_PHYSICAL, 100)]), now);
    e.stats.get_mut(ABSORB_PHYSICAL).unwrap().change(-150);
    let settled = e.settle_absorb_pools();
    assert_eq!(
        settled.charged,
        vec![
            (4306, 1, ABSORB_PHYSICAL, 100),
            (4306, 2, ABSORB_PHYSICAL, 50)
        ]
    );
    assert_eq!(settled.drained.len(), 1);
    assert_eq!(e.stat_buffs.entries[0].invoker_id, 2);
    assert_eq!(e.stat_buffs.entries[0].absorb[0].remaining, 50);
    // The second shield did not lose its icon: another entry of the effect
    // is live, so no clear is owed.
    assert!(e.stat_buffs.pending_timer_clears.is_empty());
}

/// Capacity on the stat that no pool owns (an item) is spent first.
#[test]
fn unowned_capacity_is_spent_before_a_pool() {
    let mut e = entity();
    e.stats.get_mut(ABSORB_PHYSICAL).unwrap().change(100);
    e.apply_timed_effect(
        shield(4306, 1, vec![(ABSORB_PHYSICAL, 200)]),
        Instant::now(),
    );
    e.stats.get_mut(ABSORB_PHYSICAL).unwrap().change(-80);
    let settled = e.settle_absorb_pools();
    assert!(settled.charged.is_empty(), "the item's 100 paid for the 80");
    assert_eq!(e.stat_buffs.entries[0].absorb[0].remaining, 200);
}

/// A refresh by the same caster takes the old pool's remainder off before
/// the new one goes on, so pools never pile up on a re-cast.
#[test]
fn a_refresh_replaces_the_pool_instead_of_stacking_it() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_timed_effect(shield(4306, 1, vec![(ABSORB_PHYSICAL, 500)]), now);
    e.stats.get_mut(ABSORB_PHYSICAL).unwrap().change(-200);
    e.settle_absorb_pools();
    e.apply_timed_effect(shield(4306, 1, vec![(ABSORB_PHYSICAL, 500)]), now);
    assert_eq!(absorb(&e, ABSORB_PHYSICAL), 500);
    assert_eq!(e.stat_buffs.entries.len(), 1);
}

/// The stat's max (1000) caps a pool; the pool holds only what went on.
#[test]
fn a_pool_holds_only_what_the_stat_took() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_timed_effect(shield(4306, 1, vec![(ABSORB_PHYSICAL, 800)]), now);
    let out = e
        .apply_timed_effect(shield(4306, 2, vec![(ABSORB_PHYSICAL, 800)]), now)
        .unwrap();
    assert_eq!(out.applied.absorb[0].granted, 200);
    e.remove_timed_effects_where(|_| true);
    assert_eq!(absorb(&e, ABSORB_PHYSICAL), 0);
}

/// **Regression guard: a shield with no room is refused.** The stat is at
/// its max (1000), so a second caster's shield would put up a live icon that
/// absorbs nothing: it is refused and nothing changes. The first caster's
/// refresh still goes on, because it frees its own pool first.
#[test]
fn a_shield_with_no_room_is_refused() {
    let mut e = entity();
    let now = Instant::now();
    e.apply_timed_effect(shield(4306, 1, vec![(ABSORB_PHYSICAL, 1000)]), now)
        .unwrap();
    assert_eq!(e.absorb_room(&[(ABSORB_PHYSICAL, 500)], (4306, 2)), 0);
    assert!(e
        .apply_timed_effect(shield(4306, 2, vec![(ABSORB_PHYSICAL, 500)]), now)
        .is_none());
    assert_eq!(e.stat_buffs.entries.len(), 1, "no empty second entry");
    assert!(e
        .apply_timed_effect(shield(4306, 1, vec![(ABSORB_PHYSICAL, 1000)]), now)
        .is_some());
    assert_eq!(absorb(&e, ABSORB_PHYSICAL), 1000);
}
