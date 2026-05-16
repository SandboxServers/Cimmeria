//! Minigame session handlers — `StartMinigame` registers the session and
//! pushes `onStartMinigame(URL)`; `MinigameResult` notifies the client and
//! forwards the result back to CellApp for victory-chain processing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::build_entity_method_packet;

use super::super::super::helpers::send_to_witness;
use super::super::super::ConnectedClientState;

/// `CellToBaseMsg::StartMinigame` — register a session ticket and push
/// `onStartMinigame(URL)` to the player so the client launches the minigame
/// browser/iframe pointing at the in-process minigame service.
pub(super) async fn start_minigame(
    entity_id: u32,
    player_id: i32,
    game_name: String,
    difficulty: u32,
    on_victory_chains: Vec<i64>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    minigame_registry: &Option<crate::minigame::SessionRegistry>,
    minigame_external_host: &str,
    minigame_external_port: u16,
) {
    tracing::info!(entity_id, player_id, %game_name, difficulty, "Starting minigame session");
    if let Some(registry) = minigame_registry {
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
            )
            .await;

        if let Some(ticket) = ticket {
            // Build URL: http://unused/{ip}/{port}/{gameName}/{entityId}/{ticket}
            let url = format!(
                "http://unused/{}/{}/{}/{}/{}",
                minigame_external_host, minigame_external_port, game_name, entity_id, ticket
            );
            tracing::info!(entity_id, %url, "Sending onStartMinigame to client");

            // onStartMinigame(URL: WSTRING) — MinigamePlayer client method
            // Method index for onStartMinigame in the SGWPlayer flat dispatch table
            let url_utf16: Vec<u16> = url.encode_utf16().collect();
            let mut args = Vec::with_capacity(4 + url_utf16.len() * 2);
            args.extend_from_slice(&(url_utf16.len() as u32).to_le_bytes());
            for ch in &url_utf16 {
                args.extend_from_slice(&ch.to_le_bytes());
            }
            let method = crate::cell::dispatch::CLIENT_MG_ON_START_MINIGAME;
            if let Err(e) = send_to_witness(
                socket,
                connected,
                entity_to_addr,
                entity_id,
                entity_id,
                "METHOD",
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id, method, &args)
                },
            )
            .await
            {
                tracing::warn!(entity_id, action = "METHOD", "send_to_witness failed: {e}");
            }
        } else {
            tracing::warn!(
                entity_id,
                "Failed to register minigame session (duplicate?)"
            );
        }
    }
}

/// `CellToBaseMsg::MinigameResult` — push `onEndMinigame()` to the client
/// and forward the result to CellApp so it can fire any victory chains.
pub(super) async fn minigame_result(
    entity_id: u32,
    result_code: u8,
    on_victory_chains: Vec<i64>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    tracing::info!(entity_id, result_code, "Minigame result received");
    // Send onEndMinigame to client
    let method = crate::cell::dispatch::CLIENT_MG_ON_END_MINIGAME;
    if let Err(e) = send_to_witness(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        entity_id,
        "METHOD",
        |key, seq, acks| build_entity_method_packet(key, seq, acks, entity_id, method, &[]),
    )
    .await
    {
        tracing::warn!(entity_id, action = "METHOD", "send_to_witness failed: {e}");
    }
    // Forward to CellApp for victory chain processing
    if let Some(cell_tx) = cell_tx {
        if let Err(e) = cell_tx
            .send(BaseToCellMsg::MinigameResult {
                entity_id,
                result_code,
                on_victory_chains,
            })
            .await
        {
            tracing::warn!(entity_id, "MinigameResult send failed: {e}");
        }
    }
}
