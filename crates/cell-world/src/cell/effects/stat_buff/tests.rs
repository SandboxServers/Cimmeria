//! `StatBuff`: NVP parsing, the stat-keyed replace rule through the script,
//! `on_remove`, and the skip paths.

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::{COORDINATION, ENGAGEMENT, INTELLIGENCE, PERCEPTION};
use tracing::Level;

use super::*;
use crate::cell::effects::registry;
use crate::cell::effects::test_fixtures::{effect_with_nvp, make_mgr_with_target};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

/// A stimpack effect row: `pulse_count = 1`, `pulse_duration = 3600`,
/// `flags = 2`, one stat NVP.
fn stim(effect_id: i32, ability_id: i32, nvp: &str, value: &str) -> EffectDef {
    let mut e = effect_with_nvp(nvp, value);
    e.effect_id = effect_id;
    e.ability_id = ability_id;
    e.pulse_count = 1;
    e.pulse_duration = 3600.0;
    e.flags = 2;
    e.script_name = Some("StatBuff".to_string());
    e
}

fn run(mgr: &mut SpaceManager, effect: &EffectDef) {
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect,
        space_mgr: mgr,
    };
    StatBuff.on_apply(&mut ctx);
}

fn cur(mgr: &SpaceManager, stat: i32) -> i32 {
    mgr.get_entity(1).unwrap().stats.get(stat).unwrap().cur
}

/// The fixture's primary attributes start at the `StatList` default 1/1.
#[test]
fn a_mark_iii_stim_raises_its_stat_by_the_nvp() {
    let mut mgr = make_mgr_with_target();
    run(&mut mgr, &stim(3950, 2735, "Coordination", "5"));
    assert_eq!(cur(&mgr, COORDINATION), 6);
    let buffs = &mgr.get_entity(1).unwrap().stat_buffs.buffs;
    assert_eq!(buffs.len(), 1);
    assert_eq!((buffs[0].effect_id, buffs[0].ability_id), (3950, 2735));
    assert_eq!(buffs[0].effect_flags, 2);
    assert_eq!(buffs[0].duration_secs, 3600.0);
}

#[test]
fn a_mark_v_stims_two_effects_land_on_two_stats() {
    let mut mgr = make_mgr_with_target();
    run(&mut mgr, &stim(3955, 2740, "Engagement", "3"));
    run(&mut mgr, &stim(3956, 2740, "Coordination", "7"));
    assert_eq!(cur(&mgr, ENGAGEMENT), 4);
    assert_eq!(cur(&mgr, COORDINATION), 8);
    assert_eq!(mgr.get_entity(1).unwrap().stat_buffs.buffs.len(), 2);
}

#[test]
fn a_mark_v_coordination_replaces_a_mark_iii_coordination() {
    let mut mgr = make_mgr_with_target();
    run(&mut mgr, &stim(3950, 2735, "Coordination", "5"));
    run(&mut mgr, &stim(3956, 2740, "Coordination", "7"));
    assert_eq!(cur(&mgr, COORDINATION), 8, "1 + 7, not 1 + 5 + 7");
}

#[test]
fn intellect_moves_the_intelligence_stat() {
    let mut mgr = make_mgr_with_target();
    run(&mut mgr, &stim(3953, 2738, "Intellect", "5"));
    assert_eq!(cur(&mgr, INTELLIGENCE), 6);
}

#[test]
fn on_remove_takes_off_only_its_own_effect() {
    let mut mgr = make_mgr_with_target();
    let coord = stim(3950, 2735, "Coordination", "5");
    run(&mut mgr, &coord);
    run(&mut mgr, &stim(3954, 2739, "Perception", "5"));
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &coord,
        space_mgr: &mut mgr,
    };
    StatBuff.on_remove(&mut ctx);
    assert_eq!(cur(&mgr, COORDINATION), 1);
    assert_eq!(cur(&mgr, PERCEPTION), 6);
}

#[test]
fn an_effect_without_a_stat_nvp_warns_and_applies_nothing() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr_with_target();
    run(&mut mgr, &stim(3950, 2735, "NotAStat", "5"));
    assert!(mgr.get_entity(1).unwrap().stat_buffs.is_idle());
    assert!(capture
        .find_event(Level::WARN, "StatBuff effect", "no_stat_nvps")
        .is_some());
}

#[test]
fn an_effect_without_a_duration_warns_and_applies_nothing() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr_with_target();
    let mut e = stim(3950, 2735, "Coordination", "5");
    e.pulse_duration = 0.0;
    run(&mut mgr, &e);
    assert_eq!(cur(&mgr, COORDINATION), 1);
    assert!(capture
        .find_event(Level::WARN, "StatBuff effect", "no_duration")
        .is_some());
}

#[test]
fn a_missing_target_applies_nothing() {
    let mut mgr = make_mgr_with_target();
    let effect = stim(3950, 2735, "Coordination", "5");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 99_999,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    StatBuff.on_apply(&mut ctx);
    assert!(mgr.get_entity(1).unwrap().stat_buffs.is_idle());
}

#[test]
fn applied_and_replaced_rows_carry_identity_and_values() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr_with_target();
    run(&mut mgr, &stim(3950, 2735, "Coordination", "5"));
    run(&mut mgr, &stim(3956, 2740, "Coordination", "7"));
    let replaced = capture
        .find_event(Level::INFO, "stat buff removed", "replaced")
        .expect("a replacement logs the old buff's removal");
    assert!(replaced.fields.get("player_id").is_some_and(|v| v == "100"));
    let applied = capture
        .find_message(Level::INFO, "stat buff applied")
        .expect("stat_buff_applied row");
    assert!(applied.fields.contains_key("stat_before"));
    assert!(applied.fields.contains_key("stat_after"));
}

#[test]
fn the_registry_resolves_stat_buff() {
    assert!(registry::lookup("StatBuff").is_some());
}
