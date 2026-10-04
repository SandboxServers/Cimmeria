//! `server_sessions` tool logic — enumerate connected players.
//!
//! Every id carries its name (Rule 6, NT-41): `entity_id` → `entity_name`,
//! `player_id` → `player_name`, `account_id` → `account_name`, and the zone
//! as `world`. A name the session does not hold is left out. `name` and
//! `zone` predate Rule 6 and stay for existing callers (the UAT runner keys
//! on `name`).

use serde_json::{json, Map, Value};

use cimmeria_services::base::OnlinePlayer;

use crate::state::LabState;

/// Snapshot of currently-connected players: entity id, character name,
/// player and account, archetype, level, world, load status, and session
/// address.
pub async fn list_sessions(state: &LabState) -> Value {
    shape(&state.online_players().await)
}

/// The tool's JSON for a roster snapshot.
fn shape(players: &[OnlinePlayer]) -> Value {
    let sessions: Vec<Value> = players.iter().map(session).collect();
    json!({ "count": sessions.len(), "sessions": sessions })
}

fn session(p: &OnlinePlayer) -> Value {
    let character = non_empty(&p.name);
    let mut row = Map::new();
    row.insert("entity_id".into(), json!(p.id));
    put(&mut row, "entity_name", character);
    row.insert("name".into(), json!(p.name));
    if let Some(player_id) = p.player_id {
        row.insert("player_id".into(), json!(player_id));
        put(&mut row, "player_name", character);
    }
    row.insert("account_id".into(), json!(p.account_id));
    put(
        &mut row,
        "account_name",
        p.account_name.as_deref().and_then(non_empty),
    );
    row.insert("archetype".into(), json!(p.archetype));
    row.insert("level".into(), json!(p.level));
    put(&mut row, "world", non_empty(&p.zone));
    row.insert("zone".into(), json!(p.zone));
    row.insert("status".into(), json!(p.status));
    row.insert("session".into(), json!(p.session));
    Value::Object(row)
}

/// Insert `key` only when the name resolved: Rule 6 leaves a missing name
/// out rather than writing a placeholder.
fn put(row: &mut Map<String, Value>, key: &str, name: Option<&str>) {
    if let Some(name) = name {
        row.insert(key.into(), json!(name));
    }
}

fn non_empty(s: &str) -> Option<&str> {
    Some(s).filter(|s| !s.is_empty())
}

#[cfg(test)]
#[path = "sessions_tests.rs"]
mod tests;
