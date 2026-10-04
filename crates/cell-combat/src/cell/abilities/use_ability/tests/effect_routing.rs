//! AB-07: per-effect routing in the cast pipeline (audit B-27, B-28).
//!
//! `duel_gate`'s fixture: player A (1) casts at the origin, player B (2) is
//! an ally at (3,0,0), player C (3) another ally at (0,0,3), NPC 4 a hostile
//! mob at (2,0,0); this file adds player D (6) at (20,0,0), out of every
//! radius. Everyone starts at 0/1000 Focus with nothing dirty.
//!
//! The abilities copy the seed's shapes:
//!
//! - 869 Morale Boost: Self, Heal-typed, effects `{1215, 939}`: 1215 "Short
//!   Radius AE 35% Focus Heal" (`TCM_AERadius`, tier Short = 5 m) and 939
//!   "Rally User Focus heal" (`TCM_Single`), both `HealFocus` 35, flags 16.
//! - 1619 Combat Sprint: Self, Buff-typed, 1962 `TimedStat`
//!   `MovementSpeedMod 50` (flags 23) and 2002 `TimedStat` `Accuracy -100`
//!   (flags 534, no beneficial bit), both 10 s: the penalty half bound, as
//!   the `stat` family binds it once AB-07 routes it.
//! - 7001, a test attack: Target, a QR-rolled `HealthDamage 30` single
//!   (7101) and a user half (7102, `EF_ResolveOnAbilityUser`, `TimedStat`
//!   `Accuracy 200`, 15 s).

use cimmeria_entity::abilities::{
    AbilityType, EffectDef, EF_RESOLVE_ON_ABILITY_USER, RC_MISS, TARGET_SELF, TARGET_TARGET,
    TCM_AE_RADIUS,
};
use cimmeria_entity::stats::{ACCURACY, FOCUS, HEALTH, MOVEMENT_SPEED_MOD};

use super::duel_gate::{duel_mgr, A, B, MOB};
use super::*;
use crate::cell::abilities::rng::pseudo_random_seed;
use crate::cell::combat;
use crate::test_support::LogCapture;

const C: u32 = 3;
const D: u32 = 6;
const MORALE_BOOST: i32 = 869;
const RALLY_AREA: i32 = 1215;
const RALLY_USER: i32 = 939;
const COMBAT_SPRINT: i32 = 1619;
const SPRINT_RUN: i32 = 1962;
const SPRINT_PENALTY: i32 = 2002;
const MIXED: i32 = 7001;
const MIXED_HIT: i32 = 7101;
const MIXED_USER: i32 = 7102;
const MAX: i32 = 1000;
const MOB_HEALTH: i32 = 100_000;

