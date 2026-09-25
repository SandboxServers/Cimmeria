//! `server_packet_tap_start` / `_read` / `_stop` tool logic (issue #688).
//!
//! A packet tap captures the decoded Mercury messages for ONE session (named
//! by its player entity id) in both directions, into the bounded per-session
//! ring in [`cimmeria_services::wire_log::tap`]. `start` resolves the session's
//! socket address so the inbound half is bound to the right peer; `read` drains
//! the ring and reports how many messages were dropped (oldest-first) since the
//! previous read; `stop` tears the tap down.

use serde_json::{json, Value};

use cimmeria_services::wire_log::tap;

use crate::state::LabState;

/// Start (or restart) a tap on `entity_id`'s session.
///
/// Resolves the session's socket address from the online-player roster so the
/// inbound half binds to the right peer. Fails if that entity is not a
/// connected in-world player.
pub async fn tap_start(
    state: &LabState,
    entity_id: u32,
    capacity: Option<usize>,
) -> Result<Value, String> {
    let Some(addr) = state.session_addr_for_entity(entity_id).await else {
        return Err(format!(
            "entity {entity_id} is not a connected in-world player (no session address); \
             use server_sessions to list tappable sessions"
        ));
    };
    let replaced = tap::start(entity_id, addr, capacity);
    Ok(json!({
        "entity_id": entity_id,
        "session": addr.to_string(),
        "capacity": capacity.unwrap_or(tap::DEFAULT_CAPACITY).clamp(1, tap::MAX_CAPACITY),
        "replaced_existing": replaced,
    }))
}

/// Drain the tap on `entity_id`: return buffered messages + dropped count.
pub fn tap_read(entity_id: u32) -> Result<Value, String> {
    match tap::read(entity_id) {
        Some(r) => Ok(json!({
            "entity_id": r.entity_id,
            "capacity": r.capacity,
            "count": r.messages.len(),
            "dropped": r.dropped,
            "messages": r.messages,
        })),
        None => Err(format!(
            "no active packet tap for entity {entity_id} (start one with server_packet_tap_start)"
        )),
    }
}

/// Stop the tap on `entity_id`.
pub fn tap_stop(entity_id: u32) -> Result<Value, String> {
    let existed = tap::stop(entity_id);
    Ok(json!({ "entity_id": entity_id, "stopped": existed }))
}
