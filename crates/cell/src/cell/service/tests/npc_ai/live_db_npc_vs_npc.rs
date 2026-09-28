//! Live-DB smoke for NPC-vs-NPC combat (#1009): a Castle standoff marine and a
//! NID guard, both built from their seeded templates with the seeded abilities,
//! effects and loot tables, fight each other to a death through the real AI
//! tick.
//!
//! The friendly is template 187 (`Castle_OpCoreSoldier_Standoff`, faction 3,
//! Pistol Shot 592) and the guard template 183 (`NID Guard - Castle inside L4`,
//! faction 10, ability set 3, loot table 5). They stand 12 u apart with a
//! player watching from 40 u. Each pass the cooldowns are cleared and any
//! warmup fired at once, so the fight is decided in a few passes instead of a
//! minute of wall time; every hit is a real `damage_apply` against the seeded
//! effect rows. The test asserts damage on both sides, a death, a corpse with
//! no loot and no loot cursor, and no `GrantXP` (an NPC-only kill pays
//! nobody).

use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_ability_defs, load_effect_defs, load_loot_tables, load_spawn_templates,
};
use crate::test_support::require_db_or_skip;

const PLAYER: u32 = 1;
const FRIENDLY: u32 = 200_901;
const GUARD: u32 = 200_902;
const FRIENDLY_TEMPLATE: i32 = 187;
const GUARD_TEMPLATE: i32 = 183;

#[tokio::test]
async fn live_db_castle_standoff_marine_and_guard_fight_to_a_death() {
    let pool = require_db_or_skip!();
    let templates = load_spawn_templates(&pool)
        .await
        .expect("load_spawn_templates must succeed");
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.ability_defs = load_ability_defs(&pool).await.expect("ability defs");
    mgr.effect_defs = load_effect_defs(&pool).await.expect("effect defs");
    mgr.loot_tables = load_loot_tables(&pool).await.expect("loot tables");

    for (id, template_id, x) in [
        (FRIENDLY, FRIENDLY_TEMPLATE, 0.0),
        (GUARD, GUARD_TEMPLATE, 12.0),
    ] {
        let mut record = templates
            .get(&template_id)
            .unwrap_or_else(|| panic!("seeded template {template_id}"))
            .clone();
        record.world_name = "Castle".to_string();
        record.x = x;
        record.y = 0.0;
        record.z = 0.0;
        record.tag = Some(format!("CIMMERIA_TEST_1009_{template_id}"));
        mgr.spawn_npc_from_record(id, &record)
            .expect("spawn from the seeded template");
        crate::cell::service::npc_ai::force_ai_state(
            mgr.get_entity_mut(id).unwrap(),
            AiState::Idle,
        );
    }
    assert_eq!(
        mgr.get_entity(FRIENDLY).unwrap().faction,
        3,
        "seed: 187 is Praxis"
    );
    assert_eq!(
        mgr.get_entity(GUARD).unwrap().faction,
        10,
        "seed: 183 is NID"
    );
    assert!(
        mgr.get_entity(GUARD).unwrap().loot_table_id.is_some(),
        "seed: the guard rolls a loot table on a player kill"
    );
    mgr.create_entity(PLAYER, "Castle", [6.0, 0.0, -40.0], [0.0; 3])
        .unwrap();
    {
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        p.is_player = true;
        p.player_id = Some(PLAYER as i32);
    }
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();

    let max_hp = |mgr: &SpaceManager, id: u32| {
        let h = mgr.get_entity(id).unwrap().stats.get(HEALTH).unwrap();
        (h.cur, h.max)
    };
    let (f0, _) = max_hp(&mgr, FRIENDLY);
    let (g0, _) = max_hp(&mgr, GUARD);

    let events =
        crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new());
    let (tx, mut rx) = mpsc::channel(65_536);
    let mut grants = 0usize;
    let mut dead = None;
    for _ in 0..400 {
        crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr, &events).await;
        for id in [FRIENDLY, GUARD] {
            if let Some(e) = mgr.get_entity_mut(id) {
                if let Some(pc) = e.pending_cast.as_mut() {
                    pc.fire_at = std::time::Instant::now();
                }
            }
        }
        crate::cell::abilities::warmup_tick(&tx, &mut mgr, &events).await;
        while let Ok(m) = rx.try_recv() {
            if matches!(m, CellToBaseMsg::GrantXP { .. }) {
                grants += 1;
            }
        }
        dead = [FRIENDLY, GUARD]
            .into_iter()
            .find(|&id| mgr.get_entity(id).unwrap().ai_state() == AiState::Dead);
        if dead.is_some() {
            break;
        }
        for id in [FRIENDLY, GUARD] {
            let e = mgr.get_entity_mut(id).unwrap();
            e.abilities = cimmeria_entity::abilities::AbilityManager::with_abilities(
                &e.abilities.known_ability_ids(),
            );
        }
    }

    let (f1, _) = max_hp(&mgr, FRIENDLY);
    let (g1, _) = max_hp(&mgr, GUARD);
    assert!(g1 < g0, "the friendly must damage the guard ({g0} -> {g1})");
    assert!(
        f1 < f0,
        "the guard must damage the friendly back ({f0} -> {f1})"
    );
    let dead = dead.expect("one of them must die within 400 passes");
    let corpse = mgr.get_entity(dead).unwrap();
    assert!(crate::cell::combat::is_dead_state(corpse.state_field));
    assert!(corpse.loot.is_empty(), "an NPC-only kill rolls no loot");
    assert_eq!(
        corpse.interaction_type_flags & crate::cell::abilities::INT_NORMAL_LOOT,
        0,
        "and shows no loot cursor"
    );
    assert_eq!(grants, 0, "an NPC-only kill pays no XP");
    let survivor = if dead == FRIENDLY { GUARD } else { FRIENDLY };
    assert!(
        !mgr.get_entity(survivor)
            .unwrap()
            .threat_list
            .contains_key(&dead),
        "the survivor drops the corpse from its threat list at once"
    );
}