fn effect(
    id: i32,
    ability: i32,
    script: Option<&str>,
    flags: u32,
    params: &[(&str, &str)],
) -> EffectDef {
    EffectDef {
        effect_id: id,
        ability_id: ability,
        script_name: script.map(str::to_string),
        flags,
        params: params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}

fn timed(mut e: EffectDef, secs: f32) -> EffectDef {
    e.pulse_count = 1;
    e.pulse_duration = secs;
    e
}

/// The fixture (module docs). Also the live-DB guard's entities.
pub(super) fn routing_mgr() -> SpaceManager {
    let mut mgr = duel_mgr();
    crate::test_support::install_effect_scripts(&mut mgr);
    mgr.create_entity(D, "Castle", [20.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    {
        let d = mgr.get_entity_mut(D).unwrap();
        d.is_player = true;
        d.player_id = Some(106);
    }
    mgr.connect_entity(D);
    let _ = mgr.compute_aoi_changes();

    let mut area = effect(
        RALLY_AREA,
        MORALE_BOOST,
        Some("HealFocus"),
        16,
        &[("HealPercentage", "35")],
    );
    area.target_collection_method = TCM_AE_RADIUS.to_string();
    area.tcm_param1 = "Short".to_string();
    let effects = [
        area,
        effect(
            RALLY_USER,
            MORALE_BOOST,
            Some("HealFocus"),
            16,
            &[("HealPercentage", "35")],
        ),
        timed(
            effect(
                SPRINT_RUN,
                COMBAT_SPRINT,
                Some("TimedStat"),
                23,
                &[("MovementSpeedMod", "50")],
            ),
            10.0,
        ),
        timed(
            effect(
                SPRINT_PENALTY,
                COMBAT_SPRINT,
                Some("TimedStat"),
                534,
                &[("Accuracy", "-100")],
            ),
            10.0,
        ),
        effect(MIXED_HIT, MIXED, None, 0, &[("HealthDamage", "30")]),
        timed(
            effect(
                MIXED_USER,
                MIXED,
                Some("TimedStat"),
                EF_RESOLVE_ON_ABILITY_USER | 1,
                &[("Accuracy", "200")],
            ),
            15.0,
        ),
    ];
    for e in effects {
        mgr.effect_defs.insert(e.effect_id, e);
    }
    for (id, target_type_id, effect_ids, type_id) in [
        (
            MORALE_BOOST,
            TARGET_SELF,
            vec![RALLY_AREA, RALLY_USER],
            AbilityType::Heal,
        ),
        (
            COMBAT_SPRINT,
            TARGET_SELF,
            vec![SPRINT_RUN, SPRINT_PENALTY],
            AbilityType::Buff,
        ),
        (
            MIXED,
            TARGET_TARGET,
            vec![MIXED_HIT, MIXED_USER],
            AbilityType::DirectDamage,
        ),
    ] {
        mgr.ability_defs.insert(
            id,
            AbilityDef {
                cooldown: 30.0,
                target_type_id,
                effect_ids,
                type_id,
                ..make_ability(id, 0, 30)
            },
        );
    }
    for eid in [A, B, C, D, MOB] {
        let e = mgr.get_entity_mut(eid).unwrap();
        e.stats.get_mut(FOCUS).unwrap().update(0, 0, MAX);
        e.stats.clear_dirty();
    }
    mgr.get_entity_mut(MOB)
        .unwrap()
        .stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, MOB_HEALTH, MOB_HEALTH);
    let a = mgr.get_entity_mut(A).unwrap();
    for id in [MORALE_BOOST, COMBAT_SPRINT, MIXED] {
        a.abilities.add_ability(id);
    }
    mgr
}

fn stat(mgr: &SpaceManager, eid: u32, id: i32) -> i32 {
    mgr.get_entity(eid).unwrap().stats.get(id).unwrap().cur
}

/// Make A's next cast of `ability` at MOB roll a miss (`want_miss`) or not.
fn next_roll(mgr: &mut SpaceManager, ability: i32, want_miss: bool) {
    let qr = combat::calculate_qr(
        &mgr.get_entity(A).unwrap().stats,
        &mgr.get_entity(MOB).unwrap().stats,
        false,
    );
    let first = mgr.get_entity(A).unwrap().abilities.effect_sequence_id;
    let seq = (first..first + 10_000)
        .find(|&s| {
            let seed = pseudo_random_seed(A, ability, s as u32);
            (combat::calculate_result(qr, seed).result_code == RC_MISS) == want_miss
        })
        .expect("a seed in range rolls the wanted result");
    mgr.get_entity_mut(A).unwrap().abilities.effect_sequence_id = seq;
}

/// **Regression guard (B-28).** Morale Boost with nothing selected restores
/// 35% Focus to the caster (939) and to the allies within 5 m (1215): B and
/// C. D, 20 m away, and the mob get nothing, and the caster is not healed
/// twice. On revert (`fire_beneficial` lands every effect on the resolved
/// target) the caster takes both heals and the allies none: `B's Focus`
/// fails.
#[tokio::test]
async fn morale_boost_heals_the_caster_and_the_allies_in_its_radius() {
    let mut mgr = routing_mgr();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(A, MORALE_BOOST, 0, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);

    assert_eq!(stat(&mgr, B, FOCUS), 350, "B's Focus (ally at 3 m)");
    assert_eq!(stat(&mgr, C, FOCUS), 350, "C's Focus (ally at 3 m)");
    assert_eq!(
        stat(&mgr, A, FOCUS),
        350,
        "A's Focus: 939 once, no area double dip"
    );
    assert_eq!(
        stat(&mgr, D, FOCUS),
        0,
        "D at 20 m is outside the Short radius"
    );
    assert_eq!(stat(&mgr, MOB, FOCUS), 0, "a hostile is never an ally");
    for ally in [B, C] {
        assert!(
            super::warmup::calls(&msgs)
                .iter()
                .any(|(e, m, _)| *e == ally && *m == method_idx::ON_STAT_UPDATE),
            "{ally} hears its Focus change"
        );
    }
}

/// **Regression guard (B-27).** Combat Sprint with its penalty half bound is
/// no longer beneficial, so before AB-07 it took the hostile path: with the
/// mob selected both halves landed on the mob (`the mob's run speed`
/// fails), and with the caster or an ally selected #444 refused the press
/// (`must commit` fails). Now both halves land on the caster, whatever the
/// client had selected, and the press always commits.
#[tokio::test]
async fn combat_sprint_lands_both_halves_on_the_caster_whatever_is_selected() {
    for wire in [MOB as i32, 0, A as i32, B as i32] {
        let mut mgr = routing_mgr();
        let (tx, _rx) = mpsc::channel(256);

        assert!(
            handle_use_ability(A, COMBAT_SPRINT, wire, &tx, &mut mgr).await,
            "Combat Sprint at wire target {wire} must commit"
        );

        assert_eq!(
            stat(&mgr, MOB, MOVEMENT_SPEED_MOD),
            100,
            "the mob's run speed ({wire})"
        );
        assert_eq!(stat(&mgr, MOB, ACCURACY), 0, "the mob's Accuracy ({wire})");
        assert_eq!(stat(&mgr, B, ACCURACY), 0, "B's Accuracy ({wire})");
        assert_eq!(
            stat(&mgr, A, MOVEMENT_SPEED_MOD),
            150,
            "A's run speed ({wire})"
        );
        assert_eq!(
            stat(&mgr, A, ACCURACY),
            -100,
            "A's Accuracy penalty ({wire})"
        );
        assert_eq!(
            stat(&mgr, MOB, HEALTH),
            MOB_HEALTH,
            "nothing hits the mob ({wire})"
        );
    }
}

/// **Regression guard (B-27, the miss gate).** An attack with a user half
/// that rolls a miss deals nothing to the mob, but the user half still lands
/// on the caster: it never rolls QR. On revert (the user half rides the
/// damage pipeline) the miss drops it: `A's Accuracy` fails.
#[tokio::test]
async fn a_missed_attack_still_lands_its_user_half() {
    let mut mgr = routing_mgr();
    next_roll(&mut mgr, MIXED, true);
    let (tx, _rx) = mpsc::channel(256);

    assert!(handle_use_ability(A, MIXED, MOB as i32, &tx, &mut mgr).await);

    assert_eq!(stat(&mgr, MOB, HEALTH), MOB_HEALTH, "a miss deals nothing");
    assert_eq!(stat(&mgr, A, ACCURACY), 200, "A's Accuracy (user half)");
    assert_eq!(stat(&mgr, MOB, ACCURACY), 0, "the mob's Accuracy");
}

/// On a hit, the mob takes the damage half and only the damage half.
#[tokio::test]
async fn a_hit_lands_the_damage_on_the_mob_and_the_user_half_on_the_caster() {
    let mut mgr = routing_mgr();
    next_roll(&mut mgr, MIXED, false);
    let (tx, _rx) = mpsc::channel(256);

    assert!(handle_use_ability(A, MIXED, MOB as i32, &tx, &mut mgr).await);

    assert!(stat(&mgr, MOB, HEALTH) < MOB_HEALTH, "the hit lands");
    assert_eq!(
        stat(&mgr, MOB, ACCURACY),
        0,
        "the buff never lands on the mob"
    );
    assert_eq!(stat(&mgr, A, ACCURACY), 200, "A's Accuracy (user half)");
}

/// **Regression guard (#444 never refuses a user half).** The same attack
/// aimed at ally B commits: the user half lands on the caster, B takes no
/// damage, and no #444 WARN is logged. On revert the launch refuses it
/// (`must commit` fails).
#[tokio::test]
async fn a_user_half_is_never_refused_with_its_friendly_target() {
    let mut mgr = routing_mgr();
    let b_health = stat(&mgr, B, HEALTH);
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(
        handle_use_ability(A, MIXED, B as i32, &tx, &mut mgr).await,
        "the attack at an ally must commit for its user half"
    );

    assert_eq!(stat(&mgr, A, ACCURACY), 200, "A's Accuracy (user half)");
    assert_eq!(stat(&mgr, B, HEALTH), b_health, "B takes no damage");
    assert_eq!(stat(&mgr, B, ACCURACY), 0, "B's Accuracy");
    assert!(
        logs.find_message(tracing::Level::WARN, "non-hostile target")
            .is_none(),
        "no #444 refusal: {:#?}",
        logs.all()
    );
    let launch = logs
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "effect_routing_launch"))
        .expect("the launch logs why it dropped the target");
    assert!(
        launch.has_field("resolution", "user_half_only"),
        "{launch:?}"
    );
    assert!(launch.has_field("wire_target_id", "2"), "{launch:?}");
}

