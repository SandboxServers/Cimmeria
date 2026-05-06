//! Respawn reload: replay the *client-side* world entry against the cell's
//! existing space, leaving the cell entity (and instance) untouched.
//!
//! Triggered by `CellToBaseMsg::RespawnReload`. Different from gate-travel:
//! gate-travel destroys+recreates the cell entity to move worlds, which for
//! instanced spaces tears down the entire instance (NPCs, kismet, doors).
//! Respawn must keep the instance alive — the player should come back into
//! the same room they died in, not a fresh copy.
//!
//! What this does:
//!   1. Send `RESET_ENTITIES` so the client destroys the ragdolled pawn.
//!   2. Stage `pending_world_entry` / `pending_player_load_data` against the
//!      existing cell entity_id + space_id (no `BaseToCellMsg::CreateEntity`
//!      round-trip — the cell entity already exists).
//!   3. Set `pending_respawn_reload = true` so `handle_on_client_ready`
//!      knows to skip `BaseToCellMsg::ConnectEntity` + `InitPlayerState` —
//!      both would re-run against an already-initialised cell entity, with
//!      `InitPlayerState` re-loading missions from DB and risking stomping
//!      in-flight state.
//!
//! What the client sees: identical to gate-travel — RESET_ENTITIES, then
//! ENABLE_ENTITIES → CREATE_BASE_PLAYER + onClientMapLoad → (mapLoaded
//! skipped because terrain unchanged, fast-forwarded by
//! `handle_on_client_ready`) → VIEWPORT + CELL + FORCED_POSITION + entity
//! data. Brief loading screen, then back on their feet.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_reset_entities, WorldEntryInfo, SGWPLAYER_CLASS_ID};

use super::super::ConnectedClientState;
use super::methods::{query_player_load_data, query_world_stargates};

/// Handle a respawn reload request from CellService.
pub(crate) async fn handle_respawn_reload(
    entity_id: u32,
    space_id: u32,
    world_name: &str,
    position: [f32; 3],
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    _cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = entity_to_addr
        .lock()
        .unwrap()
        .get(&entity_id)
        .copied()
        .ok_or("Respawn reload: no client addr for entity")?;

    let (key, account_id, pending_acks_arc, next_seq) = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients
            .get(&addr)
            .ok_or("Respawn reload: client state not found")?;
        (
            c.key,
            c.account_id,
            Arc::clone(&c.pending_acks),
            Arc::clone(&c.next_seq),
        )
    };

    // Persist destination position to sgw_player so a relog/crash mid-respawn
    // brings the player back at the spawn point, not at their corpse. Mirrors
    // the gate-travel persistence step.
    if let Some(pool) = db_pool {
        let active_pid: Option<i32> = {
            let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
            clients.get(&addr).and_then(|c| c.active_player_id)
        };

        let pid = match active_pid {
            Some(pid) => pid,
            None => {
                tracing::error!(
                    %addr, account_id, world = %world_name,
                    "RespawnReload: no active_player_id cached — refusing to persist spawn point (would risk wrong-character corruption on multi-character accounts)"
                );
                return Ok(());
            }
        };

        let res = sqlx::query(
            "UPDATE sgw_player \
               SET pos_x = $1, pos_y = $2, pos_z = $3 \
             WHERE player_id = $4 AND account_id = $5",
        )
        .bind(position[0])
        .bind(position[1])
        .bind(position[2])
        .bind(pid)
        .bind(account_id as i32)
        .execute(pool.as_ref())
        .await;

        if let Err(e) = res {
            tracing::error!(%addr, account_id, world = %world_name, "RespawnReload: failed to persist spawn point: {e}");
        }
    }

    let world_stargates = query_world_stargates(db_pool, world_name).await;

    let entry_info = WorldEntryInfo {
        player_entity_id: entity_id,
        space_id,
        pos: position,
        rot: [0.0; 3],
        world_name: world_name.to_string(),
        class_id: SGWPLAYER_CLASS_ID,
        world_stargates,
    };

    // Fail-closed on missing active_player_id: same reasoning as gate-travel.
    let active_player_id: i32 = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        match clients.get(&addr).and_then(|c| c.active_player_id) {
            Some(pid) => pid,
            None => {
                tracing::error!(
                    %addr, account_id,
                    "RespawnReload: no active_player_id cached — aborting reload"
                );
                return Ok(());
            }
        }
    };
    let player_load_data = query_player_load_data(db_pool, account_id, active_player_id).await;

    // Send RESET_ENTITIES so the client tears down the ragdolled pawn.
    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
    };
    let seq = next_seq.fetch_add(1, Ordering::Relaxed);
    let pkt = build_reset_entities(&key, seq, &acks);
    socket.send_to(&pkt, addr).await?;

    // Stage pending world entry. NOTE: pending_respawn_reload = true so
    // handle_on_client_ready skips ConnectEntity + InitPlayerState — the cell
    // entity is alive and already initialised.
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_player_entity_id = Some(entity_id);
            c.pending_world_entry = Some(entry_info);
            c.pending_player_load_data = Some(player_load_data);
            c.pending_client_ready = None;
            c.pending_respawn_reload = true;
        }
    }

    tracing::info!(
        entity_id, %addr, world = %world_name, space_id,
        "Respawn reload: RESET_ENTITIES sent — awaiting ENABLE_ENTITIES (cell entity preserved)"
    );

    Ok(())
}
