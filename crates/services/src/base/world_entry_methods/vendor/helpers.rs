use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::base::{ConnectedClientState, helpers};
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx};
use super::super::player_load::meta::query_bandolier_items;

pub async fn send_cash_changed_to_client(
    entity_id: u32,
    total: i32,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    helpers::send_to_witness(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_CASH_CHANGED,
                &total.to_le_bytes(),
            )
        },
    )
    .await;
}

pub async fn sync_bandolier_after_inventory_change(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let (Some(pool), Some(tx)) = (db_pool, cell_tx) else {
        return;
    };

    let old_active: i32 =
        sqlx::query_scalar("SELECT bandolier_slot FROM sgw_player WHERE player_id = $1 LIMIT 1")
            .bind(player_id)
            .fetch_optional(pool.as_ref())
            .await
            .ok()
            .flatten()
            .unwrap_or(0);

    let bandolier_items = query_bandolier_items(db_pool, player_id).await;
    let mut active_slot = old_active;
    if !bandolier_items.iter().any(|(slot, _)| *slot == active_slot) {
        active_slot = bandolier_items
            .iter()
            .map(|(slot, _)| *slot)
            .min()
            .unwrap_or(0);
        match sqlx::query("UPDATE sgw_player SET bandolier_slot = $1 WHERE player_id = $2")
            .bind(active_slot)
            .bind(player_id)
            .execute(pool.as_ref())
            .await
        {
            Ok(_) => {
                tracing::debug!(entity_id, player_id, active_slot, "Bandolier active slot updated");
            }
            Err(e) => {
                tracing::error!(entity_id, player_id, active_slot, "Failed to update bandolier slot: {e}");
            }
        }
    }

    if active_slot != old_active {
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&3i32.to_le_bytes());
        args.extend_from_slice(&(active_slot + 1).to_le_bytes());
        helpers::send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_ACTIVE_SLOT_UPDATE,
                    &args,
                )
            },
        )
        .await;
    }

    let _ = tx
        .send(BaseToCellMsg::SyncBandolierItems {
            entity_id,
            active_bandolier_slot: active_slot,
            bandolier_items,
        })
        .await;
}
