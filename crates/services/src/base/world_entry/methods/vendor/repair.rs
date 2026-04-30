use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use super::super::super::super::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::super::inventory::grant::normalize_item_ids;

/// Containers that can be operated on by the vendor stack — main bag, bandolier,
/// equipment slots, and quick bars. Bank, mail attachments, and loot are excluded.
use super::VENDOR_FILTER_BAGS;

pub async fn handle_repair_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    repair_ratio: f32,
    db_pool: &Option<Arc<PgPool>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "RepairInventoryItem: no DB pool");
            return;
        }
    };

    if item_id <= 0 || !repair_ratio.is_finite() || repair_ratio <= 0.0 {
        tracing::warn!(
            player_id,
            item_id,
            repair_ratio,
            "RepairInventoryItem: invalid item or ratio"
        );
        return;
    }

    // Tiny ratios (<0.005) round down to 0 points, which then matches no rows
    // in the WHERE clause and silently reports "no repairable item changed."
    // Floor-clamp so any non-zero ratio repairs at least one durability point.
    let repair_points = ((repair_ratio.clamp(0.0, 1.0) * 100.0).round() as i32).max(1);
    let result = sqlx::query(
        "UPDATE sgw_inventory \
         SET durability = LEAST(100, GREATEST(0, durability) + $1) \
         WHERE character_id = $2 AND item_id = $3 \
           AND container_id = ANY($4) \
           AND durability >= 0 AND durability < 100",
    )
    .bind(repair_points)
    .bind(player_id)
    .bind(item_id)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(r) if r.rows_affected() == 1 => {
            let total_items = send_full_inventory_update(
                entity_id,
                player_id,
                pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
            tracing::debug!(
                entity_id,
                player_id,
                item_id,
                repair_points,
                total_items,
                "Inventory item repaired"
            );
        }
        Ok(_) => {
            tracing::debug!(
                player_id,
                item_id,
                repair_points,
                "RepairInventoryItem: no repairable item changed"
            );
        }
        Err(e) => {
            tracing::error!(
                player_id,
                item_id,
                "RepairInventoryItem: update failed: {e}"
            );
        }
    }
}

pub async fn handle_repair_inventory_items(
    entity_id: u32,
    player_id: i32,
    item_ids: Vec<i32>,
    vendor_template_id: Option<i32>,
    db_pool: &Option<Arc<PgPool>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if let Some(vendor_template_id) = vendor_template_id {
        super::paid_repair::handle_paid_repair_inventory_items(
            entity_id,
            player_id,
            item_ids,
            vendor_template_id,
            db_pool,
            socket,
            connected,
            entity_to_addr,
        )
        .await;
        return;
    }

    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, "RepairInventoryItems: no DB pool");
            return;
        }
    };

    let item_ids = normalize_item_ids(item_ids);
    if item_ids.is_empty() {
        tracing::debug!(
            entity_id,
            player_id,
            "RepairInventoryItems: empty item list"
        );
        return;
    }

    let result = sqlx::query(
        "UPDATE sgw_inventory \
         SET durability = 100 \
         WHERE character_id = $1 \
           AND item_id = ANY($2) \
           AND container_id = ANY($3) \
           AND stack_size = 1 \
           AND durability >= 0 \
           AND durability < 100",
    )
    .bind(player_id)
    .bind(&item_ids)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            let total_items = send_full_inventory_update(
                entity_id,
                player_id,
                pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
            tracing::debug!(
                entity_id,
                player_id,
                item_count = item_ids.len(),
                repaired = r.rows_affected(),
                total_items,
                "Inventory items repaired"
            );
        }
        Ok(_) => tracing::debug!(
            entity_id,
            player_id,
            item_count = item_ids.len(),
            "RepairInventoryItems: no repairable items changed"
        ),
        Err(e) => tracing::error!(
            entity_id,
            player_id,
            item_count = item_ids.len(),
            "RepairInventoryItems: update failed: {e}"
        ),
    }
}
