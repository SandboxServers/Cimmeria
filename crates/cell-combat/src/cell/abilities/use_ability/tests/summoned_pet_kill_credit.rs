//! Pets PT-03 x PT-06: a pet that came out of the real summon cast earns
//! its owner's kill credit on the two kill paths PT-06's own fixtures
//! could not reach, because no summonable pet existed before PT-03:
//!
//! - a warmed-up cast whose warmup the 100 ms tick resolves
//!   (`resolve_warmups` -> `fire_due_cast`);
//! - a DoT the pet applied, finished by a later pulse (`effect_pulse_tick`
//!   -> `dot_kill_credit`).
//!
//! Both pay the XP through `kill_xp_payout` / `credit_recipient` and raise
//! `EntityDeath` on the owner. A pet whose owner's entity id now belongs to
//! another player (destroyed and reused before the sweep) credits nobody.
//!
//! Depends on PT-06 (#889): before it, a pet's kill sent `GrantXP` to the
//! pet's own id and raised no `EntityDeath`.

use std::time::{Duration, Instant};

use cimmeria_cell_world::test_fixtures::add_pet_owner;
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::HEALTH;

use super::summon::{cast_and_complete, summon_mgr, OWNER};
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::effects::{effect_pulse_tick, register_active_effect};
use crate::test_support::{RecordedContentEvent, RecordingContentEvents};

const OWNER_PLAYER_ID: i32 = 700;
const MOB_TAG: &str = "CIMMERIA_TEST_PT03_PET_KILL";
/// Level 5 pays `kill_xp(5)` = 50 XP; the seeded `transfer_xp` is 1.0.
const MOB_LEVEL: u32 = 5;
const MOB_XP: u64 = 50;
const WARMUP_ABILITY: i32 = 0x7000_0310;
const WARMUP_EFFECT: i32 = 0x7000_0311;
const DOT_EFFECT: i32 = 0x7000_0312;
const DOT_ABILITY: i32 = 0x7000_0313;
const PET_WARMUP: f32 = 1.0;

/// The summon fixture, with the owner's character id known before the
/// summon (so the pet's summon-time identity carries it), the 2826 cast run
/// to completion, and one tagged hostile level-5 mob beside the pet.
/// Returns `(mgr, pet, mob)`.
async fn summoned_pet_and_mob() -> (SpaceManager, u32, u32) {
    let mut mgr = summon_mgr();
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(OWNER_PLAYER_ID);
    let (tx, mut rx) = mpsc::channel(512);
    cast_and_complete(&mut mgr, 0, &tx, &mut rx).await;
    let pets = mgr.pets.pets_of(OWNER);
    assert_eq!(pets.len(), 1, "fixture: the summon spawns one pet");
    let pet = pets[0];

    let mob = mgr.allocate_npc_id();
    mgr.spawn_npc(mob, "Castle", [4.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    {
        let m = mgr.get_entity_mut(mob).unwrap();
        m.level = MOB_LEVEL;
        m.faction = HOSTILE_FACTION;
        m.tag = Some(MOB_TAG.to_string());
        let hp = m.stats.get_mut(HEALTH).unwrap();
        hp.update(0, 100, 100);
        hp.clear_dirty();
    }
    let _ = mgr.compute_aoi_changes();
    (mgr, pet, mob)
}

/// A lethal ability with a warmup, known by `pet`.
fn teach_warmup_kill(mgr: &mut SpaceManager, pet: u32) {
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "9999".to_string());
    mgr.effect_defs.insert(
        WARMUP_EFFECT,
        EffectDef {
            effect_id: WARMUP_EFFECT,
            ability_id: WARMUP_ABILITY,
            params,
            ..Default::default()
        },
    );
    mgr.ability_defs.insert(
        WARMUP_ABILITY,
        AbilityDef {
            cooldown: 0.0,
            warmup: PET_WARMUP,
            effect_ids: vec![WARMUP_EFFECT],
            ..make_ability(WARMUP_ABILITY, 0, 30)
        },
    );
    mgr.get_entity_mut(pet)
        .unwrap()
        .abilities
        .add_ability(WARMUP_ABILITY);
}

/// `pet`'s lethal DoT on `mob`, registered and already due to pulse.
async fn arm_pet_dot(mgr: &mut SpaceManager, pet: u32, mob: u32) {
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "500".to_string());
    let effect = EffectDef {
        effect_id: DOT_EFFECT,
        ability_id: DOT_ABILITY,
        pulse_count: 5,
        pulse_duration: 1.0,
        params,
        ..Default::default()
    };
    mgr.effect_defs.insert(DOT_EFFECT, effect.clone());
    let past = Instant::now() - Duration::from_secs(2);
    let (tx, _rx) = mpsc::channel(64);
    assert!(
        register_active_effect(mgr, mob, pet, &effect, past, &tx).await,
        "fixture: the DoT registers"
    );
    if let Some(inst) = mgr
        .get_entity_mut(mob)
        .and_then(|t| t.active_effects.first_mut())
    {
        inst.next_pulse_at = past;
    }
}

/// Destroy `OWNER` and hand its entity id to a different player (another
/// account and character) before the pet sweep runs: the reuse window
/// #870's per-pet summoner identity closes.
fn reuse_owner_id(mgr: &mut SpaceManager) {
    mgr.destroy_entity(OWNER);
    add_pet_owner(mgr, OWNER, "Castle", [0.0; 3], 50);
    let impostor = mgr.get_entity_mut(OWNER).unwrap();
    impostor.account_id = Some(4242);
    impostor.player_id = Some(4243);
}

