//! The per-effect resolver, rule by rule, and the secondary scopes. Effect
//! shapes copy the seed rows they are named after.

use std::collections::HashMap;

use cimmeria_entity::abilities::{
    AbilityDef, AbilityType, EffectDef, EF_RESOLVE_ON_ABILITY_USER, TARGET_GROUND, TARGET_SELF,
    TARGET_TARGET, TCM_AE_CONE, TCM_AE_RADIUS, TCM_SINGLE,
};

use super::*;

fn effect(
    id: i32,
    tcm: &str,
    flags: u32,
    script: Option<&str>,
    params: &[(&str, &str)],
) -> EffectDef {
    EffectDef {
        effect_id: id,
        target_collection_method: tcm.to_string(),
        flags,
        script_name: script.map(str::to_string),
        params: params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}

fn shape(self_ability: bool, has_area_effect: bool) -> CastShape {
    CastShape {
        self_ability,
        has_area_effect,
        player_caster: true,
        ..CastShape::default()
    }
}

const DAMAGE: &[(&str, &str)] = &[("FocusDamage", "200"), ("HealthDamage", "20")];

/// Rule 1: `EF_ResolveOnAbilityUser` lands on the user on a hostile cast,
/// whatever the ability's target type (Disguise 1229: flags 131093).
#[test]
fn resolve_on_ability_user_lands_on_the_user_of_a_hostile_cast() {
    let e = effect(1229, TCM_SINGLE, 131_093, Some("TimedStat"), &[]);
    for s in [shape(false, false), shape(true, true), shape(false, true)] {
        assert_eq!(
            route_effect(&e, s),
            (EffectRoute::User, REASON_RESOLVE_ON_USER)
        );
    }
}

/// Damage never routes onto the user: a user-flagged damage row stays on
/// the target.
#[test]
fn damage_never_routes_onto_the_user() {
    let e = effect(4531, TCM_AE_CONE, EF_RESOLVE_ON_ABILITY_USER, None, DAMAGE);
    assert_eq!(
        route_effect(&e, shape(false, true)),
        (EffectRoute::Target, REASON_DAMAGE_STAYS_ON_TARGET)
    );
    let script = effect(
        1,
        TCM_SINGLE,
        EF_RESOLVE_ON_ABILITY_USER,
        Some("RangedPhysicalDamage"),
        &[],
    );
    assert_eq!(
        route_effect(&script, shape(false, false)).0,
        EffectRoute::Target
    );
    // Rule 2 too: Consume Energy's single-target damage on a Self ability.
    let self_damage = effect(4648, TCM_SINGLE, 0, None, DAMAGE);
    assert_eq!(
        route_effect(&self_damage, shape(true, false)),
        (EffectRoute::Target, REASON_DAMAGE_STAYS_ON_TARGET)
    );
}

/// Rule 2: Combat Sprint (1619, Self, no area effect). Both halves land on
/// the user: 1962 "User +50% Run Speed" and 2002 "Target -100 ACC" (flags
/// 534, no beneficial bit; the tooltip says "Decreases Accuracy by 100").
#[test]
fn a_pure_self_abilitys_single_effects_land_on_the_user() {
    let run = effect(
        1962,
        TCM_SINGLE,
        23,
        Some("TimedStat"),
        &[("MovementSpeedMod", "50")],
    );
    let acc = effect(
        2002,
        TCM_SINGLE,
        534,
        Some("TimedStat"),
        &[("Accuracy", "-100")],
    );
    for e in [&run, &acc] {
        assert_eq!(
            route_effect(e, shape(true, false)),
            (EffectRoute::User, REASON_SELF_ABILITY)
        );
    }
    // The same rows on a Target ability keep their target.
    assert_eq!(
        route_effect(&acc, shape(false, false)),
        (EffectRoute::Target, REASON_TARGET)
    );
}

/// Rule 2 stops at a Self ability with an area effect: Whirlwind's
/// knockdown (2669) is a follow-up of the area hit, not a self-stun.
#[test]
fn a_self_ability_with_an_area_effect_keeps_its_singles_on_the_target() {
    let knockdown = effect(2669, TCM_SINGLE, 64, Some("Stun"), &[]);
    assert_eq!(
        route_effect(&knockdown, shape(true, true)),
        (EffectRoute::Target, REASON_TARGET)
    );
}

/// Rule 3: Morale Boost's 1215 (flags 16, `HealFocus`) fans out to allies
/// on a player's non-ground cast; so does a beneficial-bit area buff on a
/// hostile cast. Not on a ground cast, not for an NPC caster, and not a
/// hostile area effect.
#[test]
fn a_beneficial_area_effect_fans_out_to_allies() {
    let heal = effect(
        1215,
        TCM_AE_RADIUS,
        16,
        Some("HealFocus"),
        &[("HealPercentage", "35")],
    );
    let buff = effect(921, TCM_AE_RADIUS, 21, Some("TimedStat"), &[]);
    let hostile = effect(2667, TCM_AE_RADIUS, 0, None, DAMAGE);
    let target = shape(false, true);
    assert_eq!(
        route_effect(&heal, target),
        (EffectRoute::AllyArea, REASON_BENEFICIAL_AREA)
    );
    assert_eq!(route_effect(&buff, target).0, EffectRoute::AllyArea);
    assert_eq!(
        route_effect(&hostile, target),
        (EffectRoute::Target, REASON_TARGET)
    );
    // A beneficial cast makes every radius effect an ally fan-out.
    let unscripted = effect(9, TCM_AE_RADIUS, 0, Some("PetStatBuff"), &[]);
    assert_eq!(route_effect(&unscripted, target).0, EffectRoute::Target);
    let beneficial = CastShape {
        beneficial_cast: true,
        ..target
    };
    assert_eq!(
        route_effect(&unscripted, beneficial).0,
        EffectRoute::AllyArea
    );
    // Ground and NPC casts keep today's collectors.
    let ground = CastShape {
        ground: true,
        ..target
    };
    assert_eq!(route_effect(&heal, ground).0, EffectRoute::Target);
    let npc = CastShape {
        player_caster: false,
        ..target
    };
    assert_eq!(route_effect(&heal, npc).0, EffectRoute::Target);
}

/// Rule 4: `TCM_Group` and `TCM_Aura` wait for D-AB12 and keep the target.
#[test]
fn group_and_aura_effects_keep_the_target_until_d_ab12() {
    for tcm in [TCM_GROUP, TCM_AURA] {
        let e = effect(350, tcm, 21, Some("PetStatBuff"), &[]);
        assert_eq!(
            route_effect(&e, shape(false, false)),
            (EffectRoute::Target, REASON_GROUP_AURA_PENDING)
        );
    }
}

fn ability(id: i32, target_type_id: i32, effect_ids: Vec<i32>) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: "fixture".to_string(),
        cooldown: 1.0,
        warmup: 0.0,
        flags: 0,
        is_ranged: false,
        min_range: 0.0,
        max_range: 0.0,
        target_type_id,
        effect_ids,
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 100.0,
        type_id: AbilityType::DirectDamage,
        passive: false,
    }
}

