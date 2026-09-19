//! `server_sessions` tool logic — enumerate connected players.

use serde_json::{json, Value};

use crate::state::LabState;

/// Snapshot of currently-connected players: entity id, character name,
/// archetype, level, zone/space, load status, and session address.
pub async fn list_sessions(state: &LabState) -> Value {
    let players = state.online_players().await;
    let sessions: Vec<Value> = players
        .iter()
        .map(|p| {
            json!({
                "entity_id": p.id,
                "name": p.name,
                "archetype": p.archetype,
                "level": p.level,
                "zone": p.zone,
                "status": p.status,
                "session": p.session,
            })
        })
        .collect();
    json!({ "count": sessions.len(), "sessions": sessions })
}