fn grants(sent: &[CellToBaseMsg]) -> Vec<(u32, u64)> {
    sent.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::GrantXP {
                entity_id,
                xp_amount,
                ..
            } => Some((*entity_id, *xp_amount)),
            _ => None,
        })
        .collect()
}

fn deaths(recorder: &RecordingContentEvents) -> Vec<RecordedContentEvent> {
    recorder
        .events()
        .into_iter()
        .filter(|e| matches!(e, RecordedContentEvent::EntityDeath { .. }))
        .collect()
}

fn owner_death() -> RecordedContentEvent {
    RecordedContentEvent::EntityDeath {
        killer_entity_id: OWNER,
        player_id: OWNER_PLAYER_ID,
        entity_tag: MOB_TAG.to_string(),
    }
}

fn mob_hp(mgr: &SpaceManager, mob: u32) -> i32 {
    mgr.get_entity(mob).unwrap().stats.get(HEALTH).unwrap().cur
}

/// **Guard.** A summoned pet's warmed-up cast kills the mob when the warmup
/// tick fires it: the owner gets the XP and the `EntityDeath`, the pet gets
/// neither. Reverting PT-06's `credit_recipient` check in `fire_due_cast`
/// drops the `EntityDeath`; reverting `kill_xp_payout` sends the XP to the
/// pet's id.
#[tokio::test]
async fn a_summoned_pets_warmup_kill_credits_the_owner() {
    let (mut mgr, pet, mob) = summoned_pet_and_mob().await;
    teach_warmup_kill(&mut mgr, pet);
    let (tx, mut rx) = mpsc::channel(512);

    assert!(
        handle_use_ability(pet, WARMUP_ABILITY, mob as i32, &tx, &mut mgr).await,
        "the pet's cast must commit"
    );
    assert!(
        mgr.get_entity(pet).unwrap().pending_cast.is_some(),
        "fixture: the cast is held for its warmup"
    );
    assert_eq!(mob_hp(&mgr, mob), 100, "nothing lands before the warmup");
    drain(&mut rx);

    let recorder = RecordingContentEvents::new();
    let later = Instant::now() + Duration::from_secs_f32(PET_WARMUP) + Duration::from_millis(50);
    assert_eq!(resolve_warmups(later, &tx, &mut mgr, &recorder).await, 1);

    assert_eq!(mob_hp(&mgr, mob), 0, "fixture: the warmed-up cast kills");
    assert_eq!(grants(&drain(&mut rx)), vec![(OWNER, MOB_XP)]);
    assert_eq!(deaths(&recorder), vec![owner_death()]);
}

/// **Guard.** A summoned pet's DoT finishes the mob on a later pulse: the
/// owner gets the XP and the `EntityDeath`.
#[tokio::test]
async fn a_summoned_pets_dot_kill_credits_the_owner() {
    let (mut mgr, pet, mob) = summoned_pet_and_mob().await;
    arm_pet_dot(&mut mgr, pet, mob).await;
    let recorder = RecordingContentEvents::new();
    let (tx, mut rx) = mpsc::channel(512);

    effect_pulse_tick(&recorder, &tx, &mut mgr).await;

    assert_eq!(mob_hp(&mgr, mob), 0, "fixture: the pulse kills");
    assert_eq!(grants(&drain(&mut rx)), vec![(OWNER, MOB_XP)]);
    assert_eq!(deaths(&recorder), vec![owner_death()]);
}

/// **Guard.** The owner's entity id is reused by another player before the
/// sweep, and the pet's DoT then kills: the mob still dies, but nobody is
/// credited. The id's new holder gets no XP and no `EntityDeath`.
#[tokio::test]
async fn a_summoned_pets_dot_kill_after_owner_id_reuse_credits_nobody() {
    let (mut mgr, pet, mob) = summoned_pet_and_mob().await;
    arm_pet_dot(&mut mgr, pet, mob).await;
    reuse_owner_id(&mut mgr);
    assert_eq!(mgr.pets.owner_of(pet), Some(OWNER), "not swept yet");
    let recorder = RecordingContentEvents::new();
    let (tx, mut rx) = mpsc::channel(512);

    effect_pulse_tick(&recorder, &tx, &mut mgr).await;

    assert_eq!(mob_hp(&mgr, mob), 0, "the kill itself still lands");
    assert_eq!(grants(&drain(&mut rx)), vec![]);
    assert_eq!(deaths(&recorder), vec![]);
}

/// **Guard.** The same reuse, on the warmup path: the warmed-up kill lands
/// but credits nobody.
#[tokio::test]
async fn a_summoned_pets_warmup_kill_after_owner_id_reuse_credits_nobody() {
    let (mut mgr, pet, mob) = summoned_pet_and_mob().await;
    teach_warmup_kill(&mut mgr, pet);
    let (tx, mut rx) = mpsc::channel(512);
    assert!(handle_use_ability(pet, WARMUP_ABILITY, mob as i32, &tx, &mut mgr).await);
    reuse_owner_id(&mut mgr);
    drain(&mut rx);

    let recorder = RecordingContentEvents::new();
    let later = Instant::now() + Duration::from_secs_f32(PET_WARMUP) + Duration::from_millis(50);
    assert_eq!(resolve_warmups(later, &tx, &mut mgr, &recorder).await, 1);

    assert_eq!(mob_hp(&mgr, mob), 0, "the kill itself still lands");
    assert_eq!(grants(&drain(&mut rx)), vec![]);
    assert_eq!(deaths(&recorder), vec![]);
}
