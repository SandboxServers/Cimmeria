//! SS-D2: the four hostility gates admit an engaged duel partner, and only
//! them (audit A-42, § 6 "duel harm gate").
//!
//! The gates are the single-target launch (`handle_use_ability`), the
//! warmup re-check at fire (`warmup::tick`), the ground-AoE collector
//! (`dispatch::collect_ground_targets`) and the cone collector
//! (`cone_aoe::collect_cone_targets`). Each section below drives one gate on
//! its own, so reverting any one of them to the old PvE-only rule fails that
//! section:
//!
//! - gate 1 reverted: the launch at the partner is refused;
//! - gate 2 reverted: the warmup ends in `TargetLost` and nothing lands;
//! - gate 3 reverted: the partner, a secondary of an AoE whose primary is
//!   a hostile NPC, takes no damage (this section does not depend on gate 1);
//! - gate 4 reverted: the cone does not collect the partner.
//!
//! Player 1 (A) and player 2 (B) are the engaged pair, player 3 (C) is a
//! bystander, NPC 4 is a hostile mob used as the AoE and cone primary.

use std::time::Instant;

use cimmeria_common::Vector3;
use cimmeria_entity::stats::HEALTH;

use super::warmup::{after_warmup, warmup_mgr, INSTANT_ABILITY, WARMUP_ABILITY};
use super::*;
use crate::cell::abilities::{collect_cone_targets, handle_use_ability_on_ground, resolve_warmups};
use crate::test_support::NoContentEvents;

const A: u32 = 1;
const B: u32 = 2;
const C: u32 = 3;
const MOB: u32 = 4;
const A_PID: i32 = 101;
const B_PID: i32 = 102;
const C_PID: i32 = 103;
const FULL: i32 = 100_000;

