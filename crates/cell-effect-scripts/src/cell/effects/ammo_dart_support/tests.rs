//! `RemoveEffects`, the support-dart rows and their on-hit effects, and the
//! seed guard (ammo campaign AM-11c).

use std::collections::HashMap;
use std::time::Instant;

use cimmeria_entity::ammo_type::{
    DART_ADRENALINE, DART_ANTIDOTE, DART_COAGULANT, DART_DEFAULT, DART_NANITES, DART_STIM,
};
use cimmeria_entity::cell_entity::{ActiveEffectInstance, BandolierItem};
use cimmeria_entity::stats::{FOCUS, HEALTH};
use cimmeria_wire::state_field::BSF_MOVEMENT_LOCK;

use super::*;
use crate::cell::effects::ammo_damage::shot_ammo;
use crate::cell::effects::test_fixtures::make_mgr_with_target;
use crate::cell::effects::{EffectContext, EffectScript};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{AmmoCatalog, AmmoModifier};
use cimmeria_entity::abilities::EffectDef;

/// The on-hit effects and rows `ammo_modifiers_dart_support.sql` seeds.
/// `live_db_dart_support_seed_rows` pins the seed to these.
const STIM_EFFECT: i32 = 9160;
const ANTIDOTE_EFFECT: i32 = 9161;
const COAGULANT_EFFECT: i32 = 9162;
const ADRENALINE_EFFECT: i32 = 9163;
const ANTIDOTE_CATEGORIES: &str = "Poison,Disease,Contagion,Wound,Burning";

/// `(ammo_type, on_hit_effect_id, toggle_ability_id)` of each seeded row.
const SEEDED: [(i32, i32, i32); 4] = [
    (DART_STIM, STIM_EFFECT, 992),
    (DART_ANTIDOTE, ANTIDOTE_EFFECT, 1228),
    (DART_COAGULANT, COAGULANT_EFFECT, 3427),
    (DART_ADRENALINE, ADRENALINE_EFFECT, 1220),
];

fn seeded_rows() -> Vec<AmmoModifier> {
    SEEDED
        .iter()
        .map(|&(ammo_type, effect, toggle)| AmmoModifier {
            ammo_type,
            damage_mult: DART_SUPPORT_DAMAGE_MULT,
            penetration_mult: 1.0,
            damage_type: None,
            on_hit_effect_id: Some(effect),
            toggle_ability_id: toggle,
            // AM-11d: all four are beneficial, so they target allies.
            beneficial: true,
        })
        .collect()
}

fn effect(
    effect_id: i32,
    script: Option<&str>,
    nvps: &[(&str, &str)],
    pulse_duration: f32,
) -> EffectDef {
    EffectDef {
        effect_id,
        script_name: script.map(str::to_string),
        params: nvps
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>(),
        pulse_duration,
        ..Default::default()
    }
}

/// The four on-hit effects as seeded.
fn seeded_effects() -> Vec<EffectDef> {
    vec![
        effect(
            STIM_EFFECT,
            Some("HealFocus"),
            &[("HealPercentage", "10")],
            0.0,
        ),
        effect(
            ANTIDOTE_EFFECT,
            Some("RemoveEffects"),
            &[(REMOVE_CATEGORIES_NVP, ANTIDOTE_CATEGORIES)],
            0.0,
        ),
        effect(
            COAGULANT_EFFECT,
            Some("RemoveEffects"),
            &[(REMOVE_CATEGORIES_NVP, "Wound")],
            0.0,
        ),
        effect(
            ADRENALINE_EFFECT,
            Some("HealHealth"),
            &[("HealPercentage", "10")],
            0.0,
        ),
    ]
}

/// A category-tagged DoT, `id`.
fn dot(id: i32, category: Option<&str>, script: Option<&str>) -> EffectDef {
    let nvps: Vec<(&str, &str)> = category
        .map(|c| (EFFECT_CATEGORY_NVP, c))
        .into_iter()
        .collect();
    let mut e = effect(id, script, &nvps, 1.0);
    e.pulse_count = 8;
    e
}

fn instance(effect_id: i32, invoker_id: u32) -> ActiveEffectInstance {
    ActiveEffectInstance {
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
    }
}

/// Entity 1 carrying `effects` as active instances, each `(def, invoker)`.
fn mgr_with_active(effects: &[(EffectDef, u32)]) -> SpaceManager {
    let mut mgr = make_mgr_with_target();
    for (def, invoker) in effects {
        mgr.effect_defs.insert(def.effect_id, def.clone());
        mgr.get_entity_mut(1)
            .unwrap()
            .active_effects
            .push(instance(def.effect_id, *invoker));
    }
    mgr
}

fn cleanse(mgr: &mut SpaceManager, categories: &str) {
    let cleanse = effect(
        ANTIDOTE_EFFECT,
        Some("RemoveEffects"),
        &[(REMOVE_CATEGORIES_NVP, categories)],
        0.0,
    );
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &cleanse,
        space_mgr: mgr,
    };
    RemoveEffects.on_apply(&mut ctx);
}

