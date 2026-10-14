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

use super::super::super::helpers::send_to_witness_reliable;
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
    if let Some(registry) = minigame_registry {
        // The minigame server only sees the entity id; hand it the
        // character's name so its Discord result names the player.
        let addr = entity_to_addr
            .lock()
            .ok()
            .and_then(|m| m.get(&entity_id).copied());
        let player_name = addr.and_then(|a| {
            connected
                .lock()
                .ok()
                .and_then(|c| c.get(&a).and_then(|s| s.player_name.clone()))
        });
        // The listener admits only the game connection's IP, so a session
        // without one could never be claimed and would hold the entity id
        // for the whole pending TTL. Do not register it at all.
        if addr.is_none() {
            tracing::warn!(
                entity_id,
                entity_name = player_label,
                reason = "no_client_addr",
                "Minigame not started: the player has no client address, so no session was registered"
            );
            send_launch_failure(entity_id, transport, connected, entity_to_addr).await;
            return;
        }
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
                addr.map(|a| a.ip()),
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
            send_to_witness_reliable(
                transport,
                connected,
                entity_to_addr,
                entity_id,
                |key, version, seq, acks| {
                    build_player_entity_method_packet(
                        key, seq, acks, entity_id, method, &args, version,
                    )
                },
            )
            .await;
        } else {
            tracing::warn!(
                entity_id,
                entity_name = player_label,
                %game_name,
                reason = "register_refused",
                "Failed to register minigame session (duplicate?)"
            );
            send_launch_failure(entity_id, transport, connected, entity_to_addr).await;
        }
    } else {
        tracing::warn!(
            entity_id,
            entity_name = player_label,
            %game_name,
            reason = "minigame_server_disabled",
            "Minigame not started: the minigame server is disabled"
        );
        send_launch_failure(entity_id, transport, connected, entity_to_addr).await;
    }
}

