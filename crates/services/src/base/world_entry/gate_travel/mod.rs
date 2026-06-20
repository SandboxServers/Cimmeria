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

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_reset_entities, WorldEntryInfo, SGWPLAYER_CLASS_ID};

use super::super::ConnectedClientState;
use super::methods::{query_player_load_data, query_world_stargates};
use super::space_registry::resolve_space_id_fallback;

#[cfg(test)]
mod tests;

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
    fields(entity_id, target_world_name, destination_ring_id)
)]
pub(crate) async fn handle_gate_travel(
    entity_id: u32,
    target_world_name: &str,
    position: [f32; 3],
    rotation: [f32; 3],
    destination_ring_id: Option<i32>,
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

    // Persist the destination world + position to sgw_player so a future
    // relog or RespawnReload reloads the player at the new world rather than
    // snapping them back to the saved pre-gate location.
    if let Some(pool) = db_pool {
        // Look up active_player_id (cached from playCharacter) — fall back to
        // lowest-for-account only if missing, to keep gate travel functional
        // on accounts that somehow skipped the playCharacter cache.
        let active_pid: Option<i32> = {
            let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
            clients.get(&addr).and_then(|c| c.active_player_id)
        };

        // Fail closed: gate travel without a known active character would
        // otherwise persist against a fallback (e.g., MIN(player_id) for the
        // account) that could corrupt a different character on multi-character
        // accounts. The cache is set in play_character; missing here is a
        // protocol-level error, not something to paper over.
        let pid = match active_pid {
            Some(pid) => pid,
            None => {
                tracing::error!(
                    %addr, account_id, world = %target_world_name,
                    "GateTravel: no active_player_id cached — refusing to persist destination (would risk wrong-character corruption on multi-character accounts)"
                );
                return Ok(());
            }
        };

        let res = sqlx::query(
            "UPDATE sgw_player \
               SET world_location = $1, \
                   world_id = COALESCE((SELECT world_id FROM resources.worlds WHERE world = $1), world_id), \
                   pos_x = $2, pos_y = $3, pos_z = $4 \
             WHERE player_id = $5 AND account_id = $6",
        )
        .bind(target_world_name)
        .bind(position[0]).bind(position[1]).bind(position[2])
        .bind(pid).bind(account_id as i32)
        .execute(pool.as_ref()).await;

        match res {
            Ok(r) if r.rows_affected() == 0 => {
                tracing::warn!(%addr, account_id, world = %target_world_name, "GateTravel: persistence UPDATE matched 0 rows");
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(%addr, account_id, world = %target_world_name, "GateTravel: failed to persist destination: {e}");
            }
        }
    }

    // Query stargates for the destination world (Bug #3: load stargate cache for new world)
    let world_stargates = query_world_stargates(db_pool, target_world_name).await;

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
    // Fail closed: a missing active_player_id means we can't safely identify
    // which character to reload — falling back to "lowest player_id for the
    // account" would silently load the wrong character on multi-character
    // accounts. The cache is set in play_character; missing here is a
    // protocol-level error.
    let active_player_id: i32 = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        match clients.get(&addr).and_then(|c| c.active_player_id) {
            Some(pid) => pid,
            None => {
                tracing::error!(
                    %addr, account_id,
                    "GateTravel: no active_player_id cached — aborting reload (would risk loading wrong character on multi-character accounts)"
                );
                return Ok(());
            }
        }
    };
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
        exit_name.unwrap_or_else(|| "<unknown>".to_string()),
        exit_from_world.unwrap_or_else(|| "<unknown>".to_string()),
        Some(target_world_name.to_string()),
    );

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
