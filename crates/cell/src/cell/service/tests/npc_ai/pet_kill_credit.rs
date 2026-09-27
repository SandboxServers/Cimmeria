//! Pets PT-06 acceptance: a pet's AI kill advances its owner's KillCount
//! objective, through the real fight tick and the real chain engine.
//!
//! ```text
//!   npc_ai_tick -> npc_ai_fight (the pet, Fighting the tagged mob)
//!     -> handle_use_ability_with_kill_credit   (pets only)
//!       -> resolve_death -> grant_kill_xp      (GrantXP to the owner)
//!       -> credited_player -> EngineEvents::entity_death(owner, ...)
//!         -> ChainEngine: OnEntityDeath{tag} -> IncrementCounter
//! ```
//!
//! The chain is registered in memory, the shape of the seeded KillCount
//! chains (`entity_dead_tag` -> `increment_counter`, e.g. Castle chain
//! 1093), because what is under test is the runtime credit path, not the
//! loader.

use super::*;
use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::messages::CellToBaseMsg;
use crate::test_support::{add_pet_owner, seed_pet_template, PET_FIXTURE_TEMPLATE_ID};
use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::abilities::EffectDef;
use tokio::sync::mpsc;

const OWNER: u32 = 7;
const OWNER_PLAYER_ID: i32 = 700;
const MOB_TAG: &str = "CIMMERIA_TEST_PT06_KILLCOUNT_MOB";
const COUNTER: &str = "pt06_kills";
const EFFECT_ID: i32 = 0x7000_0604;
/// Level 5 pays `kill_xp(5)` = 50.
const MOB_LEVEL: u32 = 5;

/// The owner, its pet Fighting a tagged level-5 mob 4 u away with the mob
/// on its threat list, and the pet's Pistol Shot (the fixture template's
/// first ability) made lethal.
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

    seed_default_ability(&mut mgr, 0, 30);
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "9999".to_string());
    mgr.effect_defs.insert(
        EFFECT_ID,
        EffectDef {
            effect_id: EFFECT_ID,
            ability_id: crate::cell::combat::NPC_DEFAULT_ABILITY,
            params,
            ..Default::default()
        },
    );
    mgr.ability_defs
        .get_mut(&crate::cell::combat::NPC_DEFAULT_ABILITY)
        .unwrap()
        .effect_ids = vec![EFFECT_ID];

    let pet_pos = mgr.get_entity(pet).unwrap().position;
    let mob = mgr.allocate_npc_id();
    mgr.spawn_npc(
        mob,
        "Castle",
        [pet_pos.x + 4.0, pet_pos.y, pet_pos.z],
        [0.0; 3],
    )
    .unwrap();
    {
        let m = mgr.get_entity_mut(mob).unwrap();
        m.faction = HOSTILE_FACTION;
        m.level = MOB_LEVEL;
        m.tag = Some(MOB_TAG.to_string());
        let hp = m.stats.get_mut(HEALTH).unwrap();
        hp.update(0, 100, 100);
        hp.clear_dirty();
    }
    {
        let p = mgr.get_entity_mut(pet).unwrap();
        p.threat_list.insert(mob, 10.0);
        crate::cell::service::npc_ai::force_ai_state(p, AiState::Fighting);
    }
    let _ = mgr.compute_aoi_changes();
    (mgr, pet, mob)
}

/// A KillCount chain: each death of the tagged mob bumps `COUNTER` on the
/// credited entity.
fn kill_count_engine() -> ChainEngine {
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        action_delays: Vec::new(),
        id: 0x7000_0605,
        name: "test: PT-06 kill count".to_string(),
        enabled: true,
        trigger: Trigger::OnEntityDeath {
            entity_type: None,
            entity_tag: Some(MOB_TAG.to_string()),
        },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: COUNTER.to_string(),
            amount: 1,
        }],
        priority: 0,
    });
    engine
}

/// **The packet's chain-replay acceptance.** One AI tick: the pet kills the
/// tagged mob, the owner's counter reads 1, the pet's reads nothing, and the
/// kill XP goes to the owner.
///
/// Reverting the fight tick's pet branch (bare `handle_use_ability` for
/// every NPC) leaves the counter unset; reverting `credited_player` to the
/// caster's own `player_id` does the same; reverting `grant_kill_xp` to the
/// attacker id sends the `GrantXP` to the pet.
#[tokio::test]
async fn a_pet_ai_kill_advances_the_owners_kill_count() {
    let (mut mgr, pet, mob) = world();
    let engine = kill_count_engine();
    let (tx, mut rx) = mpsc::channel(1024);

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &crate::cell::content::EngineEvents(&engine),
    )
    .await;

    assert_eq!(
        mgr.get_entity(mob)
            .and_then(|e| e.stats.get(HEALTH))
            .map(|s| s.cur),
        Some(0),
        "fixture: the pet's fight tick must kill the mob"
    );
    assert_eq!(
        mgr.get_entity(OWNER).unwrap().counters.get(COUNTER),
        Some(&1),
        "the KillCount chain must credit the owner"
    );
    assert_eq!(
        mgr.get_entity(pet).unwrap().counters.get(COUNTER),
        None,
        "the pet is never the credited entity"
    );

    let mut grants = Vec::new();
    while let Ok(m) = rx.try_recv() {
        if let CellToBaseMsg::GrantXP {
            entity_id,
            xp_amount,
            ..
        } = m
        {
            grants.push((entity_id, xp_amount));
        }
    }
    assert_eq!(grants, vec![(OWNER, 50)], "kill XP goes to the owner");
}
