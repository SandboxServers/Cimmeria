//! Character list queries, delete, visuals, and shared helpers.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use crate::mercury::{
    build_char_create_failed, build_character_visuals, build_on_character_list,
    CharacterInfo, SKIN_TINTS,
};

use super::ConnectedClientState;
use super::helpers::{drain_acks_and_seq, get_account_entity_id};

/// Query the character list from the database.
pub(crate) async fn query_character_list(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
) -> Vec<CharacterInfo> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!("No DB pool -- returning empty character list");
            return Vec::new();
        }
    };

    #[derive(sqlx::FromRow)]
    struct CharRow {
        player_id: i32,
        player_name: String,
        extra_name: String,
        alignment: i32,
        level: i32,
        gender: i32,
        world_location: String,
        archetype: i32,
        title: i32,
    }

    tracing::debug!(account_id, "Querying sgw_player for character list");

    match sqlx::query_as::<_, CharRow>(
        "SELECT player_id, player_name, extra_name, alignment, level, gender, \
         world_location, archetype, title \
         FROM sgw_player WHERE account_id = $1 ORDER BY player_id",
    )
    .bind(account_id as i32)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => {
            tracing::info!(account_id, count = rows.len(), "Character list query result");
            rows.into_iter()
                .map(|r| CharacterInfo {
                    player_id: r.player_id,
                    name: r.player_name,
                    extra_name: r.extra_name,
                    alignment: r.alignment as u8,
                    level: r.level as u8,
                    gender: r.gender as u8,
                    world_location: r.world_location,
                    archetype: r.archetype as u8,
                    title: r.title as u8,
                    player_type: 1,
                    playable: 1,
                })
                .collect()
        }
        Err(e) => {
            tracing::error!(account_id, "Failed to query character list: {e}");
            Vec::new()
        }
    }
}

/// Send `onCharacterCreateFailed`.
pub(crate) async fn send_char_create_failed(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    error_code: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let account_eid = get_account_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_char_create_failed(&key, seq, &acks, error_code, account_eid);
    socket.send_to(&pkt, addr).await?;
    Ok(())
}

/// Handle `deleteCharacter` (0xC5) -- delete a character and send updated list.
pub(crate) async fn handle_delete_character(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(%addr, "deleteCharacter: no DB pool");
            return Ok(());
        }
    };

    let result = sqlx::query(
        "DELETE FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(r) => {
            if r.rows_affected() > 0 {
                tracing::info!(%addr, player_id, account_id, "Character deleted");
            } else {
                tracing::warn!(%addr, player_id, account_id, "Character not found or not owned");
            }
        }
        Err(e) => {
            tracing::error!(%addr, player_id, "Failed to delete character: {e}");
            return Ok(());
        }
    }

    let characters = query_character_list(db_pool, account_id).await;
    let account_eid = get_account_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_on_character_list(&key, seq, &acks, &characters, account_eid);
    tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT updated char_list after delete");
    socket.send_to(&pkt, addr).await?;

    Ok(())
}

/// Handle `requestCharacterVisuals` (0xC6).
pub(crate) async fn handle_request_character_visuals(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(%addr, player_id, "requestCharacterVisuals: no DB pool");
            return Ok(());
        }
    };

    let account_id = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients.get(&addr).ok_or("addr not in connected map")?.account_id
    };

    let row = sqlx::query_as::<_, (String, Vec<String>, i32, i32)>(
        "SELECT bodyset, components, skin_color_id, bandolier_slot \
         FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await;

    match row {
        Ok(Some((bodyset, mut components, skin_color_id, bandolier_slot))) => {
            let item_visuals: Vec<String> = sqlx::query_scalar(
                "SELECT ri.visual_component \
                 FROM sgw_inventory inv \
                 JOIN resources.items ri ON ri.item_id = inv.type_id \
                 WHERE inv.character_id = $1 \
                   AND ri.visual_component IS NOT NULL \
                   AND ( \
                     (inv.container_id IN (3,4,5,6,7,8,9,10,11,12,13,14) AND inv.slot_id = 0) \
                     OR (inv.container_id = 3 AND inv.slot_id = $2) \
                   )",
            )
            .bind(player_id)
            .bind(bandolier_slot)
            .fetch_all(pool.as_ref())
            .await
            .unwrap_or_default();

            components.extend(item_visuals);

            tracing::debug!(
                %addr, player_id, %bodyset,
                component_count = components.len(),
                skin_color_id,
                "Sending character visuals"
            );

            let skin_tint = SKIN_TINTS.get(skin_color_id as usize).copied().unwrap_or(0x2F1308FF);
            let account_eid = get_account_entity_id(connected, addr)?;
            let (acks, seq) = drain_acks_and_seq(connected, addr)?;
            let pkt = build_character_visuals(
                &key, seq, &acks,
                player_id,
                &bodyset,
                &components,
                0xFF,
                0xFF,
                skin_tint,
                account_eid,
            );
            tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT onCharacterVisuals");
            socket.send_to(&pkt, addr).await?;
        }
        Ok(None) => {
            tracing::warn!(%addr, player_id, "requestCharacterVisuals: player not found");
        }
        Err(e) => {
            tracing::error!(%addr, player_id, error = %e, "requestCharacterVisuals: DB error");
        }
    }

    Ok(())
}
