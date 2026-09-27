//! Pets PT-06 (#889 review): kill credit for a pet on the warmup and DoT
//! kill paths, with the pet registered through the fixture spawn
//! (`spawn_pet_from_template` -> `PetRegistry::register(owner, pet,
//! summoner)`) rather than the summon cast, which is PT-03's (#890; its
//! `summoned_pet_kill_credit.rs` covers the same paths from a real summon).
//!
//! - a warmed-up cast whose warmup the 100 ms tick resolves
//!   (`resolve_warmups` -> `fire_due_cast`, the `credit_recipient_quiet`
//!   gate in `warmup/tick.rs`);
//! - a DoT the pet applied, finished by a later pulse (`effect_pulse_tick`
//!   -> `dot_kill_credit` -> `credited_player`).
//!
//! Both pay the owner the XP and raise `EntityDeath` on the owner. A pet
//! whose owner's entity id now belongs to another player credits nobody.

use std::time::{Duration, Instant};

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::HEALTH;

use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::effects::{effect_pulse_tick, register_active_effect};
use crate::test_support::{
    add_pet_owner, seed_pet_template, RecordedContentEvent, RecordingContentEvents,
    PET_FIXTURE_TEMPLATE_ID,
};

const OWNER: u32 = 7;
const OWNER_PLAYER_ID: i32 = 700;
const MOB_TAG: &str = "CIMMERIA_TEST_PT06_REGISTERED_PET_KILL";
/// Level 5 pays `kill_xp(5)` = 50 XP; the fixture's `transfer_xp` is 1.0.
const MOB_LEVEL: u32 = 5;
const MOB_XP: u64 = 50;
const WARMUP_ABILITY: i32 = 0x7000_0604;
const WARMUP_EFFECT: i32 = 0x7000_0605;
const DOT_EFFECT: i32 = 0x7000_0606;
const DOT_ABILITY: i32 = 0x7000_0607;
const PET_WARMUP: f32 = 1.0;

/// `OWNER` (character id known before the spawn, so the pet's summon-time
/// identity carries it) with one registered pet, and one tagged hostile
/// level-5 mob at 100 HP beside it. Returns `(mgr, pet, mob)`.
fn registered_pet_and_mob() -> (SpaceManager, u32, u32) {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    seed_pet_template(&mut mgr, PET_FIXTURE_TEMPLATE_ID);
    add_pet_owner(&mut mgr, OWNER, "Castle", [0.0; 3], 12);
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(OWNER_PLAYER_ID);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 2826)
        .expect("fixture: the pet spawns");
    assert_eq!(mgr.pets.owner_of(pet), Some(OWNER), "fixture: registered");

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

/// Destroy `OWNER` and hand its entity id to a different player before the
/// pet sweep runs: the id-reuse window #870's per-pet summoner identity
/// closes.
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

/// Cast the warmup kill, optionally reuse the owner's id while it warms up,
/// then let the warmup tick fire it. Returns what the fire sent.
async fn warmup_kill(
    mgr: &mut SpaceManager,
    pet: u32,
    mob: u32,
    reuse_before_fire: bool,
    recorder: &RecordingContentEvents,
) -> Vec<CellToBaseMsg> {
    teach_warmup_kill(mgr, pet);
    let (tx, mut rx) = mpsc::channel(512);
    assert!(
        handle_use_ability(pet, WARMUP_ABILITY, mob as i32, &tx, mgr).await,
        "the pet's cast must commit"
    );
    assert!(
        mgr.get_entity(pet).unwrap().pending_cast.is_some(),
        "fixture: the cast is held for its warmup"
    );
    assert_eq!(mob_hp(mgr, mob), 100, "nothing lands before the warmup");
    if reuse_before_fire {
        reuse_owner_id(mgr);
        assert_eq!(mgr.pets.owner_of(pet), Some(OWNER), "not swept yet");
    }
    drain(&mut rx);

    let later = Instant::now() + Duration::from_secs_f32(PET_WARMUP) + Duration::from_millis(50);
    assert_eq!(resolve_warmups(later, &tx, mgr, recorder).await, 1);
    assert_eq!(mob_hp(mgr, mob), 0, "the warmed-up cast kills");
    drain(&mut rx)
}

/// **Guard.** A registered pet's warmed-up cast kills a tagged mob when the
/// warmup tick fires it: the owner gets the XP and the `EntityDeath`, the
/// pet neither. Reverting the `warmup/tick.rs` gate to the caster's own
/// `is_player` drops the `EntityDeath`.
#[tokio::test]
async fn registered_pet_warmup_kill_credits_the_owner() {
    let (mut mgr, pet, mob) = registered_pet_and_mob();
    let recorder = RecordingContentEvents::new();
    let sent = warmup_kill(&mut mgr, pet, mob, false, &recorder).await;
    assert_eq!(grants(&sent), vec![(OWNER, MOB_XP)]);
    assert_eq!(deaths(&recorder), vec![owner_death()]);
}

/// **Guard.** The owner's id is reused by another player while the cast
/// warms up: the kill lands but credits nobody, and the id's new holder
/// gets neither XP nor `EntityDeath`.
#[tokio::test]
async fn registered_pet_warmup_kill_after_owner_id_reuse_credits_nobody() {
    let (mut mgr, pet, mob) = registered_pet_and_mob();
    let recorder = RecordingContentEvents::new();
    let sent = warmup_kill(&mut mgr, pet, mob, true, &recorder).await;
    assert_eq!(grants(&sent), vec![]);
    assert_eq!(deaths(&recorder), vec![]);
}

/// **Guard.** A registered pet's DoT finishes a tagged mob on a later
/// pulse: the owner gets the XP and the `EntityDeath`. Reverting
/// `dot_kill_credit` to the invoker's own `player_id` drops the
/// `EntityDeath`.
#[tokio::test]
async fn registered_pet_dot_kill_credits_the_owner() {
    let (mut mgr, pet, mob) = registered_pet_and_mob();
    arm_pet_dot(&mut mgr, pet, mob).await;
    let recorder = RecordingContentEvents::new();
    let (tx, mut rx) = mpsc::channel(512);

    effect_pulse_tick(&recorder, &tx, &mut mgr).await;

    assert_eq!(mob_hp(&mgr, mob), 0, "fixture: the pulse kills");
    assert_eq!(grants(&drain(&mut rx)), vec![(OWNER, MOB_XP)]);
    assert_eq!(deaths(&recorder), vec![owner_death()]);
}

/// **Guard.** The owner's id is reused before the DoT's lethal pulse: the
/// kill lands but credits nobody.
#[tokio::test]
async fn registered_pet_dot_kill_after_owner_id_reuse_credits_nobody() {
    let (mut mgr, pet, mob) = registered_pet_and_mob();
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
