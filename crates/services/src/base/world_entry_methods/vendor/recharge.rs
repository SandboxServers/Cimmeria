use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use super::super::super::ConnectedClientState;
use super::helpers;
use super::super::super::inventory::core;
use super::super::super::inventory::grant;
use super::super::super::inventory::grant::normalize_item_ids;

pub async fn handle_recharge_inventory_items(
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
        super::vendor_paid_recharge::handle_paid_recharge_inventory_items(
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
            tracing::debug!(player_id, "RechargeInventoryItems: no DB pool");
            return;
        }
    };

    let item_ids = normalize_item_ids(item_ids);
    if item_ids.is_empty() {
        tracing::debug!(
            entity_id,
            player_id,
            "RechargeInventoryItems: empty item list"
        );
        return;
    }

    let result = sqlx::query(
        "UPDATE sgw_inventory inv \
         SET charges = ri.charges \
         FROM resources.items ri \
         WHERE inv.character_id = $1 \
           AND inv.item_id = ANY($2) \
           AND inv.type_id = ri.item_id \
           AND inv.stack_size = 1 \
           AND ri.charges > 0 \
           AND inv.charges < ri.charges",
    )
    .bind(player_id)
    .bind(&item_ids)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            let total_items = inventory_core::send_full_inventory_update(
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
                recharged = r.rows_affected(),
                total_items,
                "Inventory items recharged"
            );
        }
        Ok(_) => tracing::debug!(
            entity_id,
            player_id,
            item_count = item_ids.len(),
            "RechargeInventoryItems: no rechargeable items changed"
        ),
        Err(e) => tracing::error!(
            entity_id,
            player_id,
            item_count = item_ids.len(),
            "RechargeInventoryItems: update failed: {e}"
        ),
    }
}
