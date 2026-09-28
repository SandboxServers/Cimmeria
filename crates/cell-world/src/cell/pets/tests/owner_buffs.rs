//! PT-08: "the owner's pet" resolution, the pet buff ledger, the pet effect
//! scripts and the passive-effect pass.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_entity::abilities::{AbilityDef, EffectDef, AF_TOGGLED, EF_ALWAYS_PERSIST};
use cimmeria_entity::stats::{ACCURACY, DEFENSE, HEALTH, INTERRUPT_RES, SPEED_PET};
use cimmeria_wire::state_field::BSF_DEAD;

use super::*;
use crate::cell::effects::passives::{apply_passives, PassiveChange};
use crate::cell::effects::{dispatch_by_name, EffectContext};
use crate::cell::pets::{BuffRemoval, OwnerPetRefusal};

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

// ── owner_pet_targets ─────────────────────────────────────────────────────

#[test]
fn the_owners_live_pet_resolves() {
    let (mgr, pet) = world_with_pet();
    assert_eq!(mgr.owner_pet_targets(OWNER), Ok(vec![pet]));
}

#[test]
fn no_pet_dead_pet_and_elsewhere_are_refused_with_their_reason() {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    assert_eq!(mgr.owner_pet_targets(OWNER), Err(OwnerPetRefusal::NoPet));

    let (mut mgr, pet) = world_with_pet();
    mgr.get_entity_mut(pet).unwrap().state_field |= BSF_DEAD;
    assert_eq!(mgr.owner_pet_targets(OWNER), Err(OwnerPetRefusal::PetDead));
    assert_eq!(OwnerPetRefusal::PetDead.error_code(), 14);

    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    let stray = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 1643)
        .unwrap();
    // Move the owner out; the pet stays in Agnos.
    mgr.destroy_entity(OWNER);
    add_pet_owner(&mut mgr, OWNER, "Castle", [10.0, 0.0, 10.0], 12);
    assert!(mgr.get_entity(stray).is_some());
    assert_eq!(
        mgr.owner_pet_targets(OWNER),
        Err(OwnerPetRefusal::PetOtherSpace)
    );
}

/// **Regression guard.** The owner's entity id now belongs to another
/// player: the pet the earlier holder summoned is not theirs to act on.
/// Fails when the summoner check is removed.
#[test]
fn a_reused_owner_id_does_not_resolve_the_earlier_pet() {
    let (mut mgr, _pet) = world_with_pet();
    reuse_owner_id_by_another_player(&mut mgr);
    assert_eq!(
        mgr.owner_pet_targets(OWNER),
        Err(OwnerPetRefusal::OwnerIdentityMismatch)
    );
}

/// The id's new holder summons its own pet: that one resolves, the earlier
/// holder's does not.
#[test]
fn after_reuse_only_the_new_holders_pet_resolves() {
    let (mut mgr, old_pet) = world_with_pet();
    let new_pet = reuse_owner_id_then_resummon(&mut mgr);
    let resolved = mgr.owner_pet_targets(OWNER).unwrap();
    assert_eq!(resolved, vec![new_pet]);
    assert!(!resolved.contains(&old_pet));
}

// ── The buff ledger ───────────────────────────────────────────────────────

#[test]
fn a_buff_widens_a_zero_bound_and_its_removal_restores_the_value() {
    let (mut mgr, pet) = world_with_pet();
    let applied = mgr
        .apply_pet_buff(pet, 1, 2, &[(DEFENSE, -100), (INTERRUPT_RES, 50)], None)
        .unwrap();
    assert_eq!(applied, vec![(DEFENSE, -100), (INTERRUPT_RES, 50)]);
    assert_eq!(stat(&mgr, pet, DEFENSE), -100);
    assert_eq!(stat(&mgr, pet, INTERRUPT_RES), 50);
    assert!(mgr.has_pet_buff(pet, 1));

    let removed = mgr.remove_pet_buff(pet, 1, BuffRemoval::Expired).unwrap();
    assert_eq!(removed.stat_deltas, applied);
    assert_eq!(stat(&mgr, pet, DEFENSE), 0);
    assert_eq!(stat(&mgr, pet, INTERRUPT_RES), 0);
    assert!(!mgr.has_pet_buff(pet, 1));
    assert!(mgr.remove_pet_buff(pet, 1, BuffRemoval::Expired).is_none());
}

#[test]
fn re_applying_the_same_effect_refreshes_instead_of_stacking() {
    let (mut mgr, pet) = world_with_pet();
    let now = Instant::now();
    mgr.apply_pet_buff(pet, 1, 2, &[(ACCURACY, 400)], Some(now));
    mgr.apply_pet_buff(
        pet,
        1,
        2,
        &[(ACCURACY, 400)],
        Some(now + Duration::from_secs(60)),
    );
    assert_eq!(stat(&mgr, pet, ACCURACY), 400);
    assert_eq!(mgr.expired_pet_buffs(now + Duration::from_secs(1)), vec![]);
    assert_eq!(
        mgr.expired_pet_buffs(now + Duration::from_secs(61)),
        vec![(pet, 1)]
    );
}

#[test]
fn a_non_pet_takes_no_buff() {
    let (mut mgr, _pet) = world_with_pet();
    assert!(mgr
        .apply_pet_buff(OWNER, 1, 2, &[(ACCURACY, 400)], None)
        .is_none());
    assert_eq!(stat(&mgr, OWNER, ACCURACY), 0);
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
