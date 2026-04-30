use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use super::super::super::super::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::purchase_helpers::normalize_item_quantities;
use super::store::handle_open_vendor_store;
use super::data::load_vendor_sell_prices;
use super::serializers::reserve_free_inventory_slots;
use super::helpers::{send_cash_changed_to_client, sync_bandolier_after_inventory_change};
use super::purchase_helpers::load_vendor_template_lists;

const INV_BANDOLIER: i32 = 3;
const INV_BUYBACK: i32 = 16;
const ITEM_FLAG_CAN_BE_SOLD: i32 = 1 << 10;
use super::VENDOR_FILTER_BAGS;

#[derive(sqlx::FromRow)]
struct SellInventoryRow {
    item_id: i32,
    stack_size: i32,
    container_id: i32,
    unit_price: i32,
}

/// Transactionally sell owned items to a vendor and refresh inventory/cash.
pub async fn handle_sell_vendor_items(
    entity_id: u32,
    player_id: i32,
    vendor_entity_id: i32,
    vendor_template_id: i32,
    items: Vec<(i32, i32)>,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(pool) => pool,
        None => {
            tracing::debug!(entity_id, player_id, "SellVendorItems: no DB pool");
            return;
        }
    };

    let items = normalize_item_quantities(items, false);
    if items.is_empty() {
        tracing::debug!(entity_id, player_id, "SellVendorItems: empty item list");
        return;
    }

    let Some(template) =
        load_vendor_template_lists(pool, vendor_template_id, "SellVendorItems").await
    else {
        return;
    };
    let Some(sell_item_list) = template.sell_item_list else {
        tracing::debug!(
            entity_id,
            player_id,
            vendor_template_id,
            "SellVendorItems: vendor has no sell list"
        );
        return;
    };

    let requested_item_ids: Vec<i32> = items.iter().map(|(item_id, _)| *item_id).collect();
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(entity_id, player_id, "SellVendorItems: begin failed: {e}");
            return;
        }
    };

    let sell_rows = match sqlx::query_as::<_, SellInventoryRow>(
        "SELECT inv.item_id, inv.stack_size, inv.container_id, ili.naquadah AS unit_price \
         FROM sgw_inventory inv \
         JOIN resources.items ri ON ri.item_id = inv.type_id \
         JOIN resources.item_list_items ili \
           ON ili.item_list_id = $3 AND ili.design_id = inv.type_id \
         WHERE inv.character_id = $1 \
           AND inv.item_id = ANY($2) \
           AND inv.container_id = ANY($4) \
           AND inv.bound = false \
           AND (ri.flags & $5) <> 0 \
         FOR UPDATE OF inv",
    )
    .bind(player_id)
    .bind(&requested_item_ids)
    .bind(sell_item_list)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .bind(ITEM_FLAG_CAN_BE_SOLD)
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
                "SellVendorItems: inventory query failed: {e}"
            );
            return;
        }
    };

    let rows_by_id: HashMap<i32, SellInventoryRow> = sell_rows
        .into_iter()
        .map(|row| (row.item_id, row))
        .collect();

    let mut buyback_slots =
        match reserve_free_inventory_slots(&mut tx, player_id, INV_BUYBACK, items.len()).await {
            Ok(Some(slots)) => slots.into_iter(),
            Ok(None) => {
                let _ = tx.rollback().await;
                tracing::warn!(
                    entity_id,
                    player_id,
                    requested_items = items.len(),
                    "SellVendorItems: not enough buyback slots"
                );
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    entity_id,
                    player_id,
                    "SellVendorItems: buyback slot query failed: {e}"
                );
                return;
            }
        };

    let mut total_cash_gain = 0i32;
    let mut removed_items = Vec::new();
    let mut bandolier_changed = false;

    for (item_id, quantity) in &items {
        let Some(row) = rows_by_id.get(item_id) else {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                item_id,
                "SellVendorItems: item not sellable to this vendor"
            );
            return;
        };

        if *quantity > row.stack_size {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                item_id,
                quantity,
                stack_size = row.stack_size,
                "SellVendorItems: requested quantity exceeds stack"
            );
            return;
        }

        let Some(line_cash_gain) = row.unit_price.checked_mul(*quantity) else {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                item_id,
                "SellVendorItems: line cash overflow"
            );
            return;
        };

        total_cash_gain = match total_cash_gain.checked_add(line_cash_gain) {
            Some(total) => total,
            None => {
                let _ = tx.rollback().await;
                tracing::warn!(entity_id, player_id, "SellVendorItems: cash gain overflow");
                return;
            }
        };

        let Some(buyback_slot) = buyback_slots.next() else {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                item_id,
                "SellVendorItems: buyback slot reservation exhausted"
            );
            return;
        };

        let result = if *quantity >= row.stack_size {
            // Pin the UPDATE to the (character, item, container) triple that was
            // just locked above so we cannot accidentally relocate a same-item-id
            // row from a different container if the schema's uniqueness ever
            // weakens.
            sqlx::query(
                "UPDATE sgw_inventory \
                 SET container_id = $1, slot_id = $2, flags = $3 \
                 WHERE character_id = $4 AND item_id = $5 AND container_id = $6",
            )
            .bind(INV_BUYBACK)
            .bind(buyback_slot)
            .bind(row.unit_price)
            .bind(player_id)
            .bind(item_id)
            .bind(row.container_id)
            .execute(&mut *tx)
            .await
        } else {
            let insert_result = sqlx::query(
                "INSERT INTO sgw_inventory \
                 (character_id, type_id, stack_size, slot_id, container_id, bound, \
                  durability, charges, ammo_type, ammo_types, ammo, flags) \
                 SELECT character_id, type_id, $1, $2, $3, bound, durability, charges, \
                        ammo_type, ammo_types, ammo, $4 \
                 FROM sgw_inventory \
                 WHERE character_id = $5 AND item_id = $6 AND container_id = $7 AND stack_size >= $1",
            )
            .bind(quantity)
            .bind(buyback_slot)
            .bind(INV_BUYBACK)
            .bind(row.unit_price)
            .bind(player_id)
            .bind(item_id)
            .bind(row.container_id)
            .execute(&mut *tx)
            .await;

            match insert_result {
                Ok(r) if r.rows_affected() == 1 => {
                    sqlx::query(
                        "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
                 WHERE character_id = $2 AND item_id = $3 AND container_id = $4 AND stack_size >= $1",
                    )
                    .bind(quantity)
                    .bind(player_id)
                    .bind(item_id)
                    .bind(row.container_id)
                    .execute(&mut *tx)
                    .await
                }
                Ok(_) => {
                    let _ = tx.rollback().await;
                    tracing::warn!(
                        entity_id,
                        player_id,
                        item_id,
                        "SellVendorItems: failed to copy item into buyback"
                    );
                    return;
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    tracing::error!(
                        entity_id,
                        player_id,
                        item_id,
                        "SellVendorItems: buyback insert failed: {e}"
                    );
                    return;
                }
            }
        };

        match result {
            Ok(r) if r.rows_affected() == 1 => {
                if *quantity >= row.stack_size {
                    removed_items.push((*item_id, row.container_id));
                }
                if row.container_id == INV_BANDOLIER {
                    bandolier_changed = true;
                }
            }
            Ok(_) => {
                let _ = tx.rollback().await;
                tracing::warn!(
                    entity_id,
                    player_id,
                    item_id,
                    "SellVendorItems: no rows changed"
                );
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    entity_id,
                    player_id,
                    item_id,
                    "SellVendorItems: inventory update failed: {e}"
                );
                return;
            }
        }
    }

    let new_cash_total = if total_cash_gain > 0 {
        // Pre-validate that current naquadah + total_cash_gain fits in i32
        // (sgw_player.naquadah is INT4). Without this check the UPDATE below
        // would error on overflow and rollback the entire sale, surfacing as
        // a confusing failure to the player. Pre-checking lets us return a
        // clean refusal instead of relying on PostgreSQL to error.
        let current_balance: i32 =
            match sqlx::query_scalar::<_, i32>("SELECT naquadah FROM sgw_player WHERE player_id = $1 FOR UPDATE")
                .bind(player_id)
                .fetch_optional(&mut *tx)
                .await
            {
                Ok(Some(b)) => b,
                Ok(None) => {
                    let _ = tx.rollback().await;
                    tracing::warn!(entity_id, player_id, "SellVendorItems: player not found");
                    return;
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    tracing::error!(entity_id, player_id, "SellVendorItems: balance read failed: {e}");
                    return;
                }
            };

        if current_balance.checked_add(total_cash_gain).is_none() {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id, player_id, current_balance, total_cash_gain,
                "SellVendorItems: refusing sale — naquadah balance would overflow i32"
            );
            return;
        }

        match sqlx::query_scalar::<_, i32>(
            "UPDATE sgw_player SET naquadah = naquadah + $1 \
             WHERE player_id = $2 RETURNING naquadah",
        )
        .bind(total_cash_gain)
        .bind(player_id)
        .fetch_optional(&mut *tx)
        .await
        {
            Ok(Some(total)) => Some(total),
            Ok(None) => {
                let _ = tx.rollback().await;
                tracing::warn!(entity_id, player_id, "SellVendorItems: player not found");
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    entity_id,
                    player_id,
                    "SellVendorItems: cash update failed: {e}"
                );
                return;
            }
        }
    } else {
        None
    };

    if let Err(e) = tx.commit().await {
        tracing::error!(entity_id, player_id, "SellVendorItems: commit failed: {e}");
        return;
    }

    if let Some(total) = new_cash_total {
        send_cash_changed_to_client(entity_id, total, socket, connected, entity_to_addr).await;
    }

    let total_items = send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        socket,
        connected,
        entity_to_addr,
    )
    .await;

    // Note: a per-item store update would be redundant here — handle_open_vendor_store
    // below sends a complete store payload, which already supersedes any
    // incremental price clears we'd compute from `removed_items`.

    if let Some(cell_tx) = cell_tx {
        for (item_id, container_id) in &removed_items {
            let _ = cell_tx
                .send(BaseToCellMsg::InventoryItemRemoved {
                    entity_id,
                    item_id: *item_id,
                    source_container_id: *container_id,
                })
                .await;
        }
    }

    if bandolier_changed {
        sync_bandolier_after_inventory_change(
            entity_id,
            player_id,
            db_pool,
            cell_tx,
            socket,
            connected,
            entity_to_addr,
        )
        .await;
    }

    handle_open_vendor_store(
        entity_id,
        player_id,
        vendor_entity_id,
        Some(vendor_template_id),
        db_pool,
        socket,
        connected,
        entity_to_addr,
    )
    .await;

    tracing::debug!(
        entity_id,
        player_id,
        vendor_template_id,
        item_count = items.len(),
        total_items,
        cash_gained = total_cash_gain,
        "Vendor sale completed"
    );
}