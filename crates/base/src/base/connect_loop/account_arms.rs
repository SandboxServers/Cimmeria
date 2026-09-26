//! Dispatch for the `0xC2..=0xC7` Account base-method range and the
//! in-world SGWPlayer base methods.
//!
//! The branching here is "are we in-world yet" — Account methods (character
//! select) live below the world-entry threshold; SGWPlayer base methods take
//! over once the player is connected. `0xC0` (versionInfoRequest) and `0xC1`
//! (elementDataRequest) are protocol-level cache messages and are handled in
//! the encrypted dispatcher directly because they fire in both phases.

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
use super::super::dispatch::{dispatch_sgw_player_base_method, sgw_player_base};
use super::super::login::handle_log_off;
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
            tracing::trace!(%addr, msg_id = format_args!("{:#04x}", id), "Unhandled Account base method");
        }
    }
    Ok(())
}