/// A manager with only `effects` loaded: no caster entity, so the shape is
/// an NPC's (no ally fan-out), which these scope tests do not need.
fn mgr_with(effects: Vec<EffectDef>) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    for e in effects {
        mgr.effect_defs.insert(e.effect_id, e);
    }
    mgr
}

/// **Regression guard (AB-03 carry-over).** 3170 Contaminate Area: a ground
/// secondary takes the radius damage 4729 (and the pulse 4730 and the VFX
/// 4731), never the primary's single-target 4728. On revert (the whole def
/// for every secondary) the scope still holds 4728.
#[test]
fn a_ground_secondary_takes_the_area_part_not_the_primarys_hit() {
    let mgr = mgr_with(vec![
        effect(4728, TCM_SINGLE, 128, None, DAMAGE),
        effect(4729, TCM_AE_RADIUS, 0, None, DAMAGE),
        effect(4730, TCM_AE_RADIUS, 0, None, &[]),
        effect(4731, TCM_SINGLE, 0, None, &[]),
    ]);
    let def = Some(ability(3170, TARGET_GROUND, vec![4728, 4729, 4730, 4731]));
    let scoped = secondary_scope(&mgr, 1, &def).unwrap();
    assert_eq!(scoped.effect_ids, vec![4729, 4730, 4731]);
}

