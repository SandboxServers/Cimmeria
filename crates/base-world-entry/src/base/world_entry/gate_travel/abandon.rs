//! Last-resort session teardown for a gate transfer that cannot complete.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use super::super::super::session_identity;
use super::super::super::ConnectedClientState;
use super::session_owner;
use crate::cell::messages::BaseToCellMsg;

/// Last-resort teardown for a transfer that cannot be completed *after* the
/// cell has already removed the entity from its origin space.
///
/// At that point the player's entity exists in no space: their position
/// updates are dropped as `EntityMissing`, nobody can see them, and nothing
/// on this side can put them back — the base is never told the origin space
/// id, only the destination. Leaving the session alive would strand them
/// permanently, so the session is ended and a reconnect rebuilds the player
/// from the DB.
///
/// This is deliberately *not* `destroy_client_entities`: that helper needs the
/// `EntityManager`, which the cell-dispatch chain does not carry. The
/// consequence is that this path leaks the account/player entity ids instead
/// of returning them to the free list. On an already-catastrophic branch a
/// leaked id (which simply never gets recycled) is much cheaper than the
/// alternative, and it cannot strand anybody.
pub(super) async fn abandon_unspaced_session(
    addr: SocketAddr,
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    // Snapshot the names before the session is removed below (Rule 6).
    let mut id = cimmeria_entity::cell_entity::PlayerIdentity::UNKNOWN;
    if let Ok(mut clients) = connected.lock() {
        if let Some(c) = clients.get(&addr) {
            // A relaunched client may have taken the address over while the
            // transfer was in flight; its session is not ours to end.
            if !session_owner::owns(c, entity_id) {
                session_owner::log_replaced(addr, entity_id, "abandon_unspaced_session", c);
                return;
            }
            id = session_identity::session_identity(c);
            // Stop the tick-sync loop before the session goes, same as every
            // other teardown path.
            c.cancelled.store(true, Ordering::Relaxed);
            cimmeria_base_session::base::player_index::log_unlisted(addr, c, "gate_travel_abandon");
            cimmeria_base_session::base::deferred_aoi::log_discarded_on_teardown(
                addr,
                c,
                "gate_travel_abandon",
            );
        }
        clients.remove(&addr);
    }
    // Only drop the reverse mapping if it still points at *this* session —
    // the id may already have been recycled to somebody else.
    {
        let mut map = entity_to_addr.lock().unwrap();
        if map.get(&entity_id) == Some(&addr) {
            map.remove(&entity_id);
        }
    }
    if let Some(tx) = cell_tx {
        // This path has no `EntityManager` handle (see the function doc: it
        // deliberately leaks the account/player entity ids rather than
        // freeing them), so there is no free-list return to gate on the
        // cell's teardown ack -- drop the receiver rather than await it.
        let (reply_tx, _reply_rx) = tokio::sync::oneshot::channel();
        if let Err(e) = tx
            .send(BaseToCellMsg::DisconnectEntity {
                entity_id,
                reply_tx,
            })
            .await
        {
            tracing::error!(
                entity_id,
                entity_name = id.player_name,
                account_id = id.account_id,
                account_name = id.account_name,
                player_id = id.player_id,
                player_name = id.player_name,
                %addr,
                "GateTravel: DisconnectEntity send failed while abandoning an \
                 un-spaced session ({e}) — cell may keep stale player state"
            );
        }
    }
    tracing::warn!(
        entity_id,
        entity_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        %addr,
        "GateTravel: session ended after an unrecoverable transfer abort — \
         the client must reconnect"
    );
}