fn active(mgr: &SpaceManager) -> Vec<(i32, u32)> {
    mgr.get_entity(1)
        .unwrap()
        .active_effects
        .iter()
        .map(|i| (i.effect_id, i.invoker_id))
        .collect()
}

#[test]
fn remove_categories_parses_the_nvp() {
    let e = effect(1, None, &[(REMOVE_CATEGORIES_NVP, " Poison, ,Wound ")], 0.0);
    assert_eq!(remove_categories(&e), ["Poison", "Wound"]);
    assert!(remove_categories(&effect(1, None, &[], 0.0)).is_empty());
    let tagged = dot(2, Some(" Burning "), None);
    assert_eq!(effect_category(&tagged), Some("Burning"));
    assert_eq!(effect_category(&dot(3, None, None)), None);
}

/// Antidote removes one of each listed category, the oldest first. A
/// second Poison from another invoker survives, as do an untagged DoT
/// and a category it does not list.
#[test]
fn antidote_removes_one_effect_of_each_listed_category() {
    let mut mgr = mgr_with_active(&[
        (dot(500, Some("Poison"), None), 7),
        (dot(500, Some("Poison"), None), 8),
        (dot(501, Some("wound"), None), 7),
        (dot(502, None, None), 7),
        (dot(503, Some("Snare"), None), 7),
        (dot(504, Some("Burning"), None), 9),
    ]);
    cleanse(&mut mgr, ANTIDOTE_CATEGORIES);
    assert_eq!(active(&mgr), [(500, 8), (502, 7), (503, 7)]);
}

/// Coagulant lists only Wound: a Poison stays.
#[test]
fn coagulant_removes_only_wound() {
    let mut mgr = mgr_with_active(&[
        (dot(500, Some("Poison"), None), 7),
        (dot(501, Some("Wound"), None), 7),
    ]);
    cleanse(&mut mgr, "Wound");
    assert_eq!(active(&mgr), [(500, 7)]);
}

/// A removed effect gets its script's `on_remove`: a Stun tagged Wound
/// releases its movement lock when the cleanse takes it off.
#[test]
fn a_removed_effect_runs_its_on_remove() {
    let stun = dot(510, Some("Wound"), Some("Stun"));
    let mut mgr = mgr_with_active(&[(stun.clone(), 7)]);
    let mut ctx = EffectContext {
        source_id: 7,
        target_id: 1,
        effect: &stun,
        space_mgr: &mut mgr,
    };
    crate::cell::effects::dispatch_by_name("Stun", &mut ctx);
    let locked = |m: &SpaceManager| m.get_entity(1).unwrap().state_field & BSF_MOVEMENT_LOCK != 0;
    assert!(locked(&mgr));
    cleanse(&mut mgr, "Wound");
    assert!(active(&mgr).is_empty());
    assert!(!locked(&mgr), "the Stun's on_remove must clear the lock");
}

/// No categories: nothing is removed.
#[test]
fn a_cleanse_without_categories_removes_nothing() {
    let mut mgr = mgr_with_active(&[(dot(500, Some("Poison"), None), 7)]);
    cleanse(&mut mgr, "");
    assert_eq!(active(&mgr), [(500, 7)]);
}

/// The support rows zero the shot: the multiplier times the largest
/// pre-armour damage it must absorb rounds to 0.
#[test]
fn the_support_multiplier_rounds_any_seeded_shot_to_zero() {
    let scale = f64::from(DART_SUPPORT_DAMAGE_MULT);
    assert!(scale > 0.0, "the table's CHECK requires damage_mult > 0");
    assert_eq!((4999.0 * scale).round() as i32, 0);
}

/// Player 1 with a dart pistol loaded with `ammo_type`, the seeded rows
/// and on-hit effects loaded.
fn mgr_loaded(ammo_type: i32) -> SpaceManager {
    let mut mgr = make_mgr_with_target();
    mgr.ammo_catalog = AmmoCatalog::from_rows(seeded_rows(), []);
    for e in seeded_effects() {
        mgr.effect_defs.insert(e.effect_id, e);
    }
    let p = mgr.get_entity_mut(1).unwrap();
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 1,
            item_id: 1520,
            clip_size: 10,
            default_ammo_type: DART_DEFAULT,
            current_ammo: 10,
            cur_ammo_type: ammo_type,
        },
    );
    mgr
}

fn dart_shot() -> cimmeria_entity::abilities::AbilityDef {
    cimmeria_entity::abilities::AbilityDef {
        ability_id: 1086,
        name: "Dart Pistol Auto Attack".to_string(),
        cooldown: 0.0,
        warmup: 0.0,
        flags: 0,
        is_ranged: true,
        min_range: 0.0,
        max_range: 30.0,
        target_type_id: 2,
        effect_ids: vec![],
        moniker_ids: vec![],
        required_ammo: 1,
        event_set_id: None,
        velocity: 0.0,
        type_id: Default::default(),
        passive: false,
    }
}

