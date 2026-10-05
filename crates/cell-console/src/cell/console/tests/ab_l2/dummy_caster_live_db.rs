//! Live-DB pins on the seeded abilities the ability UAT stages with
//! `.dummy caster` (rows AB-U20 and AB-U22,
//! `docs/analysis/ability-mechanics/lab-uat-and-telemetry.md`).
//!
//! - **AB-U20.** 1354 Disabling Shot is the caster's ability: a 4 s warmup
//!   (time to press Interrupting Shot, 657) and a 5 s cooldown, so the
//!   default 8 s interval places.
//! - **AB-U22.** Its two Health-tagged `TimedStat` debuffs (4335 -100
//!   Response, 4333 -200 Accuracy/Defense) are what a cleanse can take off.
//!   Its Mental-tagged Disorient (4334) has no mechanic, and no seeded Mental
//!   effect lands a held one, so Clear: Mind (2099, effect 2827 `Mental:5`)
//!   has nothing to remove; the row runs with Absolution (2865, effect 4169
//!   `Health:2`) until a Mental debuff is bound. When this test fails because
//!   a Mental effect gained a script, move AB-U22 back to 2099.

use cimmeria_entity::abilities::ability_is_beneficial;

use super::dummy::dummy_world;
use super::{console, lines, CALLER};
use crate::cell::space_manager::LabCaster;
use crate::cell::spawner::{load_ability_defs, load_effect_defs};
use crate::test_support::require_db_or_skip;

const DISABLING_SHOT: i32 = 1354;
const CLEAR_MIND_EFFECT: i32 = 2827;
const ABSOLUTION_HEALTH_EFFECT: i32 = 4169;

#[tokio::test]
async fn ab_l2_dummy_caster_uat_picks_live_db() {
    let pool = require_db_or_skip!();
    let mut mgr = dummy_world();
    mgr.ability_defs = load_ability_defs(&pool).await.expect("load_ability_defs");
    mgr.effect_defs = load_effect_defs(&pool).await.expect("load_effect_defs");

    // AB-U20: Disabling Shot places as a caster at the default interval.
    let shot = &mgr.ability_defs[&DISABLING_SHOT];
    assert_eq!((shot.warmup, shot.cooldown), (4.0, 5.0), "{shot:?}");
    assert!(!ability_is_beneficial(shot, &mgr.effect_defs));
    let out = lines(&console(&mut mgr, None, &format!(".dummy caster {DISABLING_SHOT}")).await);
    assert!(out[0].contains("placed: caster"), "{out:?}");
    let dummy = mgr.lab_dummies_of(CALLER)[0];
    let mark = *mgr
        .get_entity(dummy)
        .unwrap()
        .extensions
        .get::<LabCaster>()
        .unwrap();
    assert_eq!(mark.ability_id, DISABLING_SHOT);

    // AB-U22: what Disabling Shot leaves on its target, by category.
    let category = |id: i32| {
        let e = &mgr.effect_defs[&id];
        (
            e.script_name.clone(),
            e.params.get("EffectCategory").cloned(),
        )
    };
    let shot = &mgr.ability_defs[&DISABLING_SHOT];
    let mut held: Vec<(i32, Option<String>, Option<String>)> = shot
        .effect_ids
        .iter()
        .map(|&id| {
            let (script, cat) = category(id);
            (id, script, cat)
        })
        .filter(|(_, _, cat)| cat.is_some())
        .collect();
    held.sort();
    let s = |v: &str| Some(v.to_string());
    assert_eq!(
        held,
        vec![
            (4333, s("TimedStat"), s("Health")),
            (4334, None, s("Mental")),
            (4335, s("TimedStat"), s("Health")),
        ],
        "Disabling Shot's categorised effects"
    );
    let removes = |id: i32| mgr.effect_defs[&id].params.get("RemoveCategories").cloned();
    assert_eq!(removes(ABSOLUTION_HEALTH_EFFECT), s("Health:2"));
    assert_eq!(removes(CLEAR_MIND_EFFECT), s("Mental:5"));

    // No seeded Mental effect has a mechanic that holds on its target.
    let landing_mental: Vec<i32> = mgr
        .effect_defs
        .values()
        .filter(|e| e.params.get("EffectCategory").map(String::as_str) == Some("Mental"))
        .filter(|e| e.script_name.is_some() && (e.is_pulsing() || e.pulse_duration > 0.0))
        .filter(|e| e.script_name.as_deref() != Some("Suppression") || e.is_pulsing())
        .map(|e| e.effect_id)
        .collect();
    assert!(
        landing_mental.is_empty(),
        "a Mental effect now holds on its target ({landing_mental:?}): stage AB-U22 with Clear: Mind (2099) again"
    );
}
