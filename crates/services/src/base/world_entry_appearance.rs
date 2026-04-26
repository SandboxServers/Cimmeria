//! BeingAppearance assembly, onEntityTint, and visual resend helpers.
//!
//! Extracted from `world_entry.rs` — these functions build the appearance
//! wire data and handle the post-transaction / post-cinematic resend logic.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{
    build_entity_method_packet, method_idx, write_wstring, SKIN_TINTS,
};

use super::ConnectedClientState;
use super::helpers::send_to_witness;

// ── Appearance data builders ────────────────────────────────────────────────

/// Build the BeingAppearance wire args: `[wstring bodyset][u32 count][wstring comp]*`.
///
/// Used by `handle_map_loaded` to cache for later resend, and by
/// `handle_on_client_ready` / `handle_cancel_movie` to resend.
pub(crate) fn build_appearance_args(bodyset: &str, components: &[String]) -> Vec<u8> {
    let mut buf = Vec::new();
    write_wstring(&mut buf, bodyset);
    buf.extend_from_slice(&(components.len() as u32).to_le_bytes());
    for comp in components {
        write_wstring(&mut buf, comp);
    }
    buf
}

/// Build the onEntityTint wire args: `[u32 primary=0][u32 secondary=0][u32 skin_tint]`.
///
/// Maps `skin_color_id` (DB index) through the SKIN_TINTS table, matching
/// the C++ `requestCharacterVisuals` flow that sends the mapped tint value.
pub(crate) fn build_tint_args(skin_color_id: i32) -> Vec<u8> {
    let skin_tint = if (skin_color_id as usize) < SKIN_TINTS.len() {
        SKIN_TINTS[skin_color_id as usize]
    } else {
        SKIN_TINTS[0]
    };
    let mut buf = Vec::with_capacity(12);
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&skin_tint.to_le_bytes());
    buf
}

// ── Visual resend handlers ──────────────────────────────────────────────────

/// Finalize world entry after the client sends `SGWPlayer.onClientReady`.
///
/// Also resends BeingAppearance + onEntityTint. The first copy was sent in the
/// mapLoaded bundle but may have been dropped because the entity was still in a
/// "transaction" during bundle processing. The C++ server sends BeingAppearance
/// 3-5 times via createCacheStamp replays; this second send mimics that.
pub(crate) async fn handle_on_client_ready(
    addr: SocketAddr,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<sqlx::PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pending = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients
            .get_mut(&addr)
            .and_then(|c| c.pending_client_ready.take())
    };

    let Some(pending) = pending else {
        tracing::debug!(%addr, "SGWPlayer.onClientReady received with no pending world-entry finalization");
        return Ok(());
    };

    let entity_id = pending.entity_id;

    tracing::info!(
        %addr,
        entity_id,
        player_id = pending.player_id,
        world = %pending.world_name,
        "SGWPlayer.onClientReady received -- finalizing world entry"
    );

    // Query saved missions from DB before sending InitPlayerState
    let saved_missions = super::world_entry_methods::query_saved_missions(
        db_pool, pending.player_id,
    ).await;

    // Query player abilities from DB
    let abilities: Vec<i32> = if let Some(pool) = db_pool {
        sqlx::query_scalar("SELECT unnest(abilities) FROM sgw_player WHERE player_id = $1")
            .bind(pending.player_id)
            .fetch_all(pool.as_ref())
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    if let Some(ref tx) = cell_tx {
        let _ = tx.send(BaseToCellMsg::ConnectEntity {
            entity_id,
        }).await;

        let _ = tx.send(BaseToCellMsg::InitPlayerState {
            entity_id,
            player_id: pending.player_id,
            world_name: pending.world_name.clone(),
            saved_missions,
            abilities,
            active_bandolier_slot: 0,
            bandolier_items: vec![],
        }).await;
    }

    // Resend BeingAppearance + onEntityTint now that the entity is fully ready.
    let appearance_args = pending.appearance_args;
    let tint_args = pending.tint_args;
    send_to_witness(
        socket, connected, entity_to_addr, entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key, seq, acks, entity_id,
                method_idx::BEING_APPEARANCE, &appearance_args,
            )
        },
    ).await;
    send_to_witness(
        socket, connected, entity_to_addr, entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key, seq, acks, entity_id,
                method_idx::ON_ENTITY_TINT, &tint_args,
            )
        },
    ).await;

    tracing::info!(%addr, entity_id, "World entry finalized (BeingAppearance resent)");
    Ok(())
}

/// Resend BeingAppearance + onEntityTint after the first-login cinematic finishes.
///
/// The client sends `cancelMovie` (exposed cell method index 108) when the intro
/// cinematic ends. By this point both previous BeingAppearance sends (in the
/// mapLoaded bundle and after onClientReady) may have been lost because the
/// cinematic was rendering full-screen. This third send ensures the model loads.
pub(crate) async fn handle_cancel_movie(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let cached = {
        let clients = connected.lock().unwrap();
        clients.get(&addr).and_then(|c| {
            match (&c.cached_appearance_args, &c.cached_tint_args) {
                (Some(a), Some(t)) => Some((a.clone(), t.clone())),
                _ => None,
            }
        })
    };

    let Some((appearance_args, tint_args)) = cached else {
        tracing::debug!(%addr, entity_id, "cancelMovie: no cached appearance data -- skipping resend");
        return;
    };

    send_to_witness(
        socket, connected, entity_to_addr, entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key, seq, acks, entity_id,
                method_idx::BEING_APPEARANCE, &appearance_args,
            )
        },
    ).await;
    send_to_witness(
        socket, connected, entity_to_addr, entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key, seq, acks, entity_id,
                method_idx::ON_ENTITY_TINT, &tint_args,
            )
        },
    ).await;

    tracing::info!(%addr, entity_id, "cancelMovie: BeingAppearance + onEntityTint resent after cinematic");
}
