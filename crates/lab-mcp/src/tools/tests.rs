//! Focused rmcp adapter behavior tests.

use std::sync::Arc;

use cimmeria_admin_api::ws::broadcast_layer::LogBuffer;
use cimmeria_services::cell::messages::{
    BaseToCellMsg, LabEntitySnapshot, LabQuery, LabQueryReply,
};
use cimmeria_services::orchestrator::Orchestrator;
use tokio::sync::mpsc;
use tracing::Level;

use super::*;
use crate::test_support::LogCapture;

#[tokio::test]
async fn console_list_adapter_refuses_live_non_gm_snapshot() {
    let orchestrator = Arc::new(Orchestrator::new(Default::default()));
    let (tx, mut rx) = mpsc::channel(1);
    orchestrator.state().write().await.cell_tx = Some(tx);
    let tools = LabTools::new(LabState::new(orchestrator, LogBuffer::new()));
    let responder = tokio::spawn(async move {
        let BaseToCellMsg::LabQuery { query, reply_tx } = rx.recv().await.unwrap() else {
            panic!("catalog must query current cell authority");
        };
        assert!(matches!(query, LabQuery::EntityGet { entity_id: 42 }));
        let entity: LabEntitySnapshot = serde_json::from_value(json!({
            "entity_id": 42, "space_id": 3, "world": "Castle_CellBlock",
            "world_name": "Castle_CellBlock", "position": [0.0, 0.0, 0.0],
            "direction": [0.0, 0.0, 0.0], "velocity": [0.0, 0.0, 0.0],
            "is_on_ground": true, "is_player": true, "class_id": 2,
            "faction": 0, "alignment": 0, "level": 5, "name": null,
            "template_id": null, "spawn_id": null, "tag": null,
            "name_id": null, "archetype_id": null, "access_level": 0,
            "ai_state": "Idle", "current_target_id": null, "aoi_radius": 100.0,
            "state_field": 0, "interaction_type_flags": 0,
            "weapon_holstered": true, "has_static_mesh": false,
            "component_count": 0, "witness_count": 0,
            "health_cur": 10, "health_max": 10
        }))
        .unwrap();
        reply_tx
            .send(Ok(LabQueryReply::Entity {
                entity: Some(entity),
            }))
            .unwrap();
    });
    let result = tools
        .server_console_list(Parameters(ConsoleListArgs { entity_id: 42 }))
        .await
        .unwrap();
    responder.await.unwrap();
    assert_eq!(result.is_error, Some(true));
    assert_eq!(result.content[0].as_text().unwrap().text, "not_gm");
}

#[tokio::test]
async fn minigame_state_tool_tracks_generated_board_update_and_teardown() {
    let orchestrator = Arc::new(Orchestrator::new(Default::default()));
    let tools = LabTools::new(LabState::new(orchestrator.clone(), LogBuffer::new()));
    assert!(tools.tool_router.has_route("server_minigame_state"));
    async fn read(tools: &LabTools) -> Value {
        let result = tools
            .server_minigame_state(Parameters(EntityGetArgs { entity_id: 42 }))
            .await
            .unwrap();
        serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap()
    }
    assert!(read(&tools).await["state"].is_null());
    let registry = orchestrator
        .state()
        .read()
        .await
        .base
        .minigame_registry
        .clone();
    let ticket = registry
        .register(
            42,
            1,
            "Livewire".into(),
            1,
            30,
            17,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            None,
            1,
        )
        .await
        .unwrap();
    let session = registry
        .authenticate_and_claim(42, &ticket, "Livewire")
        .await
        .unwrap();
    let mut game = cimmeria_services::minigame::game::create_game(&session).unwrap();
    game.started();
    registry
        .publish_board(42, &ticket, game.board_snapshot().unwrap())
        .await;
    let ready = read(&tools).await;
    assert_eq!(ready["state"]["state"], "ready");
    assert_eq!(ready["state"]["goal_total"], 2);
    assert!(ready["state"]["wires"].as_array().unwrap().len() > 2);
    assert!(ready["state"]["wires"][0]["lib"].is_string());
    assert!(!ready.to_string().contains(&ticket));
    game.message("opendoor", &Default::default());
    registry
        .publish_board(42, &ticket, game.board_snapshot().unwrap())
        .await;
    assert_eq!(read(&tools).await["state"]["state"], "running");
    game.aborted();
    registry
        .publish_board(42, &ticket, game.board_snapshot().unwrap())
        .await;
    registry
        .record_event(42, &ticket, "result_canceled", None)
        .await;
    registry.remove_if_ticket(42, &ticket).await;
    let final_state = read(&tools).await;
    assert_eq!(final_state["state"]["outcome"], "canceled");
    assert!(final_state["state"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event"] == "ended"));
}

#[test]
fn nav_path_is_routed_with_a_described_contract() {
    let orchestrator = Arc::new(Orchestrator::new(Default::default()));
    let tools = LabTools::new(LabState::new(orchestrator, LogBuffer::new()));
    let route = tools.tool_router.get("server_nav_path").expect("routed");
    assert!(route
        .description
        .as_deref()
        .unwrap_or_default()
        .contains("partial"));
}

/// A bad or failed SQL call leaves one WARN `lab.tool_call` row carrying
/// `error` and `duration_ms`. No database is connected here, so the call
/// fails on the pool check, which takes the same error path as bad SQL.
#[tokio::test]
async fn bad_sql_audit_row_has_error() {
    let capture = LogCapture::install();
    let orchestrator = Arc::new(Orchestrator::new(Default::default()));
    let tools = LabTools::new(LabState::new(orchestrator, LogBuffer::new()));
    let result = tools
        .server_db_query(Parameters(DbQueryArgs {
            sql: "SELEC nonsense".to_string(),
        }))
        .await;
    assert!(
        result.is_ok(),
        "a failed query is a tool-level error result"
    );

    let event = capture
        .find_message(Level::WARN, "lab MCP tool call")
        .expect("an error outcome logs at WARN");
    assert!(event
        .fields
        .get("tool")
        .is_some_and(|t| t.contains("server_db_query")));
    assert!(event
        .fields
        .get("outcome")
        .is_some_and(|o| o.contains("error")));
    assert!(event.fields.contains_key("duration_ms"));
    assert!(event
        .fields
        .get("error")
        .is_some_and(|e| e.contains("database not connected")));
}
