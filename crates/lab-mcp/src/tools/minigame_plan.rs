//! Read the published, bounded minigame event ring without consuming it.

use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::state::LabState;

#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct MinigameTapArgs {
    pub entity_id: u32,
    #[serde(default)]
    pub since_seq: Option<u64>,
}

pub(super) async fn tap_read(state: &LabState, args: &MinigameTapArgs) -> Value {
    let board = state.minigame_board(args.entity_id).await;
    let tap = board.map(|board| {
        json!({
            "state": board.state,
            "outcome": board.outcome,
            "events_dropped": board.events_dropped,
            "events": board.events.into_iter()
                .filter(|row| row.seq > args.since_seq.unwrap_or(0))
                .collect::<Vec<_>>()
        })
    });
    json!({ "entity_id": args.entity_id, "tap": tap })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use cimmeria_admin_api::ws::broadcast_layer::LogBuffer;
    use cimmeria_services::orchestrator::Orchestrator;

    use super::*;

    #[tokio::test]
    async fn tap_filter_keeps_order_and_does_not_consume_rows() {
        let orchestrator = Arc::new(Orchestrator::new(Default::default()));
        let state = LabState::new(orchestrator.clone(), LogBuffer::new());
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
                None,
                None,
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
        registry
            .record_traffic(42, &ticket, "in", "processover", BTreeMap::new())
            .await;
        let args = MinigameTapArgs {
            entity_id: 42,
            since_seq: None,
        };
        let first = tap_read(&state, &args).await;
        let rows = first["tap"]["events"].as_array().unwrap();
        let seq = rows.last().unwrap()["seq"].as_u64().unwrap();
        assert_eq!(rows.last().unwrap()["cmd"], "processover");
        assert_eq!(
            tap_read(&state, &args).await["tap"]["events"],
            first["tap"]["events"]
        );
        let filtered = tap_read(
            &state,
            &MinigameTapArgs {
                entity_id: 42,
                since_seq: Some(seq - 1),
            },
        )
        .await;
        assert_eq!(filtered["tap"]["events"].as_array().unwrap().len(), 1);
        assert_eq!(filtered["tap"]["events"][0]["seq"], seq);
    }
}
