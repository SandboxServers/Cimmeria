//! Gate-travel: replay the world-entry flow against a new destination space.
//!
//! Triggered by `CellToBaseMsg::GateTravel`. Sends RESET_ENTITIES to tear down
//! the client's view, persists the destination world+position, and seeds
//! `pending_world_entry` so the client's next ENABLE_ENTITIES drives a fresh
//! create-player + enter-world cycle.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::contact_list::handlers::fanout_contact_event;
use crate::base::contact_list::wire::EVENT_GATE_TRAVEL;
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_reset_entities, WorldEntryInfo, SGWPLAYER_CLASS_ID};

use super::super::ConnectedClientState;
use super::methods::{query_player_load_data, query_world_stargates};
use super::space_registry::resolve_space_id_fallback;

mod address_grant;
mod persist_arrival;
pub(crate) use address_grant::handle_grant_stargate_address;
use persist_arrival::persist_arrival;

#[cfg(test)]
mod tests;

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
async fn abandon_unspaced_session(
    addr: SocketAddr,
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    if let Ok(mut clients) = connected.lock() {
        if let Some(c) = clients.get(&addr) {
            // Stop the tick-sync loop before the session goes, same as every
            // other teardown path.
            c.cancelled.store(true, Ordering::Relaxed);
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
        if let Err(e) = tx.send(BaseToCellMsg::DisconnectEntity { entity_id }).await {
            tracing::error!(
                entity_id, %addr,
                "GateTravel: DisconnectEntity send failed while abandoning an \
                 un-spaced session ({e}) — cell may keep stale player state"
            );
        }
    }
    tracing::warn!(
        entity_id, %addr,
        "GateTravel: session ended after an unrecoverable transfer abort — \
         the client must reconnect"
    );
}

/// Handle a gate travel request from CellService.
///
/// This re-uses the world entry flow (teardown -> create player -> enter world):
/// 1. Send RESET_ENTITIES to tear down the client entity system.
/// 2. Set up pending world entry for the new world (reusing same entity_id).
/// 3. Client responds with ENABLE_ENTITIES -> create-player + enter-world steps send the new world packets.
///
/// The CellService has already removed the entity from its old space.
/// We tell it to create the entity in the new space, then send the client
/// the full world-entry + mapLoaded sequence for the destination.
#[tracing::instrument(
    name = "gate_travel.execute",
    level = "info",
    skip_all,
    fields(
        entity_id,
        target_world_name,
        destination_ring_id,
        destination_space_id
    )
)]
pub(crate) async fn handle_gate_travel(
    entity_id: u32,
    target_world_name: &str,
    position: [f32; 3],
    rotation: [f32; 3],
    destination_ring_id: Option<i32>,
    // Exact destination instance from `CellToBaseMsg::GateTravel`. `None`
    // keeps the historical "resolve by world name" behavior.
    destination_space_id: Option<u32>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Look up client transport from entity_id
    let addr = entity_to_addr
        .lock()
        .unwrap()
        .get(&entity_id)
        .copied()
        .ok_or("Gate travel: no client addr for entity")?;

    // Get client state. Also snapshot the name + current world for the
    // Discord world-exit emit before they're overwritten by the new world.
    let (
        key,
        enc_version,
        account_id,
        account_name,
        _access_level,
        pending_acks_arc,
        next_seq,
        exit_name,
        exit_from_world,
    ) = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients
            .get(&addr)
            .ok_or("Gate travel: client state not found")?;
        (
            c.key,
            c.enc_version,
            c.account_id,
            c.account_name.clone(),
            c.access_level,
            Arc::clone(&c.pending_acks),
            Arc::clone(&c.next_seq),
            c.player_name.clone(),
            c.world_name.clone(),
        )
    };

    tracing::info!(
        entity_id, %addr, world = %target_world_name,
        "Gate travel: sending RESET_ENTITIES for world transition"
    );

    // Fail closed BEFORE anything destructive. Gate travel without a known
    // active character can neither persist the destination (it would risk
    // writing another character's row on a multi-character account) nor
    // reload the right character, so the whole transfer is refused here.
    //
    // This check used to sit *after* the `CreateEntity` round-trip below.
    // That ordering is not "wrong and this is right" — it is a different
    // failure shape: the old one aborted with the entity already in the
    // DESTINATION space (present but desynced), this one aborts with the
    // entity in NO space, because the cell half of the flow removed it from
    // its origin before we were called. Neither is recoverable in place and
    // neither corrupts the DB; checking first is still the right call because
    // `CreateEntity` is the point of no return on this side and there is no
    // reason to cross it when we already know the transfer must fail.
    //
    // Because an aborted transfer leaves a live client bound to an entity
    // that is in no space at all — every position update it sends would be
    // dropped as `EntityMissing`, invisible to everyone, forever — the abort
    // ends the session so a reconnect rebuilds the player cleanly. That is
    // the only recovery available at this seam: the base cannot re-create the
    // entity because it was never told the origin space id.
    let active_player_id: i32 = {
        let maybe_pid = {
            let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
            clients.get(&addr).and_then(|c| c.active_player_id)
        };
        match maybe_pid {
            Some(pid) => pid,
            None => {
                tracing::error!(
                    %addr, account_id, world = %target_world_name,
                    "GateTravel: no active_player_id cached — refusing the transfer \
                     (would risk wrong-character corruption / loading the wrong character \
                     on multi-character accounts); ending the session because the entity \
                     is already out of its origin space"
                );
                abandon_unspaced_session(addr, entity_id, connected, entity_to_addr, cell_tx).await;
                return Ok(());
            }
        }
    };

    // Tell CellService to create the entity in the new space and await the
    // resolved space_id via oneshot (needed for the world-entry wire packet).
    let space_id = if let Some(tx) = cell_tx {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        if tx
            .send(BaseToCellMsg::CreateEntity {
                entity_id,
                world_name: target_world_name.to_string(),
                position,
                rotation,
                destination_space_id,
                // Gate travel destroys the origin entity and builds a fresh
                // one here, so without this stamp the destination entity would
                // be anonymous until `InitPlayerState` re-arrives after the
                // client finishes loading — the same gap `character_name`
                // still has. Both halves are already validated above.
                account_id: Some(account_id),
                player_id: Some(active_player_id),
                reply_tx,
            })
            .await
            .is_ok()
        {
            match reply_rx.await {
                Ok(sid) => sid,
                Err(_) => {
                    tracing::warn!(world = %target_world_name, "Gate travel: CellService oneshot dropped -- using fallback");
                    resolve_space_id_fallback(target_world_name)
                }
            }
        } else {
            resolve_space_id_fallback(target_world_name)
        }
    } else {
        resolve_space_id_fallback(target_world_name)
    };

    // Disconnect-during-transfer reap. The cell already removed the entity
    // from its origin space, and the `CreateEntity` above put it back in the
    // destination one. If the client dropped while that round-trip was in
    // flight, `destroy_client_entities` has already pulled its
    // `entity_to_addr` mapping and queued its own `DisconnectEntity` — which
    // the cell may well have processed *before* our `CreateEntity`, leaving a
    // clientless ghost entity sitting in the destination space forever. Reap
    // it explicitly instead of leaking it.
    //
    // "Mapped to a different addr" is NOT the same as "unmapped", and only the
    // latter is safe to reap. `EntityManager::allocate_id` recycles ids from a
    // free list, and `destroy_client_entities` frees ours on the way out — so a
    // second client can legitimately be handed this exact id and re-register it
    // while we are still blocked on the oneshot. Reaping then would destroy a
    // live player's entity, which is far worse than leaking a dead one. Bail
    // loudly instead and let the new session keep its entity.
    if cell_tx.is_some() {
        let mapped_addr = entity_to_addr.lock().unwrap().get(&entity_id).copied();
        if mapped_addr.is_some() && mapped_addr != Some(addr) {
            tracing::error!(
                entity_id, %addr, ?mapped_addr, world = %target_world_name,
                "GateTravel: entity id was recycled to another session mid-transfer — \
                 abandoning the transfer WITHOUT reaping (destroying this entity now \
                 would strand the live player who owns the id)"
            );
            return Ok(());
        }
        let addr_still_mapped = mapped_addr == Some(addr);
        let session_still_open = connected
            .lock()
            .map_err(|_| "connected lock poisoned")?
            .contains_key(&addr);
        if !addr_still_mapped || !session_still_open {
            tracing::warn!(
                entity_id, %addr, world = %target_world_name,
                addr_still_mapped, session_still_open,
                "GateTravel: client disconnected mid-transfer — destroying the \
                 freshly-created destination entity so it doesn't leak"
            );
            if let Some(tx) = cell_tx {
                if let Err(e) = tx.send(BaseToCellMsg::DestroyEntity { entity_id }).await {
                    tracing::error!(
                        entity_id, world = %target_world_name,
                        "GateTravel: DestroyEntity send failed after mid-transfer \
                         disconnect ({e}) — ghost entity left in the destination space"
                    );
                }
            }
            return Ok(());
        }
    }

    // Query stargates for the destination world (Bug #3: load stargate cache for new world)
    let world_stargates = query_world_stargates(db_pool, target_world_name).await;

    // Persist the destination world, position and the newly learned stargate
    // addresses in one statement, so a future relog or RespawnReload reloads
    // the player at the new world rather than snapping them back to the saved
    // pre-gate location. `active_player_id` was resolved (fail-closed) before
    // the teardown above.
    //
    // Ordering matters twice over: this must run *after* the mid-transfer
    // abort branches (nothing is written for a transfer that never happened)
    // and *before* `query_player_load_data` below, which is what fills the
    // `setupStargateInfo` address list the client is about to be handed. Get
    // that second one wrong and the client renders an address book one hop
    // out of date while the cell enforces the current one.
    //
    // Only the *destination* list is passed. The origin half — the half of
    // the unlock rule that actually does any work — is resolved inside the
    // statement from the row's own pre-update `world_location`, which is the
    // only source that stays correct across consecutive hops.
    persist_arrival(
        db_pool,
        active_player_id,
        account_id,
        target_world_name,
        position,
        &world_stargates,
    )
    .await;

    // Build the world entry info for the new destination
    let entry_info = WorldEntryInfo {
        player_entity_id: entity_id,
        space_id,
        pos: position,
        rot: rotation,
        world_name: target_world_name.to_string(),
        class_id: SGWPLAYER_CLASS_ID, // See NOTE above -- SGWGmPlayer shifts method indices
        world_stargates,
    };

    // Query player load data from DB (same player, different world).
    // `active_player_id` is the fail-closed value resolved before teardown:
    // falling back to "lowest player_id for the account" would silently load
    // the wrong character on multi-character accounts.
    let player_load_data = query_player_load_data(db_pool, account_id, active_player_id).await;

    // Entity teardown: Send RESET_ENTITIES
    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
    };
    let seq = next_seq.fetch_add(1, Ordering::Relaxed) & cimmeria_mercury::packet::SEQUENCE_MASK;
    let pkt = build_reset_entities(&key, seq, &acks, enc_version);
    transport.send_to(&pkt, addr).await?;
    // RESET_ENTITIES is one-shot state — kicks off the cross-world
    // handoff. Channel retransmit covers loss.
    crate::base::helpers::shadow_register_reliable_send(
        connected,
        addr,
        seq,
        cimmeria_mercury::packet::Bytes::copy_from_slice(&pkt),
    );

    // Discord world-channel: emit only here, once the transition is
    // committed — past the active_player_id fail-closed early-returns and
    // the RESET_ENTITIES send (which `?`-returns on a send error). Firing it
    // at the top of the handler would post a false "world exit" whenever
    // gate travel aborts. `from_world` is the session's last world; `to_world`
    // is the gate destination. (Snapshotted above before the new world
    // overwrites the connected state.)
    cimmeria_discord::emit_player_world_exit(
        account_id,
        account_name,
        exit_name.clone().unwrap_or_else(|| "<unknown>".to_string()),
        exit_from_world.unwrap_or_else(|| "<unknown>".to_string()),
        Some(target_world_name.to_string()),
    );

    // Contact-list GateTravel fanout (CM 89, eventId=GateTravel).
    //
    // data_value = the destination world_id from `resources.worlds`. The client
    // passes this value to `getWorldInfo(value).Name` to display the world name.
    // We use the same table the gate_travel persistence UPDATE already uses for
    // COALESCE world_id lookup — this is the canonical source. If the client's
    // getWorldInfo index space differs from resources.worlds.world_id, confirm
    // via send-and-observe in playtest and adjust the lookup accordingly.
    //
    // Fire-and-forget: spawned so the pending_world_entry store below (which
    // drives the client's create-player step) is not delayed by the DB lookup.
    if let Some(traveler_name) = exit_name {
        let db_pool_clone = db_pool.clone();
        let transport_clone = Arc::clone(transport);
        let connected_clone = Arc::clone(connected);
        let entity_to_addr_clone = Arc::clone(entity_to_addr);
        let dest_world = target_world_name.to_string();
        tokio::spawn(async move {
            // Resolve the numeric world_id the client expects.
            let world_id: i32 = if let Some(pool) = &db_pool_clone {
                sqlx::query_scalar::<_, i32>(
                    "SELECT world_id FROM resources.worlds WHERE world = $1",
                )
                .bind(&dest_world)
                .fetch_optional(pool.as_ref())
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        world = %dest_world,
                        "GateTravel fanout: world_id lookup failed: {e}"
                    );
                    None
                })
                .unwrap_or(0)
            } else {
                0
            };

            fanout_contact_event(
                &traveler_name,
                EVENT_GATE_TRAVEL,
                world_id,
                &db_pool_clone,
                &transport_clone,
                &connected_clone,
                &entity_to_addr_clone,
            )
            .await;
        });
    }

    // Store pending world entry for the create-player step (ENABLE_ENTITIES handler)
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_player_entity_id = Some(entity_id);
            c.pending_world_entry = Some(entry_info);
            c.pending_player_load_data = Some(player_load_data);
            c.pending_client_ready = None;
            // Carry the cross-world ring transport id forward — consumed in
            // `world_entry_appearance::handle_client_ready` once the
            // destination world signals `onClientReady`. Stays None for
            // stargate-driven gate travel (the `Effect::TeleportCrossWorld`
            // dispatcher is the only producer).
            c.pending_destination_ring_id = destination_ring_id;
            // world_entry_sent stays true -- we don't reset it, since
            // handle_enable_entities checks pending_player_entity_id
        }
    }

    tracing::info!(
        entity_id, %addr, world = %target_world_name,
        "Gate travel: RESET_ENTITIES sent -- awaiting ENABLE_ENTITIES"
    );

    Ok(())
}
