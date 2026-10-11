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
use super::super::session_identity;
use super::super::world_entry::{handle_on_client_ready, handle_play_character};
use super::super::ConnectedClientState;
use super::bundle_drop::{self, DropReason, DroppedMsg};

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
            tracing::info!(
                %addr,
                account_id,
                account_name = session_identity::identity_for_addr(connected, addr).account_name,
                "Client requests createCharacter"
            );
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
            tracing::info!(
                %addr,
                account_id,
                account_name = session_identity::identity_for_addr(connected, addr).account_name,
                player_id, // nt:id-only requested character; play_character names it once loaded
                "Client requests playCharacter"
            );
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
            tracing::info!(
                %addr,
                account_id,
                account_name = session_identity::identity_for_addr(connected, addr).account_name,
                player_id, // nt:id-only requested character; the delete handler names it from the row
                "Client requests deleteCharacter"
            );
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
            tracing::debug!(
                %addr,
                player_id, // nt:id-only requested character; select screen has no name cached
                "Client sent requestCharacterVisuals"
            );
            handle_request_character_visuals(transport, addr, key, player_id, connected, db_pool)
                .await?;
        }
        0xC7 => {
            tracing::debug!(%addr, "Client sent onClientVersion -- acknowledged");
        }
        _ => {
            bundle_drop::log_drop(
                DropReason::UnhandledAccountMethod,
                addr,
                DroppedMsg {
                    msg_id: id,
                    class_id: cimmeria_wire::names::ACCOUNT_CLASS_ID,
                    payload,
                    // `method_name` comes from the wire byte; the Account
                    // index offset (after the cache ids) is not restated here.
                    method_index: None,
                    offset: None,
                    body_len: None,
                },
                || session_identity::identity_for_addr(connected, addr),
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
                    account_name = session_identity::identity_for_addr(&connected_c, addr)
                        .account_name,
                    player_id, // nt:id-only requested character; the row failed to load
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
                account_name = session_identity::identity_for_addr(connected, addr).account_name,
                player_id, // nt:id-only requested character; play_character names it once loaded
                event = "cooked_data.world_entry_held",
                "playCharacter held until the held cooked-data categories are resynced"
            );
        }
        // The resync finished between the check and the hold: enter now.
        Err(action) => action().await,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
    use tracing::Level;

    /// Rule 6 guard for the character-select rows: `createCharacter` names the
    /// account next to its ID. Fails with `account_id` or `account_name`
    /// removed from the arm's INFO.
    #[tokio::test]
    async fn create_character_dispatch_names_the_account() {
        let capture = LogCapture::install();
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let addr: SocketAddr = "127.0.0.1:55583".parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.account_id = 0x7000_2501;
        state.account_name = Some("acct_create".to_string());
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));

        // The refusal of an empty payload is not under test; only the INFO row.
        let _ = dispatch_base_method(
            0xC3,
            &[],
            addr,
            &transport,
            [0u8; 32],
            0x7000_2501,
            &connected,
            &None,
            &entity_manager,
            &None,
            &entity_to_addr,
            &None,
        )
        .await;

        let line = capture
            .find_message(Level::INFO, "Client requests createCharacter")
            .unwrap_or_else(|| panic!("no createCharacter line: {:#?}", capture.all()));
        assert!(
            line.has_field("account_id", &0x7000_2501u32.to_string()),
            "{line:#?}"
        );
        assert!(line.has_field("account_name", "acct_create"), "{line:#?}");
    }
}