/// An ordinary attack keeps #444: aimed at an ally with no user half, it is
/// still refused.
#[tokio::test]
async fn an_attack_without_a_user_half_keeps_the_444_gate() {
    let mut mgr = routing_mgr();
    mgr.effect_defs.remove(&MIXED_USER);
    mgr.ability_defs.get_mut(&MIXED).unwrap().effect_ids = vec![MIXED_HIT];
    let (tx, _rx) = mpsc::channel(256);
    assert!(!handle_use_ability(A, MIXED, B as i32, &tx, &mut mgr).await);
}

/// The `(effect_id, target_id, path, reason)` of every `effect_planned` row,
/// sorted, each checked to carry the launch row's `cast_id`.
fn plan_rows(all: &[crate::test_support::Captured]) -> Vec<(String, String, String, String)> {
    let cast_id = all
        .iter()
        .find(|c| c.has_field("event", "ability_launched"))
        .and_then(|c| c.fields.get("cast_id").cloned())
        .expect("the launch row carries cast_id");
    let mut rows: Vec<_> = all
        .iter()
        .filter(|c| c.has_field("event", "effect_planned"))
        .map(|c| {
            assert_eq!(c.target, "abilities.effect");
            assert!(c.has_field("cast_id", &cast_id), "{c:?}");
            let f = |k: &str| c.fields.get(k).cloned().unwrap_or_default();
            (f("effect_id"), f("target_id"), f("path"), f("reason"))
        })
        .collect();
    rows.sort();
    rows
}