/// `warmup_mgr`'s player 1, with entity 2 turned into player B at (3,0,0),
/// bystander C at (0,0,3) and hostile mob 4 at (2,0,0). Every entity has
/// plenty of health; A's weapon is drawn so no holster queue intercepts.
fn duel_mgr() -> SpaceManager {
    let mut mgr = warmup_mgr();
    mgr.create_entity(C, "Castle", [0.0, 0.0, 3.0], [0.0; 3])
        .unwrap();
    mgr.spawn_npc(MOB, "Castle", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    for (eid, pid) in [(B, Some(B_PID)), (C, Some(C_PID)), (MOB, None)] {
        let e = mgr.get_entity_mut(eid).unwrap();
        e.is_player = pid.is_some();
        e.player_id = pid;
        e.faction = if pid.is_some() {
            0
        } else {
            crate::cell::combat::HOSTILE_FACTION
        };
        let hp = e.stats.get_mut(HEALTH).unwrap();
        hp.update(0, FULL, FULL);
        hp.clear_dirty();
    }
    mgr.get_entity_mut(A).unwrap().weapon_holstered = false;
    mgr.connect_entity(B);
    mgr.connect_entity(C);
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// Put A and B in an engaged duel, straight through the registry.
fn engage(mgr: &mut SpaceManager) {
    let now = Instant::now();
    mgr.duels.open_challenge(A_PID, B_PID, now).unwrap();
    let space = mgr.get_entity_space_id(A).expect("A has a space");
    let p = mgr.duels.take_pending_for(B_PID, now).unwrap();
    let duel = mgr
        .duels
        .start_duel(&p, space, Vector3::new(1.5, 0.0, 0.0), now);
    mgr.duels.engage(duel.duel_id, [A, B], now).unwrap();
    assert!(mgr.duels.can_harm(A_PID, B_PID) && mgr.duels.can_harm(B_PID, A_PID));
}

fn health(mgr: &SpaceManager, eid: u32) -> i32 {
    mgr.get_entity(eid).unwrap().stats.get(HEALTH).unwrap().cur
}

fn heal(mgr: &mut SpaceManager, eid: u32) {
    let hp = mgr
        .get_entity_mut(eid)
        .unwrap()
        .stats
        .get_mut(HEALTH)
        .unwrap();
    hp.update(0, FULL, FULL);
}

/// Every gate lets A hit their engaged partner B.
#[tokio::test]
async fn duel_partner_damage_allowed_at_all_four_gates() {
    let mut mgr = duel_mgr();
    engage(&mut mgr);
    let (tx, mut rx) = mpsc::channel(1024);

    // Gate 1: the single-target launch commits and B takes the hit.
    assert!(
        handle_use_ability(A, INSTANT_ABILITY, B as i32, &tx, &mut mgr).await,
        "gate 1: the launch at the duel partner was refused"
    );
    assert!(health(&mgr, B) < FULL, "gate 1: the partner took no damage");
    drain(&mut rx);
    heal(&mut mgr, B);

    // Gate 2: a warmup cast at B survives the fire-time re-check and lands.
    assert!(handle_use_ability(A, WARMUP_ABILITY, B as i32, &tx, &mut mgr).await);
    assert_eq!(
        health(&mgr, B),
        FULL,
        "nothing lands before the warmup ends"
    );
    resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    assert!(
        health(&mgr, B) < FULL,
        "gate 2: the warmup re-check refused the duel partner"
    );
    drain(&mut rx);
    heal(&mut mgr, B);
    heal(&mut mgr, MOB);

    // Gate 3: a ground AoE on the mob (the primary) also hits B, one unit
    // behind it, as a secondary.
    mgr.get_entity_mut(A)
        .unwrap()
        .abilities
        .clear_all_cooldowns();
    handle_use_ability_on_ground(A, INSTANT_ABILITY, [2.0, 0.0, 0.0], &tx, &mut mgr).await;
    assert!(health(&mgr, MOB) < FULL, "the AoE primary was hit");
    assert!(
        health(&mgr, B) < FULL,
        "gate 3: the ground AoE did not collect the duel partner"
    );
    drain(&mut rx);

    // Gate 4: a cone from A toward the mob collects B behind it.
    let hits = collect_cone_targets(&mgr, A, MOB, 10.0, std::f32::consts::FRAC_PI_4);
    assert!(
        hits.contains(&B),
        "gate 4: the cone did not collect the duel partner: {hits:?}"
    );
}

/// While A and B duel, bystander C stays untouchable at every gate, and a
/// duel that ends mid-warmup stops the cast at fire.
#[tokio::test]
async fn bystander_untouchable_during_duel() {
    let mut mgr = duel_mgr();
    // C stands where the AoE and the cone reach, B does not.
    mgr.get_entity_mut(C).unwrap().position = Vector3::new(3.0, 0.0, 0.5);
    mgr.get_entity_mut(B).unwrap().position = Vector3::new(-20.0, 0.0, 0.0);
    engage(&mut mgr);
    let (tx, mut rx) = mpsc::channel(1024);

    // Gate 1: the launch at C is refused, no cooldown, no damage.
    assert!(!handle_use_ability(A, INSTANT_ABILITY, C as i32, &tx, &mut mgr).await);
    assert!(!mgr
        .get_entity(A)
        .unwrap()
        .abilities
        .is_on_cooldown(INSTANT_ABILITY));
    assert_eq!(health(&mgr, C), FULL);
    // Nor may C hit A: the duel admits only its own pair.
    assert!(!handle_use_ability(C, INSTANT_ABILITY, A as i32, &tx, &mut mgr).await);

    // Gate 3: an AoE on the mob does not reach C beside it.
    handle_use_ability_on_ground(A, INSTANT_ABILITY, [2.0, 0.0, 0.0], &tx, &mut mgr).await;
    assert!(health(&mgr, MOB) < FULL, "the AoE fired");
    assert_eq!(health(&mgr, C), FULL, "gate 3 hit the bystander");

    // Gate 4: the cone toward the mob does not collect C.
    let hits = collect_cone_targets(&mgr, A, MOB, 10.0, std::f32::consts::FRAC_PI_4);
    assert!(
        !hits.contains(&C),
        "gate 4 collected the bystander: {hits:?}"
    );
    drain(&mut rx);

    // Gate 2: a warmup launched at the partner does not land once the duel
    // has ended; the partner is a bystander again.
    mgr.get_entity_mut(B).unwrap().position = Vector3::new(3.0, 0.0, 0.0);
    assert!(handle_use_ability(A, WARMUP_ABILITY, B as i32, &tx, &mut mgr).await);
    let duel_id = mgr.duels.duel_of(A_PID).unwrap().duel_id;
    mgr.duels.end_duel(duel_id);
    resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    assert_eq!(
        health(&mgr, B),
        FULL,
        "gate 2 let a finished duel's partner be hit"
    );
    assert!(mgr.get_entity(A).unwrap().pending_cast.is_none());
}

/// Pets stay out of duels (the default until the owner decides otherwise).
/// The widening in `player_may_attack` covers the two engaged players
/// themselves, never a pet: a duelist can neither target nor splash the
/// opponent's pet, and the pet will not take up the fight against the
/// duelist. Fails if the duel rule ever leaks to pets.
#[tokio::test]
async fn duel_opponent_cannot_harm_partner_pet() {
    use crate::cell::combat::{
        area_candidates, may_hit_in_area, player_may_attack, player_may_attack_pve,
    };
    use crate::cell::service::npc_ai::pet::{fight_refusal, threat_refusal};
    use crate::test_support::{add_pet_owner, make_pet_world, PET_FIXTURE_TEMPLATE_ID};

    const ABILITY: i32 = 60;
    let mut mgr = make_pet_world();
    add_pet_owner(&mut mgr, A, "Agnos", [0.0; 3], 10);
    add_pet_owner(&mut mgr, B, "Agnos", [30.0, 0.0, 0.0], 10);
    let (a_pid, b_pid) = (
        mgr.get_entity(A).unwrap().player_id.unwrap(),
        mgr.get_entity(B).unwrap().player_id.unwrap(),
    );
    let pet = mgr
        .spawn_pet_from_template(B, PET_FIXTURE_TEMPLATE_ID, 0)
        .expect("B's pet spawns");
    mgr.get_entity_mut(pet).unwrap().position = cimmeria_common::Vector3::new(3.0, 0.0, 0.0);
    {
        let hp = mgr
            .get_entity_mut(pet)
            .unwrap()
            .stats
            .get_mut(HEALTH)
            .unwrap();
        hp.update(0, FULL, FULL);
        hp.clear_dirty();
    }
    let mut def = super::warmup::cast_ability(ABILITY, 0.0);
    def.event_set_id = None;
    mgr.ability_defs.insert(ABILITY, def);
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "5".to_string());
    mgr.effect_defs.insert(
        500,
        cimmeria_entity::abilities::EffectDef {
            effect_id: 500,
            params,
            ..Default::default()
        },
    );
    {
        let a = mgr.get_entity_mut(A).unwrap();
        a.abilities.add_ability(ABILITY);
        a.weapon_holstered = false;
    }
    let _ = mgr.compute_aoi_changes();

    // A and B duel.
    let now = Instant::now();
    mgr.duels.open_challenge(a_pid, b_pid, now).unwrap();
    let space = mgr.get_entity_space_id(A).expect("A has a space");
    let p = mgr.duels.take_pending_for(b_pid, now).unwrap();
    let duel = mgr
        .duels
        .start_duel(&p, space, Vector3::new(15.0, 0.0, 0.0), now);
    mgr.duels.engage(duel.duel_id, [A, B], now).unwrap();
    let (attacker, pet_e) = (mgr.get_entity(A).unwrap(), mgr.get_entity(pet).unwrap());
    assert!(!player_may_attack(attacker, pet_e, &mgr.duels), "the rule");
    assert!(
        !may_hit_in_area(attacker, pet_e, &mgr.duels),
        "the area rule"
    );
    assert!(!player_may_attack_pve(attacker, pet_e), "the no-duel rule");
    assert!(
        !area_candidates(&mgr, A).contains(&pet),
        "the partner's pet is an area candidate"
    );
    assert!(
        area_candidates(&mgr, A).contains(&B),
        "the partner is a candidate"
    );

    let (tx, _rx) = mpsc::channel(1024);
    // Single target: refused, no damage.
    assert!(!handle_use_ability(A, ABILITY, pet as i32, &tx, &mut mgr).await);
    assert_eq!(
        health(&mgr, pet),
        FULL,
        "single target hit the partner's pet"
    );
    // Ground AoE on the pet: the pet is not collected (B is out of radius).
    handle_use_ability_on_ground(A, ABILITY, [3.0, 0.0, 0.0], &tx, &mut mgr).await;
    assert_eq!(
        health(&mgr, pet),
        FULL,
        "the AoE splashed the partner's pet"
    );
    // Cone: A fires at a hostile mob just past the pet; the cone reaches the
    // pet's spot but must not collect it.
    mgr.spawn_npc(MOB, "Agnos", [6.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(MOB).unwrap().faction = crate::cell::combat::HOSTILE_FACTION;
    let hits = collect_cone_targets(&mgr, A, MOB, 10.0, std::f32::consts::FRAC_PI_4);
    assert!(
        !hits.contains(&pet),
        "the cone collected the partner's pet: {hits:?}"
    );

    // The pet does not take up its owner's duel against A.
    let owner = mgr.get_entity(B).unwrap();
    assert!(fight_refusal(owner, mgr.get_entity(A).unwrap()).is_some());
    assert!(threat_refusal(&mgr, mgr.get_entity(pet).unwrap(), A).is_some());
    let _ = crate::cell::combat::generate_threat(
        &mut mgr,
        A,
        pet,
        50.0,
        crate::cell::combat::AggroCause::Damage,
    );
    assert!(
        !mgr.get_entity(pet).unwrap().threat_list.contains_key(&A),
        "the pet put the duel opponent on its threat list"
    );
}
