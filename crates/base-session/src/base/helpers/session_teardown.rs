//! Session teardown: [`destroy_client_entities`] and its owner-checked
//! variant [`destroy_owned_client_entities`].
//!
//! Every way a client session ends (disconnect, logOff, inactivity
//! timeout, send error, duplicate login, relaunch takeover) funnels
//! through here, so the cleanup order and the `disconnect_reason` label
//! live in one place.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cimmeria_common::EntityId;
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::transport::Transport;

use crate::cell::messages::BaseToCellMsg;

use super::super::ConnectedClientState;
use super::unmap_departed_witness;

/// Destroy all entities associated with a disconnecting client and remove it from the map.
///
/// Safe to call multiple times for the same address -- returns silently if the
/// session was already removed (e.g. DISCONNECT handler cleaned up, then the
/// tick-sync inactivity timeout fires on the now-absent session).
///
/// Always sets `cancelled` on the session before removal so the tick-sync loop
/// exits promptly instead of running until the 60-second inactivity timeout.
///
/// `reason` is a short, stable label naming why the disconnect fired
/// (`"client_disconnect"`, `"inactivity_timeout"`, `"send_error"`,
/// `"duplicate_login"`, `"relaunch_takeover"`, `"logoff"`). Pin it across every call site
/// so SigNoz can pivot on `disconnect_reason` to answer "what kind
/// of disconnect am I looking at?" without inferring from message
/// text.
///
/// A session whose character was still in the world (`listed_online`)
/// also tells its contact-list watchers and organizations it went offline
/// (`session_presence::spawn_offline`, on its own task; audit A-35, ORG-06),
/// with `reason` as the `disconnect_reason`.
///
/// The player entity id is **not** returned to `EntityManager`'s free list
/// until the cell confirms it has torn the mirrored cell entity down (its
/// `DisconnectEntity` reply). Freeing it eagerly let a concurrent login
/// recycle the id via `allocate_id`'s FIFO free list before the cell had
/// even seen the disconnect, so the old session's `DisconnectEntity` — sent
/// or still in flight — could land on and destroy the *new* player's cell
/// entity (issue #999). When the cell send fails outright, or the cell
/// drops the reply without confirming teardown, the id is withheld from
/// reuse permanently rather than reused unconfirmed: an unrecycled id costs
/// nothing (the id space is an `i32` counter), a reused one racing a live
/// cell can destroy another player's session.
///
/// The Base→Cell send and the wait for that reply run on a **spawned
/// task**, not inline. This function is called from the base's single UDP
/// receive loop (`client_disconnect`, `duplicate_login`) and from the
/// per-session tick-sync loop (`inactivity_timeout`); awaiting a cell round
/// trip — which itself does a DB write (`persist_last_position`) before
/// replying — inline there would pause packet intake for every connected
/// player whenever the cell is busy or the shared Base→Cell channel is
/// backpressured. Everything that does not depend on the cell's reply (the
/// session-map removal, the Account entity free, the reverse-index removal,
/// the plugins' teardown hook (the crafting-queue drop), the offline-presence
/// fan-out, the Discord emit)
/// still runs synchronously before this function returns.
pub fn destroy_client_entities(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    addr: SocketAddr,
    cell_tx: &Option<tokio::sync::mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    transport: &Arc<dyn Transport>,
    db_pool: &Option<Arc<sqlx::PgPool>>,
    reason: &'static str,
) {
    teardown_session(
        connected,
        entity_manager,
        addr,
        cell_tx,
        entity_to_addr,
        transport,
        db_pool,
        reason,
        None,
    );
}

/// [`destroy_client_entities`], but only when the session at `addr` is
/// still the one whose cancel flag is `owner`. Returns `false`, and
/// touches nothing, when `addr` now belongs to a different session.
///
/// The per-session tick-sync loop calls this for its own teardown
/// (inactivity timeout, send error). A client relaunched on the same
/// address:port replaces the session at that address
/// (`relaunch_takeover`, `crates/base/src/base/login/eviction.rs`). The
/// old loop sees its cancel flag on its next tick, but a loop that had
/// already decided to time out could otherwise run its teardown after the
/// takeover and destroy the *new* session. The owner check runs under the
/// same `connected` lock as the removal, so there is no window between
/// the two.
#[allow(clippy::too_many_arguments)]
pub fn destroy_owned_client_entities(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    addr: SocketAddr,
    cell_tx: &Option<tokio::sync::mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    transport: &Arc<dyn Transport>,
    db_pool: &Option<Arc<sqlx::PgPool>>,
    reason: &'static str,
    owner: &Arc<AtomicBool>,
) -> bool {
    teardown_session(
        connected,
        entity_manager,
        addr,
        cell_tx,
        entity_to_addr,
        transport,
        db_pool,
        reason,
        Some(owner),
    )
}

