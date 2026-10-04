//! `StatBuff` and `TimedStat`: NVP parsing, the two stacking rules through
//! the scripts, `on_remove`, the skip paths and the telemetry rows.

use cimmeria_entity::abilities::{AbilityDef, EffectDef};
use cimmeria_entity::stats::{
    ACCURACY, COORDINATION, DEFENSE, ENGAGEMENT, INTELLIGENCE, MOVEMENT_SPEED_MOD, PERCEPTION,
};
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

/// Aim's effect 700 as the `stat` family writes it: "+200 Accuracy: 15
/// Seconds", flags 21 (Beneficial, ClearOnDeath, DontUseQR).
fn aim() -> EffectDef {
    let mut e = effect_with_nvp("Accuracy", "200");
    e.effect_id = 700;
    e.ability_id = 637;
    e.pulse_count = 1;
    e.pulse_duration = 15.0;
    e.flags = 21;
    e.script_name = Some("TimedStat".to_string());
    e
}

fn run_as(mgr: &mut SpaceManager, script: &dyn EffectScript, effect: &EffectDef, source: u32) {
    let mut ctx = EffectContext {
        source_id: source,
        target_id: 1,
        effect,
        space_mgr: mgr,
    };
    script.on_apply(&mut ctx);
}

fn run(mgr: &mut SpaceManager, effect: &EffectDef) {
    run_as(mgr, &StatBuff, effect, 1);
}

fn cur(mgr: &SpaceManager, stat: i32) -> i32 {
    mgr.get_entity(1).unwrap().stats.get(stat).unwrap().cur
}

fn entries(mgr: &SpaceManager) -> &[cimmeria_entity::cell_entity::TimedEffect] {
    &mgr.get_entity(1).unwrap().stat_buffs.entries
}

/// The fixture's primary attributes start at the `StatList` default 1/1.
#[test]
fn a_mark_iii_stim_raises_its_stat_by_the_nvp() {
    let mut mgr = make_mgr_with_target();
    run(&mut mgr, &stim(3950, 2735, "Coordination", "5"));
    assert_eq!(cur(&mgr, COORDINATION), 6);
    let buffs = entries(&mgr);
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
    assert_eq!(entries(&mgr).len(), 2);
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
        .find_event(Level::INFO, "timed effect removed", "replaced")
        .expect("a replacement logs the old entry's removal");
    assert!(replaced.fields.get("player_id").is_some_and(|v| v == "100"));
    assert!(replaced
        .fields
        .get("target_player_id")
        .is_some_and(|v| v == "100"));
    let applied = capture
        .find_message(Level::INFO, "timed effect applied")
        .expect("stat_buff_applied row");
    for field in [
        "stat_before",
        "stat_after",
        "effect_id",
        "ability_id",
        "target_id",
    ] {
        assert!(applied.fields.contains_key(field), "{field}");
    }
}

#[test]
fn the_registry_resolves_both_ledger_scripts() {
    assert!(registry::lookup("StatBuff").is_some());
    assert!(registry::lookup("TimedStat").is_some());
}

// ── TimedStat ──────────────────────────────────────────────────────────────

#[test]
fn timed_stat_applies_accuracy_for_the_pulse_duration() {
    let mut mgr = make_mgr_with_target();
    run_as(&mut mgr, &TimedStat, &aim(), 1);
    assert_eq!(cur(&mgr, ACCURACY), 200);
    let e = &entries(&mgr)[0];
    assert_eq!((e.effect_id, e.ability_id, e.invoker_id), (700, 637, 1));
    assert_eq!(e.duration_secs, 15.0);
    assert_eq!(e.effect_flags, 21);
    assert!(e.expires_at.is_some());
}

/// Same caster twice refreshes. On a revert to per-stat stacking this still
/// passes; on a revert to stat-blind stacking it reads 400.
#[test]
fn timed_stat_same_caster_refreshes() {
    let mut mgr = make_mgr_with_target();
    run_as(&mut mgr, &TimedStat, &aim(), 1);
    run_as(&mut mgr, &TimedStat, &aim(), 1);
    assert_eq!(cur(&mgr, ACCURACY), 200);
    assert_eq!(entries(&mgr).len(), 1);
}

/// Two casters stack. Under the stimpacks' stat-keyed rule (a revert of
/// `TimedStat` to `ReplaceSameStat`) the second would replace the first.
#[test]
fn timed_stat_different_casters_stack() {
    let mut mgr = make_mgr_with_target();
    run_as(&mut mgr, &TimedStat, &aim(), 1);
    run_as(&mut mgr, &TimedStat, &aim(), 2);
    assert_eq!(cur(&mgr, ACCURACY), 400);
    assert_eq!(entries(&mgr).len(), 2);
}