async fn send_launch_failure(
    entity_id: u32,
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
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
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
    entity_id: u32,
    ticket: &str,
    result_code: u8,
    on_victory_chains: Vec<i64>,
    notify_client: bool,
    registry: &Option<crate::minigame::SessionRegistry>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    let entity_name = entity_name_for(connected, entity_to_addr, entity_id);
    let result = cimmeria_wire::cell::client_methods::minigame::minigame_result_name(result_code);
    tracing::info!(
        entity_id,
        entity_name,
        result_code,
        result,
        "Minigame result received"
    );
    // An explicit window close already hid the UI. A late onEndMinigame
    // would close a new board if the player reopened immediately.
    let replaced = match registry {
        Some(registry) => registry.has_replacement_ticket(entity_id, ticket).await,
        None => false,
    };
    if notify_client && !replaced {
        let method = crate::cell::dispatch::CLIENT_MG_ON_END_MINIGAME;
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            |key, version, seq, acks| {
                build_player_entity_method_packet(key, seq, acks, entity_id, method, &[], version)
            },
        )
        .await;
    }
    // Forward to CellApp for victory chain processing
    let chain_count = on_victory_chains.len();
    if let Some(cell_tx) = cell_tx {
        // A dropped receiver means the cell task is gone and the victory
        // chains for this result will never run.
        if let Err(e) = cell_tx
            .send(BaseToCellMsg::MinigameResult {
                entity_id,
                result_code,
                on_victory_chains,
            })
            .await
        {
            tracing::error!(
                entity_id,
                entity_name,
                result_code,
                result,
                chain_count,
                error = %e,
                reason = "cell_channel_closed",
                "Minigame result lost: the cell channel is closed, victory chains will not run"
            );
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
mod tests {
    use super::*;
    use crate::minigame::SessionRegistry;
    use crate::test_support::{test_default_connected_client_state, TestTransport};
    use std::net::{IpAddr, Ipv4Addr};

    /// Run `start_minigame` for entity 77 with the given address mapping and
    /// return the registry it registered into.
    async fn start_for_entity(addr: Option<SocketAddr>) -> SessionRegistry {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let mut entity_map = HashMap::new();
        if let Some(addr) = addr {
            connected
                .lock()
                .unwrap()
                .insert(addr, test_default_connected_client_state());
            entity_map.insert(77u32, addr);
        }
        let entity_to_addr = Arc::new(Mutex::new(entity_map));
        let registry = SessionRegistry::new();
        let minigame_registry = Some(registry.clone());

        start_minigame(
            77,
            1,
            "Livewire".into(),
            1,
            vec![],
            &transport,
            &connected,
            &entity_to_addr,
            &minigame_registry,
            "203.0.113.1",
            9339,
        )
        .await;
        registry
    }

    /// The session carries the game connection's IP, so the listener admits
    /// that address and no other.
    #[tokio::test]
    async fn start_minigame_registers_the_game_connection_ip() {
        let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
        let registry = start_for_entity(Some(addr)).await;

        assert!(
            registry
                .expects_peer(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)))
                .await
        );
        assert!(
            !registry
                .expects_peer(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 6)))
                .await
        );
    }

    /// Without a client address nothing is registered: the entity id is not
    /// held for the pending TTL by a session no peer could ever claim.
    #[tokio::test]
    async fn start_minigame_without_client_addr_registers_nothing() {
        let registry = start_for_entity(None).await;

        assert_eq!(registry.session_count().await, 0);
    }

    /// A duplicate start cannot launch a second SWF, but must tell the
    /// player why the click did not work. The content replay guard separately
    /// pins that the terminal's interaction bit remains until victory.
    #[tokio::test]
    async fn duplicate_start_sends_feedback_without_replacing_ticket() {
        let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
        let transport = Arc::new(TestTransport::new());
        let connected = Arc::new(Mutex::new(HashMap::from([(
            addr,
            test_default_connected_client_state(),
        )])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(77u32, addr)])));
        let registry = SessionRegistry::new();
        let configured = Some(registry.clone());
        start_minigame(
            77,
            1,
            "Livewire".into(),
            1,
            vec![],
            &(transport.clone() as Arc<dyn Transport>),
            &connected,
            &entity_to_addr,
            &configured,
            "203.0.113.1",
            9339,
        )
        .await;
        assert_eq!(transport.drain().len(), 1, "first click launches Livewire");
        start_minigame(
            77,
            1,
            "Livewire".into(),
            1,
            vec![],
            &(transport.clone() as Arc<dyn Transport>),
            &connected,
            &entity_to_addr,
            &configured,
            "203.0.113.1",
            9339,
        )
        .await;
        assert_eq!(registry.session_count().await, 1);
        let sent = transport.filter_to(addr);
        assert_eq!(sent.len(), 1, "refused click emits exactly one packet");
        let args = serialize_on_player_communication(
            "SYSTEM",
            0,
            CHAN_FEEDBACK,
            "Minigame could not start. Close any open minigame and try again.",
        );
        let expected = build_player_entity_method_packet(
            &[0u8; 32],
            1,
            &[],
            77,
            crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
            &args,
            cimmeria_mercury::encryption::EncryptionVersion::V1,
        );
        assert_eq!(sent[0], expected, "refused click uses CHAN_FEEDBACK");
    }

    #[tokio::test]
    async fn stale_result_does_not_end_a_reopened_board_even_if_fin_won() {
        let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
        let transport = Arc::new(TestTransport::new());
        let connected = Arc::new(Mutex::new(HashMap::from([(
            addr,
            test_default_connected_client_state(),
        )])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(77u32, addr)])));
        let (cell_tx, mut cell_rx) = mpsc::channel(2);
        let registry = SessionRegistry::new();
        let old_ticket = registry
            .register(
                77,
                1,
                "Livewire".into(),
                1,
                1,
                0,
                0,
                0,
                1,
                vec![],
                None,
                Some(addr.ip()),
            )
            .await
            .unwrap();
        assert!(registry.cancel(77).await);
        let new_ticket = registry
            .register(
                77,
                1,
                "Livewire".into(),
                1,
                1,
                0,
                0,
                0,
                1,
                vec![],
                None,
                Some(addr.ip()),
            )
            .await
            .unwrap();
        assert_ne!(old_ticket, new_ticket);

        minigame_result(
            77,
            &old_ticket,
            0,
            vec![],
            true,
            &Some(registry.clone()),
            &(transport.clone() as Arc<dyn Transport>),
            &connected,
            &entity_to_addr,
            &Some(cell_tx.clone()),
        )
        .await;
        assert_eq!(
            transport.send_count_to(addr),
            0,
            "late onEndMinigame must not close the next board"
        );
        assert!(matches!(
            cell_rx.try_recv(),
            Ok(BaseToCellMsg::MinigameResult {
                entity_id: 77,
                result_code: 0,
                ..
            })
        ));
        minigame_result(
            77,
            &new_ticket,
            1,
            vec![],
            true,
            &Some(registry),
            &(transport.clone() as Arc<dyn Transport>),
            &connected,
            &entity_to_addr,
            &Some(cell_tx),
        )
        .await;
        assert_eq!(
            transport.send_count_to(addr),
            1,
            "current result still ends its board"
        );
    }
}
