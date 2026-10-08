//! DK-04: replay the four tent flap routes through the seeded engine and
//! the real interact-tag dispatcher. The route's world gate matters because
//! neither the tag trigger nor the chain's documentary scope filters by world.

use std::collections::HashMap;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use tokio::sync::mpsc;

use super::super::engine_loader::build_engine;
use super::super::fire_interact_tag;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::WorldRow;
use crate::test_support::require_db_or_skip;

const PLAYER_EID: u32 = 80_041;
const PLAYER_ID: i32 = 80_042;
const TARGET_EID: u32 = 80_043;

struct Route {
    chain: i64,
    tag: &'static str,
    source_world: &'static str,
    wrong_world: &'static str,
    destination_world: &'static str,
    destination: [f32; 3],
}

const ROUTES: [Route; 4] = [
    Route {
        chain: 8003,
        tag: "Dakara_E1_TentFlap_ToCommand",
        source_world: "Dakara_E1",
        wrong_world: "Dakara_E1_StoryRm",
        destination_world: "Dakara_E1_StoryRm",
        destination: [71.0, 0.05, 30.0],
    },
    Route {
        chain: 8004,
        tag: "Dakara_E1_TentFlap_ToMohkatan",
        source_world: "Dakara_E1",
        wrong_world: "Dakara_E1_StoryRm",
        destination_world: "Dakara_E1_StoryRm",
        destination: [71.0, 0.05, 30.0],
    },
    Route {
        chain: 8005,
        tag: "Dakara_E1_StoryRm_TentFlap_FromCommand",
        source_world: "Dakara_E1_StoryRm",
        wrong_world: "Dakara_E1",
        destination_world: "Dakara_E1",
        destination: [141.0, -20.8, 288.0],
    },
    Route {
        chain: 8006,
        tag: "Dakara_E1_StoryRm_TentFlap_FromMohkatan",
        source_world: "Dakara_E1_StoryRm",
        wrong_world: "Dakara_E1",
        destination_world: "Dakara_E1",
        destination: [141.0, -20.8, 288.0],
    },
];

fn resolve(engine: &ChainEngine, tag: &str, world_id: Option<i32>) -> Vec<(i64, Action)> {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = world_id;
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx).actions
}

fn staged_player(world: &str) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Dakara_E1" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" />
        <Space WorldName="Dakara_E1_StoryRm" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Dakara_E1" />
        <Space WorldName="Dakara_E1_StoryRm" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.stamp_world_rows(&HashMap::from([
        ("Dakara_E1".to_string(), WorldRow::enforcing(61)),
        ("Dakara_E1_StoryRm".to_string(), WorldRow::enforcing(62)),
    ]));
    mgr.create_entity(PLAYER_EID, world, [100.0, -17.4, 230.0], [0.0; 3])
        .unwrap();
    let player = mgr.get_entity_mut(PLAYER_EID).unwrap();
    player.is_player = true;
    player.player_id = Some(PLAYER_ID);
    mgr.connect_entity(PLAYER_EID);
    mgr
}

#[tokio::test]
async fn live_db_each_tent_flap_replays_once_and_only_on_its_own_world() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for route in ROUTES {
        let source_id = if route.source_world == "Dakara_E1" {
            61
        } else {
            62
        };
        assert_eq!(
            resolve(&engine, route.tag, Some(source_id)),
            vec![(
                route.chain,
                Action::CrossWorldTeleport {
                    world_name: route.destination_world.to_string(),
                    position: route.destination,
                },
            )],
            "{} must resolve exactly its own route",
            route.tag
        );
        let wrong_id = if source_id == 61 { 62 } else { 61 };
        assert!(
            resolve(&engine, route.tag, Some(wrong_id)).is_empty(),
            "{} must not resolve in the wrong world",
            route.tag
        );
        assert!(
            resolve(&engine, route.tag, None).is_empty(),
            "{} must fail closed without world context",
            route.tag
        );

        let mut mgr = staged_player(route.source_world);
        let (tx, mut rx) = mpsc::channel(8);
        assert!(
            fire_interact_tag(
                PLAYER_EID, PLAYER_ID, route.tag, TARGET_EID, &engine, &tx, &mut mgr,
            )
            .await,
            "{} must execute through the real dispatcher",
            route.tag
        );
        let msg = rx.try_recv().expect("one transfer request");
        assert!(rx.try_recv().is_err(), "one flap sends one transfer");
        match msg {
            CellToBaseMsg::GateTravel {
                entity_id,
                target_world_name,
                position,
                ..
            } => {
                assert_eq!(entity_id, PLAYER_EID);
                assert_eq!(target_world_name, route.destination_world);
                assert_eq!(position, route.destination);
            }
            other => panic!("{} sent {other:?} instead of GateTravel", route.tag),
        }
        assert!(
            mgr.get_entity(PLAYER_EID).is_none(),
            "cell handoff removes the player"
        );

        let mut wrong_mgr = staged_player(route.wrong_world);
        let (wrong_tx, mut wrong_rx) = mpsc::channel(8);
        assert!(
            !fire_interact_tag(
                PLAYER_EID,
                PLAYER_ID,
                route.tag,
                TARGET_EID,
                &engine,
                &wrong_tx,
                &mut wrong_mgr,
            )
            .await,
            "{} must not execute from the wrong world",
            route.tag
        );
        assert!(wrong_rx.try_recv().is_err());
        assert!(wrong_mgr.get_entity(PLAYER_EID).is_some());
    }
}
