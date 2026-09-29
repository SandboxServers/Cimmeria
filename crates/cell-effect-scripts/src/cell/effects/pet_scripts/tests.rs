//! PT-08: the pet effect scripts and the passive-effect pass, moved from
//! `cimmeria-cell-world`'s `pets::tests::owner_buffs` with the scripts (the
//! owner-pet resolution and buff-ledger tests stayed there).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_entity::abilities::{AbilityDef, EffectDef, AF_TOGGLED, EF_ALWAYS_PERSIST};
use cimmeria_entity::stats::{ACCURACY, DEFENSE, HEALTH, SPEED_PET};

use crate::cell::effects::passives::{apply_passives, PassiveChange};
use crate::cell::effects::{dispatch_by_name, EffectContext};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{
    add_pet_owner, make_pet_world, PET_FIXTURE_OWNER, PET_FIXTURE_TEMPLATE_ID,
};

/// Owner entity id used throughout.
const OWNER: u32 = PET_FIXTURE_OWNER;

/// Agnos, Castle and Castle_CellBlock with the pet template cached, and the
/// effect scripts registered as the cell registers them at startup.
fn make_world() -> SpaceManager {
    let mut mgr = make_pet_world();
    super::super::registry::install(&mut mgr);
    mgr
}

/// A world with `OWNER` (level 12) in Agnos and one pet summoned for it.
fn world_with_pet() -> (SpaceManager, u32) {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 1643)
        .expect("pet spawns");
    (mgr, pet)
}

fn stat(mgr: &SpaceManager, e: u32, id: i32) -> i32 {
    mgr.get_entity(e).unwrap().stats.get(id).unwrap().cur
}

fn params(nvps: &[(&str, &str)]) -> HashMap<String, String> {
    nvps.iter()
        .map(|&(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn ability(id: i32, flags: u32, effects: &[i32]) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: format!("ability {id}"),
        cooldown: 0.0,
        warmup: 0.0,
        flags,
        is_ranged: false,
        min_range: 0.0,
        max_range: 0.0,
        target_type_id: 1,
        effect_ids: effects.to_vec(),
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
    }
}

fn run(mgr: &mut SpaceManager, source: u32, target: u32, effect: &EffectDef) {
    let script = effect.script_name.clone().unwrap();
    let mut ctx = EffectContext {
        source_id: source,
        target_id: target,
        effect,
        space_mgr: mgr,
    };
    assert!(
        dispatch_by_name(&script, &mut ctx),
        "{script} is registered"
    );
}

// ── The scripts ───────────────────────────────────────────────────────────

/// **Regression guard.** A Toggled ability's `PetStatBuff` switches: on,
/// then off. Fails when the toggle branch is removed (the second run
/// refreshes, leaving the buff on).
#[test]
fn pet_stat_buff_toggles_for_a_toggled_ability() {
    let (mut mgr, pet) = world_with_pet();
    mgr.ability_defs
        .insert(2824, ability(2824, 1560 | AF_TOGGLED, &[4220]));
    let effect = EffectDef {
        effect_id: 4220,
        ability_id: 2824,
        script_name: Some("PetStatBuff".to_string()),
        params: params(&[("Accuracy", "100"), ("Defense", "-100")]),
        ..Default::default()
    };
    run(&mut mgr, OWNER, pet, &effect);
    assert_eq!(stat(&mgr, pet, ACCURACY), 100);
    run(&mut mgr, OWNER, pet, &effect);
    assert_eq!(stat(&mgr, pet, ACCURACY), 0);
    assert_eq!(stat(&mgr, pet, DEFENSE), 0);
}

#[test]
fn pet_stat_buff_is_timed_by_its_pulse_duration() {
    let (mut mgr, pet) = world_with_pet();
    mgr.ability_defs.insert(2839, ability(2839, 16, &[4121]));
    let effect = EffectDef {
        effect_id: 4121,
        ability_id: 2839,
        script_name: Some("PetStatBuff".to_string()),
        pulse_duration: 60.0,
        params: params(&[("Accuracy", "400")]),
        ..Default::default()
    };
    run(&mut mgr, OWNER, pet, &effect);
    let later = Instant::now() + Duration::from_secs(61);
    assert_eq!(mgr.expired_pet_buffs(later), vec![(pet, 4121)]);
}

/// The redirect only hands these scripts a pet; anything else is a seed
/// defect, logged and left alone.
#[test]
fn pet_scripts_leave_a_non_pet_alone() {
    let (mut mgr, _pet) = world_with_pet();
    let effect = EffectDef {
        effect_id: 3211,
        ability_id: 967,
        script_name: Some("HealPetHealth".to_string()),
        params: params(&[("HealPercentage", "50")]),
        ..Default::default()
    };
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, 10, 100);
    run(&mut mgr, OWNER, OWNER, &effect);
    assert_eq!(stat(&mgr, OWNER, HEALTH), 10, "the caster is not healed");
}

