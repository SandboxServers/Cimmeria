//! Pets PT-06: mission kill credit for a pet's kills goes to its owner.
//!
//! `handle_use_ability_with_kill_credit` and `credit_ground_deaths` raise
//! `EntityDeath` on the credited player (`credited_player`, over
//! `SpaceManager::credit_recipient`). For a pet that is the owner, so the
//! chain sees the owner's mission context and `IncrementCounter` bumps the
//! owner's counters. Before the seam a pet caster had no `player_id`, so the
//! event was dropped with a warn and KillCount missions never advanced.

use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::content_events::ContentEvents;
use crate::test_support::{
    add_pet_owner, seed_pet_template, RecordedContentEvent, RecordingContentEvents,
    PET_FIXTURE_TEMPLATE_ID,
};
use cimmeria_entity::abilities::{AbilityDef, EffectDef};
use cimmeria_entity::stats::HEALTH;

use super::*;

const OWNER: u32 = 7;
const OWNER_PLAYER_ID: i32 = 700;
const MOB_TAG: &str = "CIMMERIA_TEST_PT06_MOB";
const ABILITY_ID: i32 = 0x7000_0602;
const EFFECT_ID: i32 = 0x7000_0603;

/// `OWNER` and its pet in the shared Castle space, one tagged hostile mob
/// at 100 HP in range, and a lethal ability the pet knows. Returns
/// `(mgr, pet, mob)`.
fn world() -> (SpaceManager, u32, u32) {
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

    let mob = mgr.allocate_npc_id();
    mgr.spawn_npc(mob, "Castle", [4.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    {
        let m = mgr.get_entity_mut(mob).unwrap();
        m.faction = HOSTILE_FACTION;
        m.tag = Some(MOB_TAG.to_string());
        let hp = m.stats.get_mut(HEALTH).unwrap();
        hp.update(0, 100, 100);
        hp.clear_dirty();
    }

    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "9999".to_string());
    mgr.effect_defs.insert(
        EFFECT_ID,
        EffectDef {
            effect_id: EFFECT_ID,
            ability_id: ABILITY_ID,
            params,
            ..Default::default()
        },
    );
    mgr.ability_defs.insert(
        ABILITY_ID,
        AbilityDef {
            cooldown: 0.0,
            effect_ids: vec![EFFECT_ID],
            ..make_ability(ABILITY_ID, 0, 30)
        },
    );
    mgr.get_entity_mut(pet)
        .unwrap()
        .abilities
        .add_ability(ABILITY_ID);
    let _ = mgr.compute_aoi_changes();
    (mgr, pet, mob)
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

/// **Guard (A-27).** A pet's killing cast raises one `EntityDeath` on the
/// owner. Reverted, the pet has no `player_id` and no event is raised.
#[tokio::test]
async fn a_pet_kill_raises_entity_death_for_the_owner() {
    let (mut mgr, pet, mob) = world();
    let recorder = RecordingContentEvents::new();
    let (tx, _rx) = mpsc::channel(512);
    let events: &dyn ContentEvents = &recorder;

    assert!(
        handle_use_ability_with_kill_credit(pet, ABILITY_ID, mob as i32, events, &tx, &mut mgr)
            .await,
        "the pet's cast must commit"
    );
    assert_eq!(
        mgr.get_entity(mob).unwrap().stats.get(HEALTH).unwrap().cur,
        0,
        "fixture: 9999 damage must kill"
    );
    assert_eq!(deaths(&recorder), vec![owner_death()]);
}

/// **Guard (A-27).** The ground-target credit path resolves the pet to the
/// owner the same way.
#[tokio::test]
async fn a_pet_ground_kill_raises_entity_death_for_the_owner() {
    let (mut mgr, pet, mob) = world();
    let recorder = RecordingContentEvents::new();
    let (tx, _rx) = mpsc::channel(512);
    credit_ground_deaths(pet, vec![mob], &recorder, &tx, &mut mgr).await;
    assert_eq!(deaths(&recorder), vec![owner_death()]);
}

/// A plain NPC caster still credits nobody: no event, as before.
#[tokio::test]
async fn a_plain_npc_ground_kill_raises_no_entity_death() {
    let (mut mgr, _pet, mob) = world();
    let other = mgr.allocate_npc_id();
    mgr.spawn_npc(other, "Castle", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let recorder = RecordingContentEvents::new();
    let (tx, _rx) = mpsc::channel(512);
    credit_ground_deaths(other, vec![mob], &recorder, &tx, &mut mgr).await;
    assert_eq!(deaths(&recorder), vec![]);
}
