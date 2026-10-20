//! Minigame session handlers — `StartMinigame` registers the session and
//! pushes `onStartMinigame(URL)`; `MinigameResult` notifies the client and
//! forwards the result back to CellApp for victory-chain processing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::credential_redaction::CredentialPrefix;
use crate::mercury::build_player_entity_method_packet;

use super::super::super::helpers::{
    send_to_session_bound_reliable, snapshot_recipient, RecipientSnapshot, WitnessSendOutcome,
};
use super::super::super::session_identity::entity_name_for;
use super::super::super::ConnectedClientState;

/// Release the owner's registration immediately and signal its SmartFox task
/// to report Canceled and close the socket. Ticket-scoped teardown protects
/// a new board if the player reopens before the old task finishes.
pub(super) async fn cancel_minigame(
    entity_id: u32,
    registry: &Option<crate::minigame::SessionRegistry>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let canceled = match registry {
        Some(registry) => registry.cancel(entity_id).await,
        None => false,
    };
    tracing::info!(
        entity_id,
        entity_name = entity_name_for(connected, entity_to_addr, entity_id),
        canceled,
        reason = "player_closed_window",
        "Minigame cancel handled"
    );
}

/// `CellToBaseMsg::StartMinigame` — register a session ticket and push
/// `onStartMinigame(URL)` to the player so the client launches the minigame
/// browser/iframe pointing at the in-process minigame service.
pub(super) async fn start_minigame(
    entity_id: u32,
    player_id: i32,
    game_name: String,
    difficulty: u32,
    on_victory_chains: Vec<i64>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    minigame_registry: &Option<crate::minigame::SessionRegistry>,
    minigame_external_host: &str,
    minigame_external_port: u16,
) {
    let player_label = entity_name_for(connected, entity_to_addr, entity_id);
    tracing::info!(
        entity_id,
        entity_name = player_label,
        player_id,
        player_name = player_label,
        %game_name,
        difficulty,
        "Starting minigame session"
    );
    let Some(recipient) = snapshot_recipient(connected, entity_to_addr, entity_id, player_id)
    else {
        if minigame_registry.is_none() {
            tracing::warn!(
                entity_id,
                entity_name = player_label,
                %game_name,
                reason = "minigame_server_disabled",
                "Minigame not started: the minigame server is disabled"
            );
        }
        tracing::warn!(
            entity_id,
            entity_name = player_label,
            player_id,
            player_name = player_label,
            reason = "no_current_owner",
            "Minigame launch refused"
        );
        return;
    };
    if let Some(registry) = minigame_registry {
        // The minigame server only sees the entity id; hand it the
        // character's name so its Discord result names the player.
        let player_name = {
            connected
                .lock()
                .ok()
                .and_then(|c| c.get(&recipient.addr).and_then(|s| s.player_name.clone()))
        };
        let seed = rand::random::<u32>();
        let ticket = registry
            .register(
                entity_id,
                player_id,
                game_name.clone(),
                difficulty,
                1, // tech_competency — TODO: read from player entity
                seed,
                0,
                0,
                1, // abilities, intelligence, player_level
                on_victory_chains,
                player_name,
                Some(recipient.addr.ip()),
                recipient.session_instance_id,
            )
            .await;

        if let Some(ticket) = ticket {
            // Build URL: http://unused/{ip}/{port}/{gameName}/{entityId}/{ticket}
            let url = format!(
                "http://unused/{}/{}/{}/{}/{}",
                minigame_external_host, minigame_external_port, game_name, entity_id, ticket
            );
            tracing::info!(
                entity_id,
                entity_name = player_label,
                %game_name,
                ticket_prefix = %CredentialPrefix(&ticket),
                "Sending onStartMinigame to client"
            );

            // onStartMinigame(URL: WSTRING) — MinigamePlayer client method
            // Method index for onStartMinigame in the SGWPlayer flat dispatch table
            let url_utf16: Vec<u16> = url.encode_utf16().collect();
            let mut args = Vec::with_capacity(4 + url_utf16.len() * 2);
            args.extend_from_slice(&(url_utf16.len() as u32).to_le_bytes());
            for ch in &url_utf16 {
                args.extend_from_slice(&ch.to_le_bytes());
            }
            let method = crate::cell::dispatch::CLIENT_MG_ON_START_MINIGAME;
            let outcome = send_to_session_bound_reliable(
                transport,
                connected,
                entity_to_addr,
                recipient,
                "minigame_start",
                |key, version, seq, acks| {
                    build_player_entity_method_packet(
                        key, seq, acks, entity_id, method, &args, version,
                    )
                },
            )
            .await;
            if !matches!(outcome, WitnessSendOutcome::Sent { .. }) {
                let ticket_canceled = registry.cancel_if_ticket(entity_id, &ticket).await;
                tracing::warn!(
                    entity_id,
                    entity_name = player_label,
                    player_id,
                    player_name = player_label,
                    %game_name,
                    send_outcome = ?outcome,
                    ticket_canceled,
                    reason = "minigame_url_not_registered",
                    "Minigame launch failed after ticket registration"
                );
                send_launch_failure(recipient, transport, connected, entity_to_addr).await;
            }
        } else {
            tracing::warn!(
                entity_id,
                entity_name = player_label,
                %game_name,
                reason = "register_refused",
                "Failed to register minigame session (duplicate?)"
            );
            send_launch_failure(recipient, transport, connected, entity_to_addr).await;
        }
    } else {
        tracing::warn!(
            entity_id,
            entity_name = player_label,
            %game_name,
            reason = "minigame_server_disabled",
            "Minigame not started: the minigame server is disabled"
        );
        send_launch_failure(recipient, transport, connected, entity_to_addr).await;
    }
}

