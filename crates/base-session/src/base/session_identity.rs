//! Base-side resolution of the stable `(account_id, player_id)` log
//! correlator for an entity, with the names that pair with it
//! (`account_name`, `player_name`; Rule 6).
//!
//! This is the base counterpart to
//! `cimmeria_services::cell::space_manager::SpaceManager::player_identity`.
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
pub fn identity_for_entity(
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
        return session_identity(c);
    }

    // Fallback: find the session that claims this entity.
    clients
        .values()
        .find(|c| c.player_entity_id == Some(entity_id))
        .map_or(PlayerIdentity::UNKNOWN, session_identity)
}

/// The character name of the player whose entity is `entity_id`, from its
/// session: `None` for an NPC or an unmapped entity.
///
/// One map hop and no fallback scan (unlike [`identity_for_entity`]), so a
/// debug line at AoI volume can name its witness. Call it inside the log
/// macro, where it runs only when the line is on.
pub fn player_name_for_entity(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    entity_id: u32,
) -> Option<&'static str> {
    let addr = entity_to_addr.lock().ok()?.get(&entity_id).copied()?;
    let clients = connected.lock().ok()?;
    let c = clients.get(&addr)?;
    cimmeria_entity::name_intern::intern_opt(c.player_name.as_deref())
}

/// The identity of one session: the Rule 5 IDs and their Rule 6 names
/// (`player_name` is the active character's, `account_name` the login).
/// A name the session doesn't have yet is `None`, so it is left off the line.
pub fn session_identity(c: &ConnectedClientState) -> PlayerIdentity {
    PlayerIdentity::new(Some(c.account_id), c.active_player_id)
        .with_names(c.player_name.as_deref(), c.account_name.as_deref())
}

/// The identity of the session at `addr`, for a line that has the address
/// but not the session in hand. [`PlayerIdentity::UNKNOWN`] when the session
/// is gone or the lock is poisoned.
///
/// Takes the `connected` lock: never call it while holding that lock.
pub fn identity_for_addr(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> PlayerIdentity {
    connected
        .lock()
        .ok()
        .and_then(|clients| clients.get(&addr).map(session_identity))
        .unwrap_or(PlayerIdentity::UNKNOWN)
}

/// The identity of the session playing character `player_id`, for a line
/// that has only the DB id (cell→base persistence messages). A scan of
/// `connected`: log branches only. [`PlayerIdentity::UNKNOWN`] when no
/// session plays it or the lock is poisoned.
///
/// Takes the `connected` lock: never call it while holding that lock (use
/// [`identity_for_player_in`] there).
pub fn identity_for_player(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    player_id: i32,
) -> PlayerIdentity {
    connected.lock().map_or(PlayerIdentity::UNKNOWN, |clients| {
        identity_for_player_in(&clients, player_id)
    })
}

/// [`identity_for_player`] over a map the caller already holds locked.
pub fn identity_for_player_in(
    clients: &HashMap<SocketAddr, ConnectedClientState>,
    player_id: i32,
) -> PlayerIdentity {
    clients
        .values()
        .find(|c| c.active_player_id == Some(player_id))
        .map_or(PlayerIdentity::UNKNOWN, session_identity)
}

/// The `entity_name` for an entity ID on the base (Rule 6): the character
/// name when a session owns the entity. `None` for an NPC, because the base
/// keeps no NPC names (the cell's lines name those), and for an entity with
/// no session, so the field is left off rather than guessed.
///
/// Same cost and locking as [`identity_for_entity`]: log branches only.
pub fn entity_name_for(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    entity_id: u32,
) -> Option<&'static str> {
    identity_for_entity(connected, entity_to_addr, entity_id).player_name
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_default_connected_client_state;

    fn maps(
        state: ConnectedClientState,
        via_reverse_map: bool,
    ) -> (
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) {
        let addr: SocketAddr = "127.0.0.1:20013".parse().unwrap();
        let entity_to_addr = if via_reverse_map {
            HashMap::from([(77, addr)])
        } else {
            HashMap::new()
        };
        (
            Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            Arc::new(Mutex::new(entity_to_addr)),
        )
    }

    fn session() -> ConnectedClientState {
        let mut c = test_default_connected_client_state();
        c.account_id = 6;
        c.account_name = Some("sgc_login".into());
        c.active_player_id = Some(12);
        c.player_name = Some("Teal'c".into());
        c.player_entity_id = Some(77);
        c
    }

    /// Both lookup strategies return the names with the IDs (Rule 6), so a
    /// base line that emits the identity names the player too.
    #[test]
    fn both_strategies_carry_the_session_names() {
        for via_reverse_map in [true, false] {
            let (connected, entity_to_addr) = maps(session(), via_reverse_map);
            let id = identity_for_entity(&connected, &entity_to_addr, 77);
            assert_eq!(id.account_id, Some(6));
            assert_eq!(id.player_id, Some(12));
            assert_eq!(
                id.account_name,
                Some("sgc_login"),
                "via_reverse_map={via_reverse_map}"
            );
            assert_eq!(
                id.player_name,
                Some("Teal'c"),
                "via_reverse_map={via_reverse_map}"
            );
        }
    }

    /// Before character select there is no character name: it is left out,
    /// not written as "".
    #[test]
    fn a_session_without_a_character_has_no_player_name() {
        let mut c = session();
        c.player_name = None;
        c.active_player_id = None;
        let id = session_identity(&c);
        assert_eq!(id.player_name, None);
        assert_eq!(id.account_name, Some("sgc_login"));
    }
}