/// Devastating Blast (2058): the DoT 2720 is the primary's; Flashbang's
/// debuff (937, a `TCM_Single` "Small Radius AE" row) reaches every target.
#[test]
fn a_ground_secondary_keeps_area_debuffs_and_drops_the_primarys_dot() {
    let mut dot = effect(2720, TCM_SINGLE, 70, None, DAMAGE);
    dot.pulse_count = 10;
    dot.pulse_duration = 1.0;
    let mgr = mgr_with(vec![
        effect(2719, TCM_AE_RADIUS, 4_194_304, None, DAMAGE),
        dot,
        effect(
            937,
            TCM_SINGLE,
            4_194_372,
            Some("TimedStat"),
            &[("Accuracy", "-200")],
        ),
    ]);
    let def = Some(ability(2058, TARGET_GROUND, vec![2719, 2720, 937]));
    assert_eq!(
        secondary_scope(&mgr, 1, &def).unwrap().effect_ids,
        vec![2719, 937]
    );
}

/// A ground ability whose damage is authored only on single-target rows
/// (3178's 4772) keeps the old behaviour: the secondaries take it.
#[test]
fn a_ground_ability_with_no_area_damage_keeps_the_whole_target_part() {
    let mgr = mgr_with(vec![
        effect(4772, TCM_SINGLE, 128, None, &[("FocusDamage", "200")]),
        effect(4774, TCM_AE_RADIUS, 0, None, &[]),
    ]);
    let def = Some(ability(3178, TARGET_GROUND, vec![4772, 4774]));
    assert_eq!(
        secondary_scope(&mgr, 1, &def).unwrap().effect_ids,
        vec![4772, 4774]
    );
}

/// A splash target takes the shot, not its DoT.
#[test]
fn a_splash_takes_the_shot_but_not_its_dot() {
    let mut dot = effect(2, TCM_SINGLE, 0, None, DAMAGE);
    dot.pulse_count = 8;
    dot.pulse_duration = 1.0;
    let defs = HashMap::from([(1, effect(1, TCM_SINGLE, 0, None, DAMAGE)), (2, dot)]);
    let def = Some(ability(579, TARGET_TARGET, vec![1, 2]));
    assert_eq!(splash_scope(&defs, &def).unwrap().effect_ids, vec![1]);
    assert!(splash_scope(&defs, &None).is_none());
}

/// `plan_cast` on Combat Sprint: both halves land on the caster and the
/// target pipeline has nothing left.
#[test]
fn plan_cast_takes_a_pure_self_ability_off_its_target() {
    let mgr = mgr_with(vec![
        effect(
            1962,
            TCM_SINGLE,
            23,
            Some("TimedStat"),
            &[("MovementSpeedMod", "50")],
        ),
        effect(
            2002,
            TCM_SINGLE,
            534,
            Some("TimedStat"),
            &[("Accuracy", "-100")],
        ),
    ]);
    let def = ability(1619, TARGET_SELF, vec![1962, 2002]);
    let routed = plan_cast(&mgr, 1, Some(&def), None);
    assert_eq!(routed.moved, 2);
    assert!(routed.target_has_nothing());
    assert!(routed.lands_on(1));
    assert_eq!(
        routed
            .landings
            .iter()
            .map(|l| (l.effect.effect_id, l.recipient))
            .collect::<Vec<_>>(),
        vec![(1962, 1), (2002, 1)]
    );
}

/// An ordinary attack routes nothing: the target pipeline gets the whole
/// ability, so every weapon shot behaves exactly as before AB-07.
#[test]
fn plan_cast_leaves_an_ordinary_attack_alone() {
    let mgr = mgr_with(vec![effect(641, TCM_SINGLE, 0, None, DAMAGE)]);
    let def = ability(579, TARGET_TARGET, vec![641]);
    let routed = plan_cast(&mgr, 1, Some(&def), None);
    assert_eq!(routed.moved, 0);
    assert!(routed.landings.is_empty());
    assert!(!routed.target_has_nothing());
    assert_eq!(routed.target_def.unwrap().effect_ids, vec![641]);
}