fn plan_row(
    effect: i32,
    target: u32,
    path: &str,
    reason: &str,
) -> (String, String, String, String) {
    (
        effect.to_string(),
        target.to_string(),
        path.to_string(),
        reason.to_string(),
    )
}

/// **Regression guard (AB-T3, routed landings).** Morale Boost logs one
/// `effect_planned` row per effect per recipient, with the route it took:
/// 939 on the cast's resolved target, the caster (a beneficial cast's
/// target half; rule 2 does not apply, the ability has an area effect), and
/// the area heal on each ally in its radius (not the caster, whose 939
/// already ran `HealFocus`), all in the cast.
#[tokio::test]
async fn morale_boost_logs_one_plan_row_per_effect_per_recipient() {
    let mut mgr = routing_mgr();
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(handle_use_ability(A, MORALE_BOOST, 0, &tx, &mut mgr).await);

    assert_eq!(
        plan_rows(&logs.all()),
        vec![
            plan_row(RALLY_AREA, B, "ally_fanout", "beneficial_area"),
            plan_row(RALLY_AREA, C, "ally_fanout", "beneficial_area"),
            plan_row(RALLY_USER, A, "script", "beneficial_cast"),
        ]
    );
}

/// **Regression guard (AB-T3).** A missed attack with a user half: the hit
/// pipeline plans the damage effect `skipped` for the miss on the mob, and
/// the landing path plans the user half on the caster. One row each.
#[tokio::test]
async fn a_missed_attack_plans_the_miss_and_the_user_half() {
    let mut mgr = routing_mgr();
    next_roll(&mut mgr, MIXED, true);
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(handle_use_ability(A, MIXED, MOB as i32, &tx, &mut mgr).await);

    assert_eq!(
        plan_rows(&logs.all()),
        vec![
            plan_row(MIXED_HIT, MOB, "skipped", "miss"),
            plan_row(MIXED_USER, A, "routed_to_user", "resolve_on_ability_user"),
        ]
    );
}