/// The body of both teardown entry points. `false` when nothing was torn
/// down: the session was already gone, or `owner` is set and the session
/// at `addr` is not the one it names.
#[allow(clippy::too_many_arguments)]
fn teardown_session(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    addr: SocketAddr,
    cell_tx: &Option<tokio::sync::mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    transport: &Arc<dyn Transport>,
    db_pool: &Option<Arc<sqlx::PgPool>>,
    reason: &'static str,
    owner: Option<&Arc<AtomicBool>>,
) -> bool {
    let (
        account_eid,
        player_eid,
        account_id,
        player_name,
        discord_account,
        discord_character,
        session_secs,
        ended,
        plugins,
        identity,
    ) = {
        let mut clients = match connected.lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let Some(c) = clients.get(&addr) else {
            tracing::debug!(%addr, disconnect_reason = reason, "destroy_client_entities: no session, already cleaned up");
            return false;
        };
        if let Some(owner) = owner {
            if !Arc::ptr_eq(&c.cancelled, owner) {
                let identity = crate::base::session_identity::session_identity(c);
                tracing::info!(
                    %addr,
                    disconnect_reason = reason,
                    reason = "session_replaced",
                    account_id = identity.account_id,
                    account_name = identity.account_name,
                    player_id = identity.player_id,
                    player_name = identity.player_name,
                    "destroy_client_entities: the address now belongs to a newer session; leaving it up"
                );
                return false;
            }
        }
        // Signal the tick-sync loop to exit before we remove the session.
        c.cancelled.store(true, Ordering::Relaxed);
        let account_eid = c.account_entity_id;
        let player_eid = c.player_entity_id;
        // Snapshot identity + session length for the Discord disconnect emit
        // before `remove` drops the state.
        let account_id = c.account_id;
        let discord_account = c.discord_account();
        let discord_character = c.discord_character();
        let player_name = c.player_name.clone();
        // The names for this function's lines (Rule 6), taken before the
        // session is removed.
        let identity = crate::base::session_identity::session_identity(c);
        let session_secs = c.connected_at.elapsed().as_secs();
        // Snapshot before `remove`: a character still in the world is
        // announced offline once (a `logOff` already unlisted and announced).
        let ended = match (c.listed_online, c.active_player_id, player_eid) {
            (true, Some(player_id), Some(entity_id)) => {
                Some(crate::base::session_presence::EndedSession {
                    account_id,
                    player_id,
                    entity_id,
                    player_name: player_name.clone(),
                    account_name: identity.account_name,
                })
            }
            _ => None,
        };
        crate::base::player_index::log_unlisted(addr, c, reason);
        crate::base::deferred_aoi::log_discarded_on_teardown(addr, c, reason);
        // The session's plugin registry outlives the session for the
        // disconnect hook below.
        let plugins = clients.remove(&addr).map(|c| c.plugins).unwrap_or_default();
        (
            account_eid,
            player_eid,
            account_id,
            player_name,
            discord_account,
            discord_character,
            session_secs,
            ended,
            plugins,
            identity,
        )
    };

    if account_eid != 0 {
        tracing::debug!(
            %addr,
            account_entity_id = account_eid,
            account_entity_name = identity.account_name,
            "Destroying Account entity"
        );
        // The Account entity has no cell-side mirror, so there is nothing to
        // race: free it immediately.
        entity_manager
            .lock()
            .unwrap()
            .destroy_entity(EntityId(account_eid as i32));
    }
    if let Some(player_eid) = player_eid {
        tracing::debug!(
            %addr,
            player_entity_id = player_eid,
            player_entity_name = identity.player_name,
            "Destroying Player entity"
        );
        tracing::info!(
            target: "session.end",
            %addr,
            entity_id = player_eid,
            entity_name = identity.player_name,
            account_id,
            account_name = identity.account_name,
            player_id = identity.player_id,
            player_name = player_name.as_deref(),
            disconnect_reason = reason,
            session_secs,
            "player session ended"
        );

        // Remove from entity->addr reverse index, and record the witness as
        // departed so the cell's in-flight sends to it log at DEBUG.
        unmap_departed_witness(entity_to_addr, player_eid);

        // The base plugins' teardown (#962 step 5): crafting's queued
        // inductions die with the session here, and nothing they would have
        // consumed is touched.
        plugins.run_session_hook(
            crate::base::plugin::SessionHookPoint::DisconnectAfterEntityUnmapped,
            crate::base::plugin::SessionEvent {
                entity_id: player_eid,
                cause: reason,
            },
        );

        // Every user chat channel this character was in loses it here too:
        // this is the disconnect/timeout/duplicate-login teardown, the
        // counterpart of `handle_log_off`'s own call for the two paths a
        // client-initiated logOff covers. Neither call site fires for gate
        // travel (`base-world-entry/gate_travel`), which reuses the same
        // entity id, so membership survives a world change untouched.
        crate::base::user_channels::user_channel_registry().leave_all(player_eid);

        // Notify CellService to disconnect and destroy the cell entity, and
        // hold `player_eid` out of `EntityManager`'s free list until the
        // cell confirms the teardown finished -- see the function doc and
        // issue #999. Spawned so the caller (the UDP receive loop, or the
        // tick-sync loop) never blocks on the cell's reply.
        match cell_tx {
            Some(tx) => {
                let tx = tx.clone();
                let entity_manager = Arc::clone(entity_manager);
                tokio::spawn(async move {
                    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                    match tx
                        .send(BaseToCellMsg::DisconnectEntity {
                            entity_id: player_eid,
                            reply_tx,
                        })
                        .await
                    {
                        Ok(()) => match reply_rx.await {
                            Ok(()) => {
                                entity_manager
                                    .lock()
                                    .unwrap()
                                    .destroy_entity(EntityId(player_eid as i32));
                            }
                            Err(_) => {
                                tracing::warn!(
                                    entity_id = player_eid,
                                    entity_name = identity.player_name,
                                    account_id,
                                    account_name = identity.account_name,
                                    disconnect_reason = reason,
                                    "destroy_client_entities: cell dropped the \
                                     DisconnectEntity reply without confirming \
                                     teardown -- entity id withheld from reuse; \
                                     the cell entity may be leaked in its space"
                                );
                            }
                        },
                        Err(e) => {
                            tracing::warn!(
                                entity_id = player_eid,
                                entity_name = identity.player_name,
                                account_id,
                                account_name = identity.account_name,
                                disconnect_reason = reason,
                                error = %e,
                                "destroy_client_entities: DisconnectEntity send \
                                 failed -- cell may leak the player's entity in \
                                 its space, and the id is withheld from reuse \
                                 until it does"
                            );
                        }
                    }
                });
            }
            // No cell configured (no-cell test harnesses and the account-only
            // "no character in world yet" teardown): nothing on the other
            // side could be mid-teardown, so free the id immediately.
            None => {
                entity_manager
                    .lock()
                    .unwrap()
                    .destroy_entity(EntityId(player_eid as i32));
            }
        }
    }
    tracing::info!(
        %addr,
        disconnect_reason = reason,
        account_id,
        account_name = identity.account_name,
        account_entity_id = account_eid,
        account_entity_name = identity.account_name,
        player_entity_id = ?player_eid,
        player_entity_name = identity.player_name,
        "Client entities cleaned up"
    );

    if let Some(ended) = ended {
        crate::base::session_presence::spawn_offline(
            ended,
            reason,
            db_pool,
            transport,
            connected,
            entity_to_addr,
        );
    }

    // Discord auth-channel: every teardown path funnels through here, so this
    // is the one place that reports *why* a player dropped. The stable
    // `reason` label maps to a typed `DisconnectReason` for the embed.
    cimmeria_discord::emit_player_disconnect(
        discord_account,
        discord_character,
        addr,
        cimmeria_discord::DisconnectReason::from_label(reason),
        session_secs,
    );
    true
}
