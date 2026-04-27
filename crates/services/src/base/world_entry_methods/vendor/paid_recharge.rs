use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use crate::base::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::super::inventory::grant::normalize_item_ids;
use super::store::send_store_update_to_client;
use super::helpers::send_cash_changed_to_client;
use super::purchase_helpers::load_vendor_template_lists;
use super::serializers::StoreItemCostUpdate;

use super::VENDOR_FILTER_BAGS;

#[derive(sqlx::FromRow)]
struct StoreItemCostRow {
    cost: i32,
    item_id: i32,
}

/// Transactionally recharge items for payment and refresh inventory/cash.
pub async fn handle_paid_recharge_inventory_items(
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

    let Some(template) =
        load_vendor_template_lists(pool, vendor_template_id, "RechargeInventoryItems").await
    else {
        return;
    };
    let Some(recharge_item_list) = template.recharge_item_list else {
        tracing::warn!(
            entity_id,
            player_id,
            vendor_template_id,
            "RechargeInventoryItems: vendor has no recharge list — client request dropped"
        );
        return;
    };

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                "RechargeInventoryItems: begin failed: {e}"
            );
            return;
        }
    };

    let rows = match sqlx::query_as::<_, StoreItemCostRow>(
        "SELECT GREATEST((ili.naquadah::BIGINT * (ri.charges - inv.charges)::BIGINT) / NULLIF(ri.charges, 0)::BIGINT, 1)::INT AS cost, \
                inv.item_id \
         FROM resources.item_list_items ili \
         JOIN sgw_inventory inv ON inv.type_id = ili.design_id \
         JOIN resources.items ri ON ri.item_id = inv.type_id \
         WHERE ili.item_list_id = $1 \
           AND inv.character_id = $2 \
           AND inv.item_id = ANY($3) \
           AND inv.container_id = ANY($4) \
           AND inv.stack_size = 1 \
           AND ri.charges > 0 \
           AND inv.charges < ri.charges \
         FOR UPDATE OF inv",
    )
    .bind(recharge_item_list)
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
                "RechargeInventoryItems: recharge query failed: {e}"
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
                "RechargeInventoryItems: item is not rechargeable at this vendor"
            );
            return;
        };

        total_cost = match total_cost.checked_add(row.cost) {
            Some(total) => total,
            None => {
                let _ = tx.rollback().await;
                tracing::warn!(
                    entity_id,
                    player_id,
                    "RechargeInventoryItems: cost overflow"
                );
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
                    "RechargeInventoryItems: balance query failed: {e}"
                );
                return;
            }
        };

    let Some(balance) = balance else {
        let _ = tx.rollback().await;
        tracing::warn!(
            entity_id,
            player_id,
            "RechargeInventoryItems: player not found"
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
            "RechargeInventoryItems: insufficient naquadah"
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
                "RechargeInventoryItems: player disappeared before cash update"
            );
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                "RechargeInventoryItems: cash update failed: {e}"
            );
            return;
        }
    };

    let result = sqlx::query(
        "UPDATE sgw_inventory inv \
         SET charges = ri.charges \
         FROM resources.items ri \
         WHERE inv.character_id = $1 \
           AND inv.item_id = ANY($2) \
           AND inv.type_id = ri.item_id \
           AND inv.container_id = ANY($3) \
           AND inv.stack_size = 1 \
           AND ri.charges > 0 \
           AND inv.charges < ri.charges",
    )
    .bind(player_id)
    .bind(&item_ids)
    .bind(VENDOR_FILTER_BAGS.as_slice())
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
                "RechargeInventoryItems: unexpected recharge update count"
            );
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                "RechargeInventoryItems: update failed: {e}"
            );
            return;
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(
            entity_id,
            player_id,
            "RechargeInventoryItems: commit failed: {e}"
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
        "Vendor recharge completed"
    );
}