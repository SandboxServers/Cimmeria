//! In-memory guards for the opt-in minigame callback resolver.

use std::collections::HashMap;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::conditions::{
    ComparisonOp, Condition, MissionStatusValue, StepStatusValue,
};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::missions::MissionInstance;
use tokio::sync::mpsc;

use super::fire_guarded_minigame_chain_by_id;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::WorldRow;

const CHAIN_ID: i64 = 71_520;
const PLAYER: u32 = 1;
const OTHER_PLAYER: u32 = 2;
const NPC: u32 = 3;

fn make_mgr(player_world: &str) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        <Space WorldName="Elsewhere" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /><Space WorldName="Elsewhere" /></Spaces>"#,
    )
    .unwrap();
    mgr.stamp_world_rows(&HashMap::from([
        ("Castle".to_string(), WorldRow::enforcing(8)),
        ("Elsewhere".to_string(), WorldRow::enforcing(9)),
    ]));
    for (id, world) in [
        (PLAYER, player_world),
        (OTHER_PLAYER, "Castle"),
        (NPC, "Castle"),
    ] {
        mgr.create_entity(id, world, [0.0; 3], [0.0; 3]).unwrap();
    }
    for (id, player_id) in [(PLAYER, 100), (OTHER_PLAYER, 200)] {
        let entity = mgr.get_entity_mut(id).unwrap();
        entity.is_player = true;
        entity.player_id = Some(player_id);
        entity
            .missions
            .add_mission(MissionInstance::new(1520, 4453, vec![]));
        mgr.connect_entity(id);
    }
    mgr
}

fn chain(id: i64, conditions: Vec<Condition>, action: Action) -> Chain {
    Chain {
        id,
        name: "guarded callback test".to_string(),
        enabled: true,
        trigger: Trigger::OnCustomEvent {
            event_name: format!("__direct_invoke_{CHAIN_ID}"),
        },
        conditions,
        actions: vec![action],
        action_delays: vec![],
        priority: 0,
        once: false,
    }
}

fn gated_engine() -> ChainEngine {
    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_ID,
        vec![
            Condition::World {
                operator: ComparisonOp::Eq,
                world_id: 8,
            },
            Condition::MissionStatus {
                mission_id: 1520,
                operator: ComparisonOp::Eq,
                expected_status: MissionStatusValue::Active,
            },
            Condition::StepStatus {
                mission_id: 1520,
                step_id: 4453,
                operator: ComparisonOp::Eq,
                expected_status: StepStatusValue::Active,
            },
        ],
        Action::IncrementCounter {
            counter_name: "callback".to_string(),
            amount: 1,
        },
    ));
    engine
}

async fn invoke_counter(mgr: &mut SpaceManager, engine: &ChainEngine, entity_id: u32) -> i32 {
    let (tx, mut rx) = mpsc::channel(8);
    fire_guarded_minigame_chain_by_id(CHAIN_ID, entity_id, engine, &tx, mgr).await;
    assert!(rx.try_recv().is_err());
    mgr.get_entity(entity_id)
        .and_then(|entity| entity.counters.get("callback"))
        .copied()
        .unwrap_or(0)
}

#[tokio::test]
async fn direct_invoke_resolves_current_world_mission_and_step() {
    let engine = gated_engine();
    let mut mgr = make_mgr("Castle");
    assert_eq!(invoke_counter(&mut mgr, &engine, PLAYER).await, 1);
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .missions
        .get_mission_mut(1520)
        .unwrap()
        .current_step_id = Some(4454);
    assert_eq!(
        invoke_counter(&mut mgr, &engine, PLAYER).await,
        1,
        "wrong step ran the action"
    );

    let mut mgr = make_mgr("Castle");
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .missions
        .get_mission_mut(1520)
        .unwrap()
        .complete();
    assert_eq!(
        invoke_counter(&mut mgr, &engine, PLAYER).await,
        0,
        "inactive mission ran the action"
    );

    let mut mgr = make_mgr("Elsewhere");
    assert_eq!(
        invoke_counter(&mut mgr, &engine, PLAYER).await,
        0,
        "wrong world ran the action"
    );

    let mut mgr = make_mgr("Castle");
    mgr.get_entity_mut(OTHER_PLAYER)
        .unwrap()
        .missions
        .get_mission_mut(1520)
        .unwrap()
        .current_step_id = Some(4454);
    assert_eq!(
        invoke_counter(&mut mgr, &engine, OTHER_PLAYER).await,
        0,
        "another player's step ran the action"
    );
}

#[tokio::test]
async fn direct_invoke_filters_id_disabled_and_real_trigger() {
    let mut mgr = make_mgr("Castle");
    let mut engine = gated_engine();
    engine.register_chain(chain(
        CHAIN_ID + 1,
        vec![],
        Action::IncrementCounter {
            counter_name: "other_chain".to_string(),
            amount: 1,
        },
    ));
    assert_eq!(invoke_counter(&mut mgr, &engine, PLAYER).await, 1);
    assert_eq!(
        mgr.get_entity(PLAYER).unwrap().counters.get("other_chain"),
        None
    );

    let mut mgr = make_mgr("Castle");
    let mut disabled = chain(
        CHAIN_ID,
        vec![],
        Action::IncrementCounter {
            counter_name: "callback".to_string(),
            amount: 1,
        },
    );
    disabled.enabled = false;
    let mut engine = ChainEngine::new();
    engine.register_chain(disabled);
    assert_eq!(
        invoke_counter(&mut mgr, &engine, PLAYER).await,
        0,
        "disabled chain ran"
    );

    let mut mgr = make_mgr("Castle");
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        trigger: Trigger::OnPlayerLoaded { world_name: None },
        ..chain(
            CHAIN_ID,
            vec![],
            Action::IncrementCounter {
                counter_name: "callback".to_string(),
                amount: 1,
            },
        )
    });
    assert_eq!(
        invoke_counter(&mut mgr, &engine, PLAYER).await,
        0,
        "real trigger ran"
    );
}

#[tokio::test]
async fn direct_invoke_unconditioned_triggerless_chain_runs_once() {
    let mut mgr = make_mgr("Castle");
    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_ID,
        vec![],
        Action::IncrementCounter {
            counter_name: "callback".to_string(),
            amount: 1,
        },
    ));
    assert_eq!(invoke_counter(&mut mgr, &engine, PLAYER).await, 1);
}

#[tokio::test]
async fn direct_invoke_requires_present_nonzero_player() {
    let mut mgr = make_mgr("Castle");
    mgr.get_entity_mut(PLAYER).unwrap().player_id = Some(0);
    mgr.get_entity_mut(NPC).unwrap().player_id = Some(100);
    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_ID,
        vec![],
        Action::GrantItem {
            item_id: 5029,
            count: 1,
            container_id: None,
        },
    ));
    let (tx, mut rx) = mpsc::channel(8);
    for entity_id in [9999, PLAYER, NPC] {
        fire_guarded_minigame_chain_by_id(CHAIN_ID, entity_id, &engine, &tx, &mut mgr).await;
        assert!(
            rx.try_recv().is_err(),
            "entity {entity_id} emitted GrantItem"
        );
    }
}
