use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use crate::base::ConnectedClientState;
use super::super::super::inventory::core::send_full_inventory_update;
use super::super::super::inventory::grant::normalize_item_ids;
use super::data::load_vendor_repair_prices;
use super::store::send_store_update_to_client;
use super::helpers::send_cash_changed_to_client;
use super::purchase_helpers::load_vendor_template_lists;
use super::serializers::StoreItemCostUpdate;

const VENDOR_FILTER_BAGS: [i32; 14] = [1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

#[derive(sqlx::FromRow)]
struct StoreItemCostRow {
    cost: i32,
    item_id: i32,
}

/// Transactionally repair items for payment and refresh inventory/cash.
pub async fn handle_paid_repair_inventory_items(
    entity_id: u32,
    player_id: i32,
    item_ids: Vec<i32>,
    vendor_template_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
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

    let Some(template) =
        load_vendor_template_lists(pool, vendor_template_id, "RepairInventoryItems").await
    else {
        return;
    };
    let Some(repair_item_list) = template.repair_item_list else {
        tracing::debug!(
            entity_id,
            player_id,
            vendor_template_id,
            "RepairInventoryItems: vendor has no repair list"
        );
        return;
    };

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                "RepairInventoryItems: begin failed: {e}"
            );
            return;
        }
    };

    let rows = match sqlx::query_as::<_, StoreItemCostRow>(
        "SELECT GREATEST((ili.naquadah * (100 - inv.durability)) / 100, 1)::INT AS cost, \
                inv.item_id \
         FROM resources.item_list_items ili \
         JOIN sgw_inventory inv ON inv.type_id = ili.design_id \
         WHERE ili.item_list_id = $1 \
           AND inv.character_id = $2 \
           AND inv.item_id = ANY($3) \
           AND inv.container_id = ANY($4) \
           AND inv.stack_size = 1 \
           AND inv.durability >= 0 \
           AND inv.durability < 100 \
         FOR UPDATE OF inv",
    )
    .bind(repair_item_list)
    .bind(player_id)
    .bind(&item_ids)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .fetch_all(&mut *tx)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                vendor_template_id,
                "RepairInventoryItems: repair query failed: {e}"
            );
            return;
        }
    };

    let rows_by_id: HashMap<i32, StoreItemCostRow> =
        rows.into_iter().map(|row| (row.item_id, row)).collect();
    let mut total_cost = 0i32;
    for item_id in &item_ids {
        let Some(row) = rows_by_id.get(item_id) else {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                item_id,
                "RepairInventoryItems: item is not repairable at this vendor"
            );
            return;
        };

        total_cost = match total_cost.checked_add(row.cost) {
            Some(total) => total,
            None => {
                let _ = tx.rollback().await;
                tracing::warn!(entity_id, player_id, "RepairInventoryItems: cost overflow");
                return;
            }
        };
    }

    let balance: Option<i32> =
        match sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1 FOR UPDATE")
            .bind(player_id)
            .fetch_optional(&mut *tx)
            .await
        {
            Ok(balance) => balance,
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    entity_id,
                    player_id,
                    "RepairInventoryItems: balance query failed: {e}"
                );
                return;
            }
        };

    let Some(balance) = balance else {
        let _ = tx.rollback().await;
        tracing::warn!(
            entity_id,
            player_id,
            "RepairInventoryItems: player not found"
        );
        return;
    };

    if balance < total_cost {
        let _ = tx.rollback().await;
        tracing::warn!(
            entity_id,
            player_id,
            balance,
            total_cost,
            "RepairInventoryItems: insufficient naquadah"
        );
        return;
    }

    let new_cash_total = match sqlx::query_scalar::<_, i32>(
        "UPDATE sgw_player SET naquadah = naquadah - $1 \
         WHERE player_id = $2 RETURNING naquadah",
    )
    .bind(total_cost)
    .bind(player_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(total)) => total,
        Ok(None) => {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                "RepairInventoryItems: player disappeared before cash update"
            );
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                "RepairInventoryItems: cash update failed: {e}"
            );
            return;
        }
    };

    let result = sqlx::query(
        "UPDATE sgw_inventory SET durability = 100 \
         WHERE character_id = $1 AND item_id = ANY($2)",
    )
    .bind(player_id)
    .bind(&item_ids)
    .execute(&mut *tx)
    .await;

    match result {
        Ok(r) if r.rows_affected() == item_ids.len() as u64 => {}
        Ok(r) => {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                expected = item_ids.len(),
                updated = r.rows_affected(),
                "RepairInventoryItems: unexpected repair update count"
            );
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                "RepairInventoryItems: update failed: {e}"
            );
            return;
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(
            entity_id,
            player_id,
            "RepairInventoryItems: commit failed: {e}"
        );
        return;
    }

    send_cash_changed_to_client(entity_id, new_cash_total, socket, connected, entity_to_addr).await;
    let total_items = send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        socket,
        connected,
        entity_to_addr,
    )
    .await;
    let store_updates: Vec<StoreItemCostUpdate> = item_ids
        .iter()
        .map(|item_id| StoreItemCostUpdate {
            item_id: *item_id,
            sell_price: 0,
            repair_price: 0,
            recharge_price: 0,
        })
        .collect();
    send_store_update_to_client(entity_id, &store_updates, socket, connected, entity_to_addr).await;

    tracing::debug!(
        entity_id,
        player_id,
        vendor_template_id,
        item_count = item_ids.len(),
        total_cost,
        total_items,
        "Vendor repair completed"
    );
}