#[test]
fn timed_stat_on_remove_takes_off_only_that_casters_entry() {
    let mut mgr = make_mgr_with_target();
    let effect = aim();
    run_as(&mut mgr, &TimedStat, &effect, 1);
    run_as(&mut mgr, &TimedStat, &effect, 2);
    let mut ctx = EffectContext {
        source_id: 2,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    TimedStat.on_remove(&mut ctx);
    assert_eq!(cur(&mgr, ACCURACY), 200);
    assert_eq!(entries(&mgr)[0].invoker_id, 1);
}

#[test]
fn timed_stat_moves_several_stats_as_one_entry() {
    let mut mgr = make_mgr_with_target();
    let mut e = effect_with_nvp("Accuracy", "-200");
    e.params.insert("Defense".to_string(), "-200".to_string());
    e.effect_id = 1980;
    e.ability_id = 1630;
    e.pulse_count = 1;
    e.pulse_duration = 15.0;
    run_as(&mut mgr, &TimedStat, &e, 7);
    assert_eq!(cur(&mgr, ACCURACY), -200);
    assert_eq!(cur(&mgr, DEFENSE), -200);
    assert_eq!(entries(&mgr).len(), 1);
    assert_eq!(entries(&mgr)[0].stats.len(), 2);
}

#[test]
fn timed_stat_moves_run_speed_in_movement_speed_mod_percent() {
    let mut mgr = make_mgr_with_target();
    let mut e = effect_with_nvp("MovementSpeedMod", "50");
    e.effect_id = 1962;
    e.ability_id = 1619;
    e.pulse_count = 1;
    e.pulse_duration = 10.0;
    run_as(&mut mgr, &TimedStat, &e, 1);
    assert_eq!(cur(&mgr, MOVEMENT_SPEED_MOD), 150);
}

#[test]
fn timed_stat_records_the_abilitys_monikers() {
    let mut mgr = make_mgr_with_target();
    mgr.ability_defs.insert(
        637,
        AbilityDef {
            ability_id: 637,
            name: "Aim".to_string(),
            cooldown: 30.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: false,
            min_range: 0.0,
            max_range: 0.0,
            target_type_id: 1,
            effect_ids: vec![700],
            moniker_ids: vec![3_212_632_871],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
            type_id: cimmeria_entity::abilities::AbilityType::Buff,
        },
    );
    run_as(&mut mgr, &TimedStat, &aim(), 1);
    assert_eq!(entries(&mgr)[0].moniker_ids, vec![3_212_632_871]);
    let removed = mgr.remove_timed_effects_by_moniker(1, 3_212_632_871);
    assert_eq!(removed.len(), 1);
    assert_eq!(cur(&mgr, ACCURACY), 0);
}

/// A held effect (a stance's `pulse_duration = 0`) is refused until AB-08
/// can toggle it off.
#[test]
fn timed_stat_refuses_a_held_effect() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr_with_target();
    let mut e = aim();
    e.pulse_duration = 0.0;
    run_as(&mut mgr, &TimedStat, &e, 1);
    assert!(mgr.get_entity(1).unwrap().stat_buffs.is_idle());
    assert!(capture
        .find_event(Level::WARN, "TimedStat effect", "no_duration")
        .is_some());
}

/// The client draws ten icons per side (B-73): the eleventh beneficial entry
/// still applies, and logs `effect_bar_overflow`; the tenth does not.
#[test]
fn an_eleventh_buff_applies_and_logs_the_effect_bar_overflow() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr_with_target();
    for invoker in 1..=10 {
        run_as(&mut mgr, &TimedStat, &aim(), invoker);
    }
    assert!(capture
        .find_message(Level::INFO, "more timed effects on one side")
        .is_none());
    run_as(&mut mgr, &TimedStat, &aim(), 11);
    assert_eq!(cur(&mgr, ACCURACY), 2200, "the eleventh still applies");
    let row = capture
        .find_message(Level::INFO, "more timed effects on one side")
        .expect("effect_bar_overflow row");
    assert!(row.fields.get("icons").is_some_and(|v| v == "11"));
    assert!(row.fields.contains_key("target_player_id"));
}

/// The generator's `stat` family writes only names this script reads. The
/// list between the `nvp-names` markers in `families/stat.py` must be a
/// subset of [`STAT_BUFF_NVPS`]; a name the generator writes and the script
/// ignores would be a buff that silently does nothing.
#[test]
fn stat_nvp_names_match_the_generator() {
    let src = include_str!("../../../../../../tools/ability_mechanics/families/stat.py");
    let start = src.find("# nvp-names begin").expect("begin marker");
    let end = src.find("# nvp-names end").expect("end marker");
    let names: Vec<&str> = src[start..end].split('"').skip(1).step_by(2).collect();
    assert!(names.len() >= 10, "parsed {names:?}");
    for name in names {
        assert!(
            STAT_BUFF_NVPS.iter().any(|&(n, _)| n == name),
            "generator writes {name}, which no ledger script reads"
        );
    }
}
