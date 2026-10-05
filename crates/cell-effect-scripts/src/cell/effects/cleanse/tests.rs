//! `RemoveEffects` as an ability cleanse: counts, the ledger and pulsing
//! halves, polarity and the target rule. The dart rows' tests are in
//! `ammo_dart_support/tests.rs`.

use std::collections::HashMap;
use std::time::Instant;

use cimmeria_cell_world::cell::combat::HOSTILE_FACTION;
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::cell_entity::{ActiveEffectInstance, TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::{ACCURACY, DEFENSE};
use tracing::Level;

use super::*;
use crate::cell::effects::test_fixtures::make_mgr_with_target;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

/// The hostile NPC the target-rule tests aim at.
const HOSTILE: u32 = 2;

fn effect(effect_id: i32, flags: u32, nvps: &[(&str, &str)]) -> EffectDef {
    EffectDef {
        effect_id,
        ability_id: effect_id + 10_000,
        flags,
        params: nvps
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>(),
        ..Default::default()
    }
}

/// Absolution's 4168 as the `cleanse` family writes it.
fn purge(categories: &str) -> EffectDef {
    let mut e = effect(
        4168,
        0,
        &[
            (REMOVE_CATEGORIES_NVP, categories),
            (REMOVE_POLARITY_NVP, "Harmful"),
        ],
    );
    e.ability_id = 2865;
    e.script_name = Some("RemoveEffects".to_string());
    e
}

/// Entity 1 (a player) plus a hostile NPC, and the tagged effect defs.
fn world(defs: &[EffectDef]) -> SpaceManager {
    let mut mgr = make_mgr_with_target();
    mgr.create_entity(HOSTILE, "W", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(HOSTILE).unwrap().faction = HOSTILE_FACTION;
    for d in defs {
        mgr.effect_defs.insert(d.effect_id, d.clone());
    }
    mgr
}

/// A pulsing instance of `effect_id` on `target`.
fn pulsing(mgr: &mut SpaceManager, target: u32, effect_id: i32, invoker_id: u32) {
    mgr.get_entity_mut(target)
        .unwrap()
        .active_effects
        .push(ActiveEffectInstance {
            invoker_identity: Default::default(),
            invoker_name: None,
            cast_id: None,
            effect_id,
            ability_id: 0,
            invoker_id,
            remaining_pulses: 5,
            total_pulses: 8,
            next_pulse_at: Instant::now(),
            pulse_interval_secs: 1.0,
            invoker_position_at_register: None,
        });
}

/// A 15 s ledger entry of `effect_id` on `target` moving `stat` by `delta`.
fn timed(
    mgr: &mut SpaceManager,
    target: u32,
    effect_id: i32,
    invoker_id: u32,
    stat: i32,
    delta: i32,
) {
    let flags = mgr.effect_defs[&effect_id].flags;
    mgr.apply_timed_effect(
        target,
        TimedEffectSpec {
            cast_id: None,
            effect_id,
            ability_id: effect_id + 10_000,
            invoker_id,
            effect_flags: flags,
            moniker_ids: vec![],
            stats: vec![(stat, delta)],
            absorb: Vec::new(),
            state_flags: 0,
            duration_secs: Some(15.0),
            stacking: TimedStacking::PerSource,
            invoker_identity: Default::default(),
            invoker_name: None,
        },
        Instant::now(),
    )
    .expect("the stat exists");
}

fn cast(mgr: &mut SpaceManager, cleanse: &EffectDef, source: u32, target: u32) {
    let mut ctx = EffectContext {
        source_id: source,
        target_id: target,
        effect: cleanse,
        space_mgr: mgr,
    };
    RemoveEffects.on_apply(&mut ctx);
}

/// `(effect_id, invoker)` of every pulsing instance and ledger entry left.
fn held(mgr: &SpaceManager, target: u32) -> (Vec<(i32, u32)>, Vec<(i32, u32)>) {
    let e = mgr.get_entity(target).unwrap();
    (
        e.active_effects
            .iter()
            .map(|i| (i.effect_id, i.invoker_id))
            .collect(),
        e.stat_buffs.entries.iter().map(|b| b.key()).collect(),
    )
}

#[test]
fn remove_categories_expands_counts() {
    let slots = |v: &str| remove_categories(&effect(1, 0, &[(REMOVE_CATEGORIES_NVP, v)]));
    assert_eq!(
        slots("Mental:2, Health:2"),
        ["Mental", "Mental", "Health", "Health"]
    );
    assert_eq!(slots("Mental : 5"), vec!["Mental"; 5]);
    assert_eq!(
        slots("Poison,Wound"),
        ["Poison", "Wound"],
        "a bare name is one"
    );
    assert!(
        slots("Mental:0,Health:x,:3").is_empty(),
        "bad counts and names drop"
    );
}

#[test]
fn polarity_defaults_to_harmful_and_rejects_unknown_values() {
    assert_eq!(polarity(&effect(1, 0, &[])), Some(Polarity::Harmful));
    let p = |v: &str| polarity(&effect(1, 0, &[(REMOVE_POLARITY_NVP, v)]));
    assert_eq!(p("beneficial"), Some(Polarity::Beneficial));
    assert_eq!(p("Harmful"), Some(Polarity::Harmful));
    assert_eq!(p("Both"), None);
}

/// Absolution's "Purge Mental Effects: X 2" with three Mental debuffs on
/// the caster (one pulsing, two on the ledger) removes exactly two, the
/// pulsing one first, and leaves the third, a Health debuff and an
/// untagged one.
#[test]
fn a_cleanse_removes_exactly_the_named_count() {
    let suppression = effect(1474, 78, &[(EFFECT_CATEGORY_NVP, "Mental")]);
    let disorient = effect(4334, 76, &[(EFFECT_CATEGORY_NVP, "mental")]);
    let fear = effect(1486, 70, &[(EFFECT_CATEGORY_NVP, "Mental")]);
    let wound = effect(4237, 68, &[(EFFECT_CATEGORY_NVP, "Health")]);
    let untagged = effect(903, 68, &[]);
    let mut mgr = world(&[suppression, disorient, fear, wound, untagged]);
    pulsing(&mut mgr, 1, 1474, HOSTILE);
    timed(&mut mgr, 1, 4334, HOSTILE, ACCURACY, -100);
    timed(&mut mgr, 1, 1486, HOSTILE, DEFENSE, -100);
    pulsing(&mut mgr, 1, 4237, HOSTILE);
    timed(&mut mgr, 1, 903, HOSTILE, DEFENSE, -50);

    cast(&mut mgr, &purge("Mental:2"), 1, 1);

    let (pulsing_left, ledger_left) = held(&mgr, 1);
    assert_eq!(
        pulsing_left,
        [(4237, HOSTILE)],
        "the Mental DoT went, the Health one stays"
    );
    assert_eq!(
        ledger_left,
        [(1486, HOSTILE), (903, HOSTILE)],
        "exactly one Mental ledger entry went (the older), the third Mental and the untagged stay"
    );
    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.stats.get(ACCURACY).unwrap().cur,
        0,
        "the cleansed debuff reverted"
    );
    assert!(
        e.stat_buffs.pending_timer_clears.contains(&(1474, HOSTILE))
            && e.stat_buffs.pending_timer_clears.contains(&(4334, HOSTILE)),
        "both removed icons are owed a clear"
    );
}

/// A harmful cleanse never takes a beneficial effect, even of its category:
/// the caster's own Mental-tagged buff stays.
#[test]
fn a_harmful_cleanse_never_strips_the_casters_buffs() {
    let buff = effect(3129, 21, &[(EFFECT_CATEGORY_NVP, "Mental")]);
    let debuff = effect(1474, 78, &[(EFFECT_CATEGORY_NVP, "Mental")]);
    let mut mgr = world(&[buff, debuff]);
    timed(&mut mgr, 1, 3129, 1, ACCURACY, 50);
    timed(&mut mgr, 1, 1474, HOSTILE, DEFENSE, -100);
    cast(&mut mgr, &purge("Mental:5"), 1, 1);
    assert_eq!(held(&mgr, 1).1, [(3129, 1)]);
}

/// A purge that lands on a hostile removes nothing and says why.
#[test]
fn a_harmful_cleanse_on_a_hostile_removes_nothing() {
    let debuff = effect(1474, 78, &[(EFFECT_CATEGORY_NVP, "Mental")]);
    let mut mgr = world(&[debuff]);
    pulsing(&mut mgr, HOSTILE, 1474, 1);
    let capture = LogCapture::install();
    cast(&mut mgr, &purge("Mental:2"), 1, HOSTILE);
    assert_eq!(held(&mgr, HOSTILE).0, [(1474, 1)]);
    assert!(capture
        .find_event(
            Level::WARN,
            "debuff cleanse landed on a target",
            "target_not_ally"
        )
        .is_some());
}

/// A buff strip removes a hostile's beneficial effects of the category and
/// is refused on the caster.
#[test]
fn a_buff_strip_works_on_a_hostile_and_never_on_the_caster() {
    let buff = effect(3129, 21, &[(EFFECT_CATEGORY_NVP, "Mental")]);
    let mut mgr = world(&[buff]);
    timed(&mut mgr, HOSTILE, 3129, HOSTILE, ACCURACY, 50);
    timed(&mut mgr, 1, 3129, 1, ACCURACY, 50);
    let mut strip = purge("Mental");
    strip
        .params
        .insert(REMOVE_POLARITY_NVP.to_string(), "Beneficial".to_string());

    let capture = LogCapture::install();
    cast(&mut mgr, &strip, 1, 1);
    assert_eq!(held(&mgr, 1).1, [(3129, 1)], "the caster keeps its buff");
    assert!(capture
        .find_event(
            Level::WARN,
            "buff strip landed on the caster",
            "target_not_hostile"
        )
        .is_some());

    cast(&mut mgr, &strip, 1, HOSTILE);
    assert!(
        held(&mgr, HOSTILE).1.is_empty(),
        "the hostile's buff is stripped"
    );
}

/// Another player in the caster's space is an ally: a purge helps them.
#[test]
fn a_harmful_cleanse_helps_an_ally_player() {
    let debuff = effect(1474, 78, &[(EFFECT_CATEGORY_NVP, "Mental")]);
    let mut mgr = world(&[debuff]);
    mgr.create_entity(3, "W", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    {
        let ally = mgr.get_entity_mut(3).unwrap();
        ally.is_player = true;
        ally.player_id = Some(101);
    }
    pulsing(&mut mgr, 3, 1474, HOSTILE);
    assert_eq!(relation(&mgr, 1, 3), Relation::Ally);
    cast(&mut mgr, &purge("Mental:2"), 1, 3);
    assert!(held(&mgr, 3).0.is_empty());
}
