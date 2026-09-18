//! Base-side resolution of the stable `(account_id, player_id)` log
//! correlator for an entity.
//!
//! This is the base counterpart to
//! [`SpaceManager::player_identity`](crate::cell::space_manager::SpaceManager::player_identity).
//! The cell can read identity straight off its `CellEntity`; the base has to
//! go `entity_id → SocketAddr → ConnectedClientState`, because the session —
//! not the entity — is where `account_id` lives.
//!
//! See `docs/architecture/instrumentation-discipline.md` §Rule 5 for why
//! `entity_id` alone is not an identity and why both fields are emitted as
//! `Option`s.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::cell_entity::PlayerIdentity;

use super::ConnectedClientState;

/// Resolve the identity of the player who owns `entity_id`.
///
/// Two lookup strategies, in order:
///
/// 1. **`entity_to_addr` → `connected`** — the normal path, the same two-step
///    every cell→base handler already uses to find a client's socket.
/// 2. **Scan `connected` by `player_entity_id`** — fallback for when the
///    reverse mapping is missing. That is not a hypothetical: the AoI
///    "unmapped witness" warning fires *precisely* because step 1 failed, and
///    it is the one log line where knowing the account matters most (it marks
///    an entity that stays invisible to that player until they relog). The
///    session itself usually still exists, so the scan recovers what the
///    missing mapping lost.
///
/// The scan is O(connected) and only runs when step 1 misses. Call this from
/// warn/error paths and one-shot lifecycle events — not from a per-packet hot
/// path.
///
/// Returns [`PlayerIdentity::UNKNOWN`] when neither strategy resolves, which
/// makes the caller emit no identity fields at all. A poisoned lock is
/// treated the same way: an observability helper must never panic or block
/// the path it is only trying to describe.
pub(crate) fn identity_for_entity(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    entity_id: u32,
) -> PlayerIdentity {
    let addr = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied());

    let Ok(clients) = connected.lock() else {
        return PlayerIdentity::UNKNOWN;
    };

    if let Some(c) = addr.and_then(|a| clients.get(&a)) {
        return PlayerIdentity::new(Some(c.account_id), c.active_player_id);
    }

    // Fallback: find the session that claims this entity.
    clients
        .values()
        .find(|c| c.player_entity_id == Some(entity_id))
        .map_or(PlayerIdentity::UNKNOWN, |c| {
            PlayerIdentity::new(Some(c.account_id), c.active_player_id)
        })
}
