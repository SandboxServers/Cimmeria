//! `mapLoaded` -> enter-world step.
//!
//! After the create-player step, the client loads terrain geometry and then
//! sends `mapLoaded` (cell method 25). This handler emits the VIEWPORT +
//! CELL_PLAYER + FORCED_POSITION standalone packet followed by a separate
//! fragmented bundle of entity methods (BeingAppearance, etc.).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{
    build_enter_world, build_map_loaded_body, fragment_count, fragment_map_loaded,
};

use super::super::world_entry_appearance::{
    build_appearance_args, build_tint_args, handle_cancel_movie,
};
use super::super::{ConnectedClientState, PendingClientReadyInfo};
use super::methods::default_player_load_data;

/// Enter world: send VIEWPORT + CELL_PLAYER + FORCED_POSITION + entity data.
///
/// Called when the client sends `mapLoaded` after receiving `onClientMapLoad`
/// in the create-player step. The client has finished loading terrain
/// geometry and is ready to receive entity placement and data.
///
/// In C++, this is triggered by the CellApp's `onCellPlayerCreateAck` callback
/// (which itself fires after `connected()` sends `onClientMapLoad`) and the
/// Python `onClientReady()` -> `mapLoaded()` callback chain.
pub(crate) async fn handle_map_loaded(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    _cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Take the pending data (consumes it -- enter-world only runs once per mapLoaded)
    let (entry_info, player_data) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
        let entry = c
            .pending_map_loaded
            .take()
            .ok_or("handle_map_loaded: no pending world entry")?;
        let data = c
            .pending_player_load_data
            .take()
            .unwrap_or_else(default_player_load_data);
        (entry, data)
    };

    tracing::info!(
        %addr,
        player_entity_id = entry_info.player_entity_id,
        space_id = entry_info.space_id,
        "Enter world: client map loaded -- sending VIEWPORT + CELL + POSITION + entity data"
    );

    // Send enter-world as TWO separate bundles, matching the C++ server:
    //
    // 1. VIEWPORT + CELL_PLAYER + FORCED_POSITION -- standalone 99-byte packet.
    //    This creates the cell entity, puts it in the world, and the entity enters
    //    a brief "transaction" state during creation.
    //
    // 2. Entity methods (mapLoaded body) -- separate fragmented bundle.
    //    By arriving in a new bundle, these are processed after the entity's
    //    creation transaction completes, so BeingAppearance hits the
    //    "SCHEDULING JOB" path instead of "HOLD FOR TRANSACTION".
    //
    // Previously we combined everything into one fragmented bundle, which caused
    // BeingAppearance to be silently dropped (HOLD FOR TRANSACTION) because the
    // entity was still in its creation transaction during bundle processing.
    let map_body = build_map_loaded_body(entry_info.player_entity_id, &player_data, &entry_info);

    let map_frags = fragment_count(map_body.len());
    // Reserve 1 seq for the standalone enter-world packet + N seqs for map fragments.
    let total_seqs = 1 + map_frags;

    let (acks, base_seq) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
        let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
        let seq = c.next_seq.fetch_add(total_seqs, Ordering::Relaxed);
        (acks, seq)
    };

    // Packet 1: VIEWPORT + CELL_PLAYER + FORCED_POSITION (standalone, ~99 bytes)
    let enter_world_pkt = build_enter_world(&key, base_seq, &acks, &entry_info);
    tracing::debug!(%addr, len = enter_world_pkt.len(), seq = base_seq,
        "UDP_OUT enter world: VIEWPORT+CELL+FORCED (standalone)");
    socket.send_to(&enter_world_pkt, addr).await?;
    // Issue #308: register this reliable send with the per-session
    // Channel's TX window. ACK consumption + RTO sampling are live,
    // and the retransmit driver in tick_sync.rs will resend the cached
    // bytes if the RTO fires before the client acks.
    super::super::helpers::shadow_register_reliable_send(
        connected,
        addr,
        base_seq,
        cimmeria_mercury::packet::Bytes::copy_from_slice(&enter_world_pkt),
    );

    // Packet 2+: Entity methods (mapLoaded body, possibly fragmented)
    let map_base_seq = base_seq + 1;
    let (map_packets, map_seqs) = fragment_map_loaded(&key, map_base_seq, &[], &map_body);
    debug_assert_eq!(map_seqs, map_frags);
    tracing::info!(
        %addr,
        enter_world_seq = base_seq,
        map_base_seq,
        map_fragments = map_packets.len(),
        map_body_len = map_body.len(),
        "mapLoaded: split send (standalone VIEWPORT+CELL + separate entity methods)"
    );
    for (i, pkt_data) in map_packets.iter().enumerate() {
        tracing::debug!(%addr, len = pkt_data.len(), seq = map_base_seq + i as u32,
            part = i + 1, total = map_packets.len(), "UDP_OUT mapLoaded entity data");
        socket.send_to(pkt_data, addr).await?;
        super::super::helpers::shadow_register_reliable_send(
            connected,
            addr,
            map_base_seq + i as u32,
            cimmeria_mercury::packet::Bytes::copy_from_slice(pkt_data),
        );
    }

    let total_bytes: usize =
        enter_world_pkt.len() + map_packets.iter().map(|p| p.len()).sum::<usize>();
    let pkt_count = 1 + map_packets.len();
    tracing::info!(%addr, player = %player_data.player_name,
        level = player_data.level, archetype = player_data.archetype,
        packets = pkt_count,
        "World entry complete ({} bytes across {} packets)", total_bytes, pkt_count);

    // Clear first_login flag in DB after sending the intro movie
    if player_data.first_login != 0 {
        if let Some(ref pool) = db_pool {
            let _ = sqlx::query("UPDATE sgw_player SET first_login = 0 WHERE player_id = $1")
                .bind(player_data.player_id)
                .execute(pool.as_ref())
                .await;
        }

        // The first-login cinematic (onPlayMovie) blocks the client from
        // processing BeingAppearance. cancelMovie fires if the player presses
        // Escape, but NOT if the cinematic plays to completion.
        // Spawn a delayed resend to cover the natural-end case.
        // Duplicates with cancelMovie are harmless -- client just re-applies.
        let delay_socket = Arc::clone(socket);
        let delay_connected = Arc::clone(connected);
        let delay_entity_to_addr = Arc::clone(entity_to_addr);
        let delay_entity_id = entry_info.player_entity_id;
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            tracing::info!(
                entity_id = delay_entity_id,
                "Cinematic timer: resending BeingAppearance after 10s delay"
            );
            handle_cancel_movie(
                &delay_socket,
                // Look up addr from entity_to_addr since it's stable
                {
                    let map = delay_entity_to_addr.lock().unwrap();
                    match map.get(&delay_entity_id).copied() {
                        Some(a) => a,
                        None => return,
                    }
                },
                delay_entity_id,
                &delay_connected,
                &delay_entity_to_addr,
            )
            .await;
        });
    }

    // Register entity_id -> addr before the final onClientReady gate so any
    // resource responses and future client-targeted traffic can resolve the
    // socket, but defer CellService player initialization until the client
    // explicitly signals readiness (matches C++ SGWPlayer.onClientReady).
    entity_to_addr
        .lock()
        .unwrap()
        .insert(entry_info.player_entity_id, addr);

    // Cache BeingAppearance + onEntityTint args for resend after onClientReady.
    // The first copy in the mapLoaded bundle may be dropped because the entity is
    // still in a "transaction" during bundle processing (all messages in a reassembled
    // bundle are processed in one frame). The C++ server sends BeingAppearance 3-5
    // times via createCacheStamp replays; this second send mimics that.
    let appearance_args = build_appearance_args(&player_data.bodyset, &player_data.components);
    let tint_args = build_tint_args(player_data.skin_color_id);

    // The C++ server waits for the exposed SGWPlayer base method
    // `onClientReady` (msg_id 0xD8) before calling into the cell-side
    // post-load logic that eventually fires `player.loaded`.
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
        // Cache appearance data for resend after cinematic (cancelMovie).
        // PendingClientReadyInfo is consumed by onClientReady, but cancelMovie
        // may arrive later (after the cinematic ends).
        c.cached_appearance_args = Some(appearance_args.clone());
        c.cached_tint_args = Some(tint_args.clone());
        c.pending_client_ready = Some(PendingClientReadyInfo {
            entity_id: entry_info.player_entity_id,
            player_id: player_data.player_id,
            world_name: entry_info.world_name.clone(),
            appearance_args,
            tint_args,
        });
    }

    tracing::info!(%addr, "World entry complete -- waiting for SGWPlayer.onClientReady");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};
    use tokio::net::UdpSocket;

    #[tokio::test]
    async fn map_loaded_errors_when_no_pending_entry() {
        let std_sock = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind UDP");
        std_sock.set_nonblocking(true).unwrap();
        let socket = Arc::new(UdpSocket::from_std(std_sock).expect("from_std"));
        let addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new({
                let mut m = HashMap::new();
                m.insert(
                    addr,
                    crate::test_support::test_default_connected_client_state(),
                );
                m
            }));
        let key = [0u8; 32];

        // Client has no pending_map_loaded — must return an error.
        let result = handle_map_loaded(
            &socket,
            addr,
            key,
            &connected,
            &None,
            &Arc::new(Mutex::new(HashMap::new())),
            &None,
        )
        .await;
        let err = result.expect_err("must fail when pending_map_loaded is missing");
        assert!(
            err.to_string().contains("no pending world entry"),
            "unexpected error: {err}"
        );
    }
}
