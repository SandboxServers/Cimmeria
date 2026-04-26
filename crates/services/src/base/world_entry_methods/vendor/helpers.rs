use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::base::{ConnectedClientState, helpers};
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx};
use super::super::player_load::meta::query_bandolier_items_tx;

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

    let mut db_tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(entity_id, player_id, "sync_bandolier: begin tx failed: {e}");
            return;
        }
    };

    let old_active: i32 = match sqlx::query_scalar(
        "SELECT bandolier_slot FROM sgw_player WHERE player_id = $1 FOR UPDATE",
    )
    .bind(player_id)
    .fetch_optional(&mut *db_tx)
    .await
    {
        Ok(v) => v.unwrap_or(0),
        Err(e) => {
            let _ = db_tx.rollback().await;
            tracing::error!(entity_id, player_id, "sync_bandolier: read slot failed: {e}");
            return;
        }
    };

    // Read bandolier items *inside* the transaction so the FOR UPDATE lock above
    // protects this read against concurrent inventory mutations on container 3.
    let bandolier_items = match query_bandolier_items_tx(&mut db_tx, player_id).await {
        Ok(items) => items,
        Err(e) => {
            let _ = db_tx.rollback().await;
            tracing::error!(entity_id, player_id, "sync_bandolier: read items failed: {e}");
            return;
        }
    };

    // Empty bandolier: nothing to reconcile, don't write a sentinel slot or
    // emit a Sync that says "active slot 0 of nothing".
    if bandolier_items.is_empty() {
        if let Err(e) = db_tx.commit().await {
            tracing::error!(entity_id, player_id, "sync_bandolier: commit failed: {e}");
        }
        return;
    }

    let mut active_slot = old_active;
    if !bandolier_items.iter().any(|(slot, _)| *slot == active_slot) {
        active_slot = bandolier_items
            .iter()
            .map(|(slot, _)| *slot)
            .min()
            .unwrap_or(old_active);
        if let Err(e) = sqlx::query("UPDATE sgw_player SET bandolier_slot = $1 WHERE player_id = $2")
            .bind(active_slot)
            .bind(player_id)
            .execute(&mut *db_tx)
            .await
        {
            let _ = db_tx.rollback().await;
            tracing::error!(entity_id, player_id, active_slot, "Failed to update bandolier slot: {e}");
            return;
        }
        tracing::debug!(entity_id, player_id, active_slot, "Bandolier active slot updated");
    }

    if let Err(e) = db_tx.commit().await {
        tracing::error!(entity_id, player_id, "sync_bandolier: commit failed: {e}");
        return;
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