/// The chain the pipeline runs on a hit: resolve the row, resolve its
/// on-hit effect, dispatch the effect's script by name. Here the shooter
/// is also the target, since the chain does not care who is hit.
fn fire_on_hit(mgr: &mut SpaceManager) {
    let shot = shot_ammo(mgr, 1, Some(&dart_shot()), true).expect("a support row");
    assert_eq!(shot.damage_scale(), f64::from(DART_SUPPORT_DAMAGE_MULT));
    let eid = shot
        .on_hit_effect_id(mgr)
        .expect("the on-hit effect is loaded");
    let def = mgr.effect_defs.get(&eid).cloned().unwrap();
    let script = def.script_name.clone().unwrap();
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &def,
        space_mgr: mgr,
    };
    assert!(
        crate::cell::effects::dispatch_by_name(&script, &mut ctx),
        "{script} must be registered"
    );
}

/// Stim restores 10% of max Focus: 200/1000 becomes 300.
#[test]
fn a_stim_dart_restores_ten_percent_focus() {
    let mut mgr = mgr_loaded(DART_STIM);
    fire_on_hit(&mut mgr);
    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(FOCUS).unwrap().cur,
        300
    );
}

/// Adrenaline restores 10% of max Health: 50/100 becomes 60.
#[test]
fn an_adrenaline_dart_restores_ten_percent_health() {
    let mut mgr = mgr_loaded(DART_ADRENALINE);
    fire_on_hit(&mut mgr);
    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
        60
    );
}

/// Antidote and Coagulant dispatch the registered cleanse.
#[test]
fn antidote_and_coagulant_darts_cleanse() {
    for (ammo, left) in [(DART_ANTIDOTE, vec![]), (DART_COAGULANT, vec![(500, 7)])] {
        let mut mgr = mgr_loaded(ammo);
        for (def, invoker) in [
            (dot(500, Some("Poison"), None), 7),
            (dot(501, Some("Wound"), None), 7),
        ] {
            mgr.effect_defs.insert(def.effect_id, def.clone());
            mgr.get_entity_mut(1)
                .unwrap()
                .active_effects
                .push(instance(def.effect_id, invoker));
        }
        fire_on_hit(&mut mgr);
        assert_eq!(active(&mgr), left, "ammo type {ammo}");
    }
}

/// Nanites has no row: its shot fires unmodified.
#[test]
fn a_nanites_dart_fires_unmodified() {
    let mgr = mgr_loaded(DART_NANITES);
    assert_eq!(shot_ammo(&mgr, 1, Some(&dart_shot()), true), None);
}

/// The seed loads the four rows, their on-hit effects and NVPs as the
/// constants above say, Nanites has no row, and the provenance abilities
/// exist. Fails if the seed or its `\ir` line is dropped or a value
/// drifts.
#[tokio::test]
async fn live_db_dart_support_seed_rows() {
    let pool = crate::test_support::require_db_or_skip!();
    let catalog = crate::cell::spawner::load_ammo_catalog(&pool)
        .await
        .expect("ammo catalog loads");
    for row in seeded_rows() {
        assert_eq!(catalog.modifier(row.ammo_type), Some(&row));
    }
    assert_eq!(catalog.modifier(DART_NANITES), None, "Nanites: no evidence");
    // AM-11d: these four are the only beneficial rows. A damaging family
    // flagged by mistake would let its shots land on allies.
    let beneficial: Vec<String> = sqlx::query_scalar(
        "SELECT ammo_type::text FROM resources.ammo_modifiers WHERE beneficial          ORDER BY ammo_type::text",
    )
    .fetch_all(&pool)
    .await
    .expect("beneficial rows query");
    assert_eq!(
        beneficial,
        [
            "Dart_Adrenaline",
            "Dart_Antidote",
            "Dart_Coagulant",
            "Dart_Stim"
        ],
        "exactly the support darts are beneficial"
    );

    let defs = crate::cell::spawner::load_effect_defs(&pool)
        .await
        .expect("effect defs load");
    for want in seeded_effects() {
        let got = defs.get(&want.effect_id).expect("seeded on-hit effect");
        assert_eq!(got.script_name, want.script_name, "{}", want.effect_id);
        assert_eq!(got.params, want.params, "{}", want.effect_id);
        assert_eq!(got.pulse_count, 1, "{}", want.effect_id);
        assert_eq!(
            got.pulse_duration, want.pulse_duration,
            "{}",
            want.effect_id
        );
        let script = got.script_name.as_deref().unwrap();
        assert!(
            crate::cell::effects::registry::lookup(script).is_some(),
            "{script} is registered"
        );
    }

    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM resources.abilities WHERE ability_id IN (992, 1220, 1228, 3427) \
         ORDER BY ability_id",
    )
    .fetch_all(&pool)
    .await
    .expect("abilities query");
    assert_eq!(
        names,
        [
            "Dart Type: Beneficial: Stim",
            "Dart Type: Beneficial: Adrenaline",
            "Dart Type: Beneficial: Antidote",
            "MS021_081024_DartType:Beneficial:Coagulant",
        ]
    );
}
