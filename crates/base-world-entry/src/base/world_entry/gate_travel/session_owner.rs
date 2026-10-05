//! Whether a gate-travel task still owns the session at its address.
//!
//! `handle_gate_travel` resolves the client's address once, then awaits the
//! cell (`CreateEntity`) and the database before it writes anything back.
//! Since a client killed and relaunched on its fixed UDP port can take its
//! address over (`relaunch_takeover`, `cimmeria-base` `login::relaunch`),
//! the session at that address after an await may be a **new** session.
//! Writing the old character's world entry onto it, removing it, or sending
//! it the old session's `RESET_ENTITIES` would break the relaunched
//! client's login. Each such step checks ownership first.
//!
//! Ownership is "the session's player entity is the one travelling". Gate
//! travel never changes `player_entity_id` (the destination entity reuses
//! the id), and a session from a fresh login has none until its own
//! `playCharacter`, which allocates another id.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use super::super::super::session_identity;
use super::super::super::ConnectedClientState;

/// `true` when `c` is the session whose player entity is `entity_id`.
pub(super) fn owns(c: &ConnectedClientState, entity_id: u32) -> bool {
    c.player_entity_id == Some(entity_id)
}

/// One INFO row for a gate-travel step skipped because the session at
/// `addr` belongs to someone else now. Names the session that holds it.
pub(super) fn log_replaced(
    addr: SocketAddr,
    entity_id: u32,
    step: &'static str,
    holder: &ConnectedClientState,
) {
    let who = session_identity::session_identity(holder);
    tracing::info!(
        %addr,
        entity_id, // nt:id-only the travelling entity's session is gone; the names below are the new holder's
        step,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        reason = "session_replaced",
        "GateTravel: the session at this address is no longer the travelling \
         player's (relaunch takeover?) -- leaving it alone"
    );
}

/// `false` (after logging) when a session other than `entity_id`'s holds
/// `addr`. `true` when the travelling session is still there, or when no
/// session is (the caller's own disconnect handling covers that).
pub(super) fn still_owned(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    entity_id: u32,
    step: &'static str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
    match clients.get(&addr) {
        Some(c) if !owns(c, entity_id) => {
            log_replaced(addr, entity_id, step, c);
            Ok(false)
        }
        _ => Ok(true),
    }
}
