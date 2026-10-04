//! Dispatch for the `0xC2..=0xC7` Account base-method range and the
//! in-world SGWPlayer base methods.
//!
//! The branching here is "are we in-world yet" — Account methods (character
//! select) live below the world-entry threshold; SGWPlayer base methods take
//! over once the player is connected. At character select `0xC0`
//! (versionInfoRequest) and `0xC1` (elementDataRequest) are the cache
//! messages, handled in the encrypted dispatcher; in-world they are
//! SGWPlayer `chatJoin` / `chatLeave` and arrive here.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;

use super::super::character::{handle_delete_character, handle_request_character_visuals};
use super::super::character_create::handle_create_character;
use super::super::cooked_data::handle_element_data_request;
use super::super::cooked_sync;
use super::super::dispatch::{dispatch_sgw_player_base_method, sgw_player_base};
use super::super::login::handle_log_off;
use super::super::resources::ResourceCache;
use super::super::world_entry::{handle_on_client_ready, handle_play_character};
use super::super::ConnectedClientState;

/// Dispatch a base-method message in the `0xC2..=0xC7` range. Branches on
/// whether the connection is in-world (player entity present) or still at
/// character select.
pub(super) async fn dispatch_base_method(
    id: u8,
    payload: &[u8],
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    key: [u8; 32],
    account_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    resource_cache: &Option<Arc<ResourceCache>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (in_world, player_name) = {
        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => (c.player_entity_id.is_some(), c.player_name.clone()),
            None => (false, None),
        }
    };

    if in_world {
        match id {
            sgw_player_base::ON_CLIENT_READY => {
                handle_on_client_ready(
                    addr,
                    key,
                    connected,
                    cell_tx,
                    transport,
                    entity_to_addr,
                    db_pool,
                )
                .await?;
            }
            // An in-world cache miss: served from the resource cache, ahead
            // of any background resync (#840).
            sgw_player_base::ELEMENT_DATA_REQUEST => {
                handle_element_data_request(
                    transport,
                    addr,
                    key,
                    payload,
                    connected,
                    resource_cache,
                )
                .await?;
            }
            _ => {
                // SGWPlayer base method dispatch
                dispatch_sgw_player_base_method(
                    id,
                    payload,
                    &player_name,
                    addr,
                    transport,
                    key,
                    connected,
                    entity_manager,
                    cell_tx,
                    entity_to_addr,
                    db_pool,
                )
                .await?;
            }
        }
        return Ok(());
    }

    // Account base method dispatch (character select).
    match id {
        0xC2 => {
            handle_log_off(
                transport,
                addr,
                key,
                connected,
                entity_manager,
                cell_tx,
                entity_to_addr,
            )
            .await?;
        }
        0xC3 => {
            tracing::info!(%addr, "Client requests createCharacter");
            handle_create_character(
                transport, addr, key, account_id, payload, connected, db_pool,
            )
            .await?;
        }
        0xC4 => {
            let player_id = if payload.len() >= 4 {
                i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
            } else {
                0
            };
            tracing::info!(%addr, player_id, "Client requests playCharacter");
            if cooked_sync::holds_world_entry(connected, addr) {
                hold_play_character(
                    transport,
                    addr,
                    key,
                    account_id,
                    player_id,
                    connected,
                    db_pool,
                    entity_manager,
                    cell_tx,
                )
                .await?;
            } else {
                handle_play_character(
                    transport,
                    addr,
                    key,
                    account_id,
                    player_id,
                    connected,
                    db_pool,
                    entity_manager,
                    cell_tx,
                )
                .await?;
            }
        }
        0xC5 => {
            let player_id = if payload.len() >= 4 {
                i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
            } else {
                0
            };
            tracing::info!(%addr, player_id, "Client requests deleteCharacter");
            handle_delete_character(
                transport, addr, key, account_id, player_id, connected, db_pool,
            )
            .await?;
        }
        0xC6 => {
            let player_id = if payload.len() >= 4 {
                i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
            } else {
                0
            };
            tracing::debug!(%addr, player_id, "Client sent requestCharacterVisuals");
            handle_request_character_visuals(transport, addr, key, player_id, connected, db_pool)
                .await?;
        }
        0xC7 => {
            tracing::debug!(%addr, "Client sent onClientVersion -- acknowledged");
        }
        _ => {
            tracing::trace!(
                %addr,
                msg_id = format_args!("{:#04x}", id),
                msg_name = cimmeria_wire::names::server_msg_name(id),
                method_name = cimmeria_wire::names::inbound_method(
                    cimmeria_wire::names::ACCOUNT_CLASS_ID,
                    id,
                    payload
                ),
                entity_type = "Account",
                "Unhandled Account base method"
            );
        }
    }
    Ok(())
}

/// Hold `playCharacter` until the session's held cooked-data categories are
/// resynced.
///
/// The client empties a mismatched category the moment it reads the resync's
/// opening `onVersionInfo`, and gets each entry back as it is pushed or as it
/// asks for it (`elementDataRequest`, served ahead of the stream). The held
/// categories (`cooked_sync::HELD_CATEGORIES`: world info, which
/// `onClientMapLoad` needs, and the others with no client miss path) cannot
/// be asked for, so world entry waits for them. Everything else keeps
/// streaming in the world. The resync task runs this once the last held
/// category is pushed; if the session disconnects first, it is dropped.
async fn hold_play_character(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (transport_c, connected_c, db_c, em_c, cell_c) = (
        Arc::clone(transport),
        Arc::clone(connected),
        db_pool.clone(),
        Arc::clone(entity_manager),
        cell_tx.clone(),
    );
    let action: cooked_sync::DeferredAction = Box::new(move || {
        Box::pin(async move {
            if let Err(e) = handle_play_character(
                &transport_c,
                addr,
                key,
                account_id,
                player_id,
                &connected_c,
                &db_c,
                &em_c,
                &cell_c,
            )
            .await
            {
                tracing::warn!(
                    %addr,
                    account_id,
                    player_id,
                    reason = "play_character_failed",
                    error = %e,
                    "Held playCharacter failed after the cooked-data resync"
                );
            }
        })
    });
    match cooked_sync::defer_until_synced(connected, addr, action) {
        Ok(()) => {
            tracing::info!(
                %addr,
                account_id,
                player_id,
                event = "cooked_data.world_entry_held",
                "playCharacter held until the held cooked-data categories are resynced"
            );
        }
        // The resync finished between the check and the hold: enter now.
        Err(action) => action().await,
    }
    Ok(())
}
