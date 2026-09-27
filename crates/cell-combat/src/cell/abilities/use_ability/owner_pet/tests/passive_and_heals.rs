//! 1650 Lord's Concentration, the pet heals (967 / 968) and 2852 Heed Our
//! Calling's passive.

use std::time::{Duration, Instant};

use cimmeria_cell_world::cell::effects::passives::{apply_passives, PassiveChange};
use cimmeria_entity::abilities::{AbilityDef, EffectDef, AF_SPEED_PET, EF_ALWAYS_PERSIST};
use cimmeria_entity::stats::{HEALTH, INTERRUPT_RES, SPEED_PET};
use tokio::sync::mpsc;

use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::cell::abilities::use_ability::handle_use_ability;
use crate::cell::abilities::use_ability::owner_pet::owner_pet_tick_at;
use crate::test_support::NoContentEvents;

/// Lord's Concentration (D-PT17): after its 2 s warmup every pet the owner
/// has out gets +50 Interrupt Resistance (whose `[0, 0]` bound widens), and
/// the buff lapses 30 s later. Fails when effect 350's script or the redirect
/// is removed.
#[tokio::test]
async fn lords_concentration_gives_the_pets_interrupt_resistance_for_30_seconds() {
    let (mut mgr, pet) = world();
    let (tx, mut rx) = mpsc::channel(256);
    let cast_at = Instant::now();

    assert!(handle_use_ability(OWNER, LORDS_CONCENTRATION, 0, &tx, &mut mgr).await);
    assert_eq!(
        stat(&mgr, pet, INTERRUPT_RES),
        0,
        "nothing before the warmup"
    );
    assert_eq!(
        resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await,
        1
    );
    assert_eq!(stat(&mgr, pet, INTERRUPT_RES), 50);
    let _ = drain(&mut rx);

    let _ = owner_pet_tick_at(cast_at + Duration::from_secs(29), &tx, &mut mgr).await;
    assert_eq!(stat(&mgr, pet, INTERRUPT_RES), 50);
    let _ = owner_pet_tick_at(cast_at + Duration::from_secs(33), &tx, &mut mgr).await;
    assert_eq!(stat(&mgr, pet, INTERRUPT_RES), 0, "the buff lapses");
    assert!(
        stat_update_to(&drain(&mut rx), OWNER, pet),
        "the lapse reaches the owner"
    );
}

fn wound(mgr: &mut SpaceManager, pet: u32) -> (i32, i32) {
    let h = mgr
        .get_entity_mut(pet)
        .unwrap()
        .stats
        .get_mut(HEALTH)
        .unwrap();
    h.update(0, 10, 1000);
    (h.cur, h.max)
}

/// **Regression guard.** Repair Turret: Percentage heals the owner's pet by
/// 20% of its max, whatever the client aimed at (here the other player,
/// whom #444 would refuse). Fails when 3211 loses `HealPetHealth` or the
/// redirect is removed.
#[tokio::test]
async fn repair_heals_the_owners_pet_not_the_target() {
    let (mut mgr, pet) = world();
    let (tx, mut rx) = mpsc::channel(256);
    let (_, max) = wound(&mut mgr, pet);
    let other_health = stat(&mgr, OTHER, HEALTH);

    assert!(handle_use_ability(OWNER, REPAIR_PERCENT, OTHER as i32, &tx, &mut mgr).await);
    assert_eq!(
        stat(&mgr, pet, HEALTH),
        10,
        "the heal waits for the 2 s warmup"
    );
    let _ = resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    assert_eq!(stat(&mgr, pet, HEALTH), 10 + max / 5);
    assert_eq!(stat(&mgr, OTHER, HEALTH), other_health);
    assert!(stat_update_to(&drain(&mut rx), OWNER, pet));
}

/// Repair Turret: Regenerate is a heal over time on the pet: the first 5%
/// lands at once and the pulsing tick holds the other 14 pulses, invoked by
/// the owner.
#[tokio::test]
async fn repair_regenerate_runs_a_heal_over_time_on_the_pet() {
    let (mut mgr, pet) = world();
    let (tx, _rx) = mpsc::channel(256);
    let (_, max) = wound(&mut mgr, pet);

    assert!(handle_use_ability(OWNER, REPAIR_REGEN, 0, &tx, &mut mgr).await);
    let _ = resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    assert_eq!(stat(&mgr, pet, HEALTH), 10 + max / 20);
    let hot = mgr
        .get_entity(pet)
        .unwrap()
        .active_effects
        .iter()
        .find(|a| a.effect_id == E_HEAL_PET_REGEN)
        .cloned()
        .expect("the HoT is registered on the pet");
    assert_eq!(hot.invoker_id, OWNER);
    assert_eq!(hot.remaining_pulses, 14);
}

/// 2852 Heed Our Calling and 4968, as seeded.
fn heed_our_calling(mgr: &mut SpaceManager) {
    mgr.ability_defs.insert(
        2852,
        AbilityDef {
            ability_id: 2852,
            name: "Heed Our Calling".to_string(),
            cooldown: 0.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 1,
            effect_ids: vec![4968],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 100.0,
        },
    );
    mgr.effect_defs.insert(
        4968,
        EffectDef {
            effect_id: 4968,
            ability_id: 2852,
            flags: EF_ALWAYS_PERSIST,
            script_name: Some("PetSummonSpeed".to_string()),
            params: [("SpeedPet".to_string(), "100".to_string())].into(),
            ..Default::default()
        },
    );
}

/// **D-PT10 end to end.** Knowing Heed Our Calling raises the owner's
/// `speedPet` to 100, and a `SpeedPet` summon's warmup (6 s for 2826)
/// becomes zero: the next summon is instant. Unlearning it puts the warmup
/// back. Fails when 4968 loses its script or the passive is not applied.
#[tokio::test]
async fn heed_our_calling_makes_the_summon_instant() {
    let (mut mgr, _pet) = world();
    heed_our_calling(&mut mgr);
    let summon = AbilityDef {
        ability_id: 2826,
        name: "Summon Straegis".to_string(),
        cooldown: 5.0,
        warmup: 6.0,
        flags: 18192,
        is_ranged: false,
        min_range: 0,
        max_range: 0,
        target_type_id: 1,
        effect_ids: vec![],
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: Some(1121),
        velocity: 100.0,
    };
    assert_ne!(summon.flags & AF_SPEED_PET, 0);
    let warmup = |mgr: &SpaceManager| {
        super::super::super::warmup::effective_warmup(Some(&summon), mgr.get_entity(OWNER).unwrap())
    };
    assert!((warmup(&mgr) - 6.0).abs() < 1e-6);

    assert_eq!(
        apply_passives(&mut mgr, OWNER, &[2852], PassiveChange::Learned),
        1
    );
    assert_eq!(stat(&mgr, OWNER, SPEED_PET), 100);
    assert_eq!(warmup(&mgr), 0.0, "the summon is instant");

    // A repeated grant does not stack.
    let _ = apply_passives(&mut mgr, OWNER, &[2852], PassiveChange::Learned);
    assert_eq!(stat(&mgr, OWNER, SPEED_PET), 100);

    let _ = apply_passives(&mut mgr, OWNER, &[2852], PassiveChange::Unlearned);
    assert_eq!(stat(&mgr, OWNER, SPEED_PET), 0);
    assert!((warmup(&mgr) - 6.0).abs() < 1e-6);
}