#[test]
fn pet_death_timer_dooms_the_pet() {
    let (mut mgr, pet) = world_with_pet();
    let effect = EffectDef {
        effect_id: 4119,
        ability_id: 2839,
        script_name: Some("PetDeathTimer".to_string()),
        pulse_duration: 60.0,
        ..Default::default()
    };
    run(&mut mgr, OWNER, pet, &effect);
    let now = Instant::now();
    assert!(mgr.doomed_pets_due(now).is_empty());
    assert_eq!(
        mgr.doomed_pets_due(now + Duration::from_secs(61)),
        vec![pet]
    );
    assert!(mgr.any_pet_buff_or_doom());
}

// ── Passives ──────────────────────────────────────────────────────────────

fn passive_world(flags: u32, script: &str) -> SpaceManager {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    mgr.ability_defs.insert(2852, ability(2852, 0, &[4968]));
    mgr.effect_defs.insert(
        4968,
        EffectDef {
            effect_id: 4968,
            ability_id: 2852,
            flags,
            script_name: Some(script.to_string()),
            params: params(&[("SpeedPet", "100"), ("HealPercentage", "50")]),
            ..Default::default()
        },
    );
    mgr
}

/// **Regression guard (D-PT10).** Learning Heed Our Calling raises the
/// owner's `speedPet` from its `[0, 0]` default to 100; unlearning puts it
/// back. Fails when the passive pass or the script is removed.
#[test]
fn a_learned_passive_raises_speed_pet_and_unlearning_restores_it() {
    let mut mgr = passive_world(EF_ALWAYS_PERSIST, "PetSummonSpeed");
    assert_eq!(
        apply_passives(&mut mgr, OWNER, &[2852], PassiveChange::Learned),
        1
    );
    assert_eq!(stat(&mgr, OWNER, SPEED_PET), 100);
    let s = mgr.get_entity(OWNER).unwrap().stats.get(SPEED_PET).unwrap();
    assert!(!s.dirty, "server-side only: never rides a dirty-stat flush");
    assert_eq!(
        apply_passives(&mut mgr, OWNER, &[2852], PassiveChange::Unlearned),
        1
    );
    assert_eq!(stat(&mgr, OWNER, SPEED_PET), 0);
}

/// Only `EF_AlwaysPersist` effects with a passive script run: a missing flag,
/// or an always-persist row carrying a heal, runs nothing at login.
#[test]
fn the_passive_pass_runs_only_flagged_passive_scripts() {
    let mut mgr = passive_world(0, "PetSummonSpeed");
    assert_eq!(
        apply_passives(&mut mgr, OWNER, &[2852], PassiveChange::Learned),
        0
    );
    assert_eq!(stat(&mgr, OWNER, SPEED_PET), 0);

    let mut mgr = passive_world(EF_ALWAYS_PERSIST, "HealHealth");
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, 10, 100);
    assert_eq!(
        apply_passives(&mut mgr, OWNER, &[2852], PassiveChange::Learned),
        0
    );
    assert_eq!(stat(&mgr, OWNER, HEALTH), 10);
}
