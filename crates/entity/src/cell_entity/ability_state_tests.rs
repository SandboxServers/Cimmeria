//! AB-T5: the ability state snapshot over a seeded warmup, cooldowns, a
//! pulse and ledger entries (a stat buff, a stun's flag hold, a shield).

use std::time::{Duration, Instant};

use cimmeria_common::{EntityId, SpaceId, Vector3};

use super::stat_buff::{TimedEffectSpec, TimedStacking};
use super::{ActiveEffectInstance, CellEntity, PendingCast};
use crate::stats::{ABSORB_PHYSICAL, ACCURACY, HEALTH};

const MOVEMENT_LOCK: u32 = 1 << 6;

fn entity() -> CellEntity {
    let mut e = CellEntity::new(EntityId(7), SpaceId(1), Vector3::zero());
    e.is_player = true;
    e
}

fn spec(effect_id: i32, invoker_id: u32) -> TimedEffectSpec {
    TimedEffectSpec {
        cast_id: Some(41),
        effect_id,
        ability_id: 637,
        invoker_id,
        effect_flags: 21,
        moniker_ids: vec![3_212_632_871],
        stats: vec![(ACCURACY, 200)],
        absorb: Vec::new(),
        state_flags: 0,
        duration_secs: Some(15.0),
        stacking: TimedStacking::PerSource,
        invoker_identity: Default::default(),
        invoker_name: None,
    }
}

/// Everything at once. The cooldowns go on first, because
/// `start_ability_cooldown` stamps its own `Instant::now()`; everything
/// else sits at a fixed offset from the returned `now`, so its remaining
/// time is exact.
fn seeded() -> (CellEntity, Instant) {
    let mut e = entity();
    e.abilities
        .start_ability_cooldown(592, Duration::from_secs(30));
    e.abilities
        .start_ability_cooldown(637, Duration::from_secs(10));
    // Lapsed by `now` but not yet swept: the snapshot leaves it out.
    e.abilities.start_ability_cooldown(700, Duration::ZERO);
    let now = Instant::now();
    e.pending_cast = Some(PendingCast {
        ability_id: 597,
        target_id: 9,
        wire_target_id: 0,
        ground: None,
        effect_seq: 44,
        received_at: std::time::Instant::now(),
        fire_at: now + Duration::from_millis(1500),
        warmup_secs: 2.0,
        anchor: Vector3::zero(),
        space_id: SpaceId(1),
        weapon_instance: None,
    });
    e.active_effects.push(ActiveEffectInstance {
        effect_id: 5001,
        ability_id: 800,
        invoker_id: 9,
        remaining_pulses: 3,
        total_pulses: 5,
        next_pulse_at: now + Duration::from_secs(2),
        pulse_interval_secs: 2.0,
        invoker_position_at_register: None,
        cast_id: Some(12),
        invoker_identity: Default::default(),
        invoker_name: None,
    });
    // A timed buff, a held stun (flag only) and a shield.
    e.apply_timed_effect(spec(700, 7), now).expect("buff");
    let stun = TimedEffectSpec {
        effect_id: 900,
        stats: vec![],
        state_flags: MOVEMENT_LOCK,
        duration_secs: None,
        ..spec(900, 9)
    };
    e.apply_timed_effect(stun, now).expect("stun");
    let shield = TimedEffectSpec {
        effect_id: 4306,
        stats: vec![],
        absorb: vec![(ABSORB_PHYSICAL, 500)],
        duration_secs: Some(30.0),
        ..spec(4306, 7)
    };
    e.apply_timed_effect(shield, now).expect("shield");
    (e, now)
}

#[test]
fn ab_t5_snapshot_reads_warmup_cooldowns_pulses_and_the_ledger() {
    let (e, now) = seeded();
    let s = e.ability_state(now);

    assert_eq!(s.entity_id, 7);
    assert!(s.is_player);
    let pc = s.pending_cast.as_ref().expect("warmup");
    assert_eq!((pc.ability_id, pc.cast_id, pc.target_id), (597, 44, 9));
    assert_eq!(pc.warmup_secs, 2.0);
    assert_eq!(pc.warmup_remaining_secs, 1.5);

    let cds: Vec<(i32, f32)> = s
        .cooldowns
        .iter()
        .map(|c| (c.ability_id, c.total_secs))
        .collect();
    assert_eq!(
        cds,
        vec![(592, 30.0), (637, 10.0)],
        "sorted, and the lapsed 700 is left out"
    );
    // Started a moment before `now`: within a tick of the full length.
    assert!((29.9..=30.0).contains(&s.cooldowns[0].remaining_secs));
    assert!((9.9..=10.0).contains(&s.cooldowns[1].remaining_secs));

    assert_eq!(s.pulsing.len(), 1);
    let p = &s.pulsing[0];
    assert_eq!((p.effect_id, p.invoker_id, p.cast_id), (5001, 9, Some(12)));
    assert_eq!((p.pulses_left, p.total_pulses), (3, 5));
    assert_eq!(p.next_pulse_in_secs, 2.0);

    let ids: Vec<i32> = s.ledger.iter().map(|l| l.effect_id).collect();
    assert_eq!(ids, vec![700, 900, 4306], "sorted by effect id");
    let buff = &s.ledger[0];
    assert_eq!(buff.cast_id, Some(41));
    assert_eq!(buff.expires_in_secs, Some(15.0));
    assert!(!buff.held);
    assert_eq!(buff.moniker_ids, vec![3_212_632_871]);
    assert_eq!(buff.stats.len(), 1);
    assert_eq!(
        (buff.stats[0].stat_id, buff.stats[0].requested),
        (ACCURACY, 200)
    );
    let stun = &s.ledger[1];
    assert!(stun.held);
    assert_eq!(stun.expires_in_secs, None);
    assert_eq!(stun.state_flags, MOVEMENT_LOCK);
    let shield = &s.ledger[2];
    assert_eq!(shield.absorb.len(), 1);
    assert_eq!(
        (shield.absorb[0].stat_id, shield.absorb[0].remaining),
        (ABSORB_PHYSICAL, 500)
    );

    assert_eq!(s.state_flag_refcounts.len(), 1);
    let r = &s.state_flag_refcounts[0];
    assert_eq!((r.bit, r.mask, r.count, r.set), (6, MOVEMENT_LOCK, 1, true));

    let acc = s.stat(ACCURACY).expect("accuracy");
    assert_eq!(acc.cur, e.stats.get(ACCURACY).unwrap().cur);
    assert!(s.stat(HEALTH).is_some(), "every stat, not a curated few");
    assert_eq!(s.stats.len(), e.stats.len());
    assert!(!s.is_quiet());
}

#[test]
fn ab_t5_a_fresh_entity_is_quiet() {
    let now = Instant::now();
    let s = entity().ability_state(now);
    assert!(s.is_quiet());
    assert!(s.state_flag_refcounts.is_empty());
}
