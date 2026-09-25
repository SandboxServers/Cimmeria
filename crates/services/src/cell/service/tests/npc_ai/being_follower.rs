//! NA24 (UAT-1 C, colo 2026-09-25 12:13): Col Marsh (template 10,
//! `class = 'being'`) was put in `Follow` by chain 1176 and never moved --
//! no `npc_ai.tick`, follow or movement row at all -- because both the AI
//! tick and the movement tick drew their candidates from the SGWMob-only
//! `all_npc_entity_ids`. The GC1 escort fixtures hid it by spawning template
//! 10 as `class: "mob"`.

use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::cell_entity::AiState;
use tokio::sync::mpsc;

const PLAYER: u32 = 100;

/// Template 10's shape as the seed loads it: a `being`.
fn being_record(template_name: &str) -> crate::cell::spawner::SpawnRecord {
    crate::cell::spawner::SpawnRecord {
        spawn_id: 7,
        world_name: "Castle".to_string(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        heading: 0.0,
        tag: Some("Preparation_ColMarsh".to_string()),
        template_id: 10,
        template_name: template_name.to_string(),
        class: "being".to_string(),
        static_mesh: None,
        body_set: "BS_HumanMale.BS_HumanMale".to_string(),
        components: None,
        flags: 24,
        interaction_type: 0,
        event_set_id: None,
        level: Some(30),
        alignment: Some(0),
        faction: Some(3),
        name_id: None,
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: true,
        loot_table_id: None,
        is_stationary: false,
        ability_ids: vec![],
        respawn_secs: None,
        patrol_path: vec![],
        patrol_point_delay_secs: 2.0,
        wander_radius: 0.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.9,
        leash_distance: None,
        aggro_radius: None,
        assist_radius: None,
        aggression_override: None,
        use_cover: None,
    }
}

fn castle_with_player() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER, "Castle", [57.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(PLAYER as i32);
    }
    mgr.connect_entity(PLAYER);
    mgr
}

/// A `being` in `Follow` 57 u from its leader (the colo distance) is ticked,
/// plans a leg toward the leader, and walks it. Revert proof: point the AI
/// tick and the movement tick back at `all_npc_entity_ids` and Marsh gets no
/// path and stays at x = 0.
#[tokio::test]
async fn following_being_is_ticked_and_moves_toward_its_leader() {
    let mut mgr = castle_with_player();
    let marsh = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(marsh, &being_record("Col Marsh (pet)"))
        .unwrap();
    assert_eq!(mgr.get_entity(marsh).unwrap().class_id, 0x01);
    if let Some(npc) = mgr.get_entity_mut(marsh) {
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Follow);
        npc.follow_target_id = Some(PLAYER);
    }

    let (tx, _rx) = mpsc::channel(256);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr, &engine).await;
    assert!(
        !mgr.get_entity(marsh).unwrap().nav_path.is_empty(),
        "the follow handler must have run and planned a leg toward the leader"
    );
    for _ in 0..10 {
        crate::cell::service::ticks::npc_movement_tick(&mut mgr);
    }
    let x = mgr.get_entity(marsh).unwrap().position.x;
    assert!(
        x > 5.0,
        "ten movement ticks at 0.9 u/tick must carry Marsh toward the player, got x = {x}"
    );
}

/// The widening admits no props: an `Idle` being (a crate, a console) is not
/// ticked, and a being shot into `Fighting` gets no fight pass.
#[test]
fn idle_or_fighting_being_is_not_ai_driven() {
    let mut mgr = castle_with_player();
    let crate_id = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(crate_id, &being_record("SGC Crate"))
        .unwrap();
    assert!(
        !mgr.ai_driven_npc_entity_ids().contains(&crate_id),
        "an Idle being is a prop"
    );
    if let Some(npc) = mgr.get_entity_mut(crate_id) {
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Fighting);
    }
    assert!(
        !mgr.ai_driven_npc_entity_ids().contains(&crate_id),
        "a being in Fighting must never get a fight pass"
    );
    if let Some(npc) = mgr.get_entity_mut(crate_id) {
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Follow);
    }
    assert!(mgr.ai_driven_npc_entity_ids().contains(&crate_id));
}