async fn send_launch_failure(
    recipient: RecipientSnapshot,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let args = serialize_on_player_communication(
        "SYSTEM",
        0,
        CHAN_FEEDBACK,
        "Minigame could not start. Close any open minigame and try again.",
    );
    send_to_session_bound_reliable(
        transport,
        connected,
        entity_to_addr,
        recipient,
        "minigame_start_failure",
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                recipient.entity_id,
                crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
                &args,
                version,
            )
        },
    )
    .await;
}

/// `CellToBaseMsg::MinigameResult` — push `onEndMinigame()` to the client
/// and forward the result to CellApp so it can fire any victory chains.
pub(super) async fn minigame_result(
    ticket: &str,
    registry: &Option<crate::minigame::SessionRegistry>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    let Some(registry) = registry else {
        tracing::warn!(
            reason = "result_registry_disabled",
            "Minigame result notification dropped"
        );
        return;
    };
    let Some(receipt) = registry.redeem_result(ticket).await else {
        tracing::warn!(reason = "result_receipt_missing", ticket_prefix = %CredentialPrefix(ticket),
            "Minigame result notification dropped");
        return;
    };
    let entity_id = receipt.entity_id;
    let player_id = receipt.player_id;
    let original_instance = receipt.session_instance_id;
    let current_owner = || {
        snapshot_recipient(connected, entity_to_addr, entity_id, player_id)
            .filter(|owner| owner.session_instance_id == original_instance)
    };
    let Some(recipient) = current_owner() else {
        tracing::warn!(
            entity_id,
            entity_name = receipt.player_name.as_deref(),
            player_id,
            player_name = receipt.player_name.as_deref(),
            reason = "result_owner_changed",
            "Minigame result notification dropped"
        );
        return;
    };
    if receipt.notify_client {
        let method = crate::cell::dispatch::CLIENT_MG_ON_END_MINIGAME;
        send_to_session_bound_reliable(
            transport,
            connected,
            entity_to_addr,
            recipient,
            "minigame_end",
            |key, version, seq, acks| {
                build_player_entity_method_packet(key, seq, acks, entity_id, method, &[], version)
            },
        )
        .await;
    }
    // A close can await transport while the same address is taken over.
    // Rechecking E/P/I prevents a Cell callback for the successor.
    if current_owner().is_none() {
        tracing::warn!(
            entity_id,
            entity_name = receipt.player_name.as_deref(),
            player_id,
            player_name = receipt.player_name.as_deref(),
            reason = "result_owner_changed_after_close",
            "Minigame Cell forward suppressed"
        );
        return;
    }
    let result_code = receipt.result_code;
    let chain_count = receipt.on_victory_chains.len();
    let entity_name = entity_name_for(connected, entity_to_addr, entity_id);
    let result = cimmeria_wire::cell::client_methods::minigame::minigame_result_name(result_code);
    if let Some(cell_tx) = cell_tx {
        if let Err(e) = cell_tx
            .send(BaseToCellMsg::MinigameResult {
                entity_id,
                player_id,
                result_code,
                on_victory_chains: receipt.on_victory_chains,
            })
            .await
        {
            tracing::error!(entity_id, entity_name, result_code, result, chain_count,
                error = %e, reason = "cell_channel_closed",
                "Minigame result lost: the cell channel is closed, victory chains will not run");
        }
    } else {
        tracing::warn!(
            entity_id,
            entity_name,
            result_code,
            result,
            chain_count,
            reason = "no_cell_channel",
            "Minigame result lost: no cell channel, victory chains will not run"
        );
    }
}

#[cfg(test)]
#[path = "minigame_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "minigame_takeover_tests.rs"]
mod takeover_tests;
