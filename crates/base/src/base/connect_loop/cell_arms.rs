//! Dispatch for the `0x80..=0xBF` cell entity method range — direct
//! encoding (0-60) and the `0xBD` extended sub-slot encoding (61+).
//!
//! The 4-byte entityId prefix is always stripped before the args are
//! forwarded to the cell. The first `mapLoaded` (method index 25,
//! `msg_id 0x99`) message also kicks off the world-entry handshake.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::names;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;

use super::super::world_entry::{handle_cancel_movie, handle_map_loaded};
use super::super::ConnectedClientState;

/// Dispatch a cell-method message in the `0x80..=0xBF` range. Returns
/// [`ControlFlow::Break`] to signal the encrypted dispatcher to `continue`
/// the bundle scan (used when the message arrived before `mapLoaded` and
/// must be ignored). `packet_seq` is the Mercury seq of the packet that
/// carried the message; it rides to the cell for the receipt rows (AB-T2).
#[allow(clippy::too_many_arguments)]
pub(super) async fn dispatch_cell_method(
    id: u8,
    payload: &[u8],
    packet_seq: Option<u32>,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> ControlFlow<()> {
    // ── Enter world trigger ──
    // After the create-player step, the client sends the exposed
    // SGWPlayer cell method `mapLoaded` (index 25, msg_id 0x99).
    // The C++ server waits for that specific method before sending
    // VIEWPORT + CELL + POSITION + the full entity data bundle.
    let map_loaded_pending = {
        let clients = connected.lock().unwrap();
        clients
            .get(&addr)
            .is_some_and(|c| c.pending_map_loaded.is_some())
    };
    if map_loaded_pending && id == 0x99 {
        if let Err(e) = handle_map_loaded(
            transport,
            addr,
            key,
            connected,
            cell_tx,
            entity_to_addr,
            db_pool,
        )
        .await
        {
            tracing::error!(%addr, error = %e, "Enter world (mapLoaded) failed");
        }
    }

    // The character name rides the lock this read already takes (interned,
    // so a hash hit): the lines below must not lock `connected` again just
    // to name the player (Rule 6, hot-path rule).
    let (player_eid, player_label) = {
        let clients = connected.lock().unwrap();
        clients.get(&addr).map_or((None, None), |c| {
            (
                c.player_entity_id,
                cimmeria_entity::name_intern::intern_opt(c.player_name.as_deref()),
            )
        })
    };
    let Some(player_eid) = player_eid else {
        tracing::trace!(
            %addr,
            msg_id = format_args!("{:#04x}", id),
            msg_name = names::server_msg_name(id),
            method_name = names::player_inbound_method(id, payload),
            "Cell method before world entry -- ignored"
        );
        return ControlFlow::Continue(());
    };

    if map_loaded_pending && id != 0x99 {
        tracing::trace!(
            %addr,
            msg_id = format_args!("{:#04x}", id),
            msg_name = names::server_msg_name(id),
            method_name = names::player_inbound_method(id, payload),
            "Ignoring cell method until mapLoaded arrives"
        );
        return ControlFlow::Break(());
    }

    // Strip 4-byte entityId prefix (always present per entity_message_handler.cpp:18-20)
    if payload.len() < 4 {
        // `method_name` is absent for the 0xBD sub-slot, whose index is in
        // the bytes this payload is too short for.
        tracing::warn!(
            %addr,
            msg_id = format_args!("{:#04x}", id),
            msg_name = names::server_msg_name(id),
            method_name = names::player_inbound_method(id, payload),
            payload_len = payload.len(),
            "Cell method payload too short for entityId prefix"
        );
        return ControlFlow::Break(());
    }
    let entity_id_from_client =
        u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let method_payload = &payload[4..];

    if id == 0xBD {
        // Extended encoding: sub_index is first byte AFTER entityId
        if !method_payload.is_empty() {
            let sub_index = method_payload[0] as u16;
            let method_index = sub_index + 61;
            let args = if method_payload.len() > 1 {
                method_payload[1..].to_vec()
            } else {
                Vec::new()
            };
            tracing::debug!(
                %addr,
                entity_id = entity_id_from_client,
                // Named only when the client addresses its own entity.
                entity_name = (entity_id_from_client == player_eid)
                    .then_some(player_label)
                    .flatten(),
                sub_index,
                method_index,
                method_name = names::player_cell_method(method_index),
                payload_hex = %payload[..payload.len().min(12)].iter().map(|b| format!("{:02x}", b)).collect::<String>(),
                "Extended cell method (0xBD)"
            );

            // cancelMovie (index 108): client sends this when a
            // cinematic finishes. Resend BeingAppearance + onEntityTint
            // so the model loads after the first-login intro movie.
            const CM_CANCEL_MOVIE: u16 = 108;
            if method_index == CM_CANCEL_MOVIE {
                tracing::info!(
                    %addr,
                    entity_id = player_eid,
                    entity_name = player_label,
                    "cancelMovie received — resending BeingAppearance + onEntityTint"
                );
                handle_cancel_movie(transport, addr, player_eid, connected, entity_to_addr).await;
            }

            if let Some(ref tx) = cell_tx {
                let _ = tx
                    .send(BaseToCellMsg::CellMethodCall {
                        entity_id: player_eid,
                        method_index,
                        args,
                        packet_seq,
                    })
                    .await;
            }
        }
    } else {
        let method_index = (id - 0x80) as u16;
        if let Some(ref tx) = cell_tx {
            let _ = tx
                .send(BaseToCellMsg::CellMethodCall {
                    entity_id: player_eid,
                    method_index,
                    args: method_payload.to_vec(),
                    packet_seq,
                })
                .await;
        }
    }

    ControlFlow::Continue(())
}
