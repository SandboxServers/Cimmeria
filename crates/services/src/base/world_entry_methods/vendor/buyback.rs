use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use super::super::super::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::purchase_helpers::normalize_item_quantities;
use super::store::handle_open_vendor_store;
use super::serializers::reserve_free_inventory_slots;
use super::helpers::send_cash_changed_to_client;

const INV_MAIN: i32 = 1;
const INV_BUYBACK: i32 = 16;

#[derive(sqlx::FromRow)]
struct BuybackInventoryRow {
    item_id: i32,
    stack_size: i32,
    unit_price: i32,
}

/// Transactionally buy back recently sold items and refresh inventory/cash/store.
pub async fn handle_buyback_vendor_items(
    entity_id: u32,
    player_id: i32,
    vendor_entity_id: i32,
    vendor_template_id: i32,
    items: Vec<(i32, i32)>,
    db_pool: &Option<Arc<PgPool>>,
    // Buyback intentionally does not emit InventoryItemGranted to the cell:
    // the moved row keeps the same `item_id` (the only identifier the cell
    // tracks), so a client-side `send_full_inventory_update` after the tx
    // commits is sufficient to resync. If a future change introduces a new
    // item_id during buyback (e.g., a stack split that creates a new row),
    // wire `cell_tx` through and emit InventoryItemGranted for the new id.
    _cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(pool) => pool,
        None => {
            tracing::debug!(entity_id, player_id, "BuybackVendorItems: no DB pool");
            return;
        }
    };

    let items = normalize_item_quantities(items, false);
    if items.is_empty() {
        tracing::debug!(entity_id, player_id, "BuybackVendorItems: empty item list");
        return;
    }

    let requested_item_ids: Vec<i32> = items.iter().map(|(item_id, _)| *item_id).collect();
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                "BuybackVendorItems: begin failed: {e}"
            );
            return;
        }
    };

    // Lock acquisition order: sgw_inventory rows BEFORE sgw_player.naquadah,
    // matching the convention documented in paid_repair.rs and shared by the
    // rest of the vendor stack. Reading the balance first would invert this
    // order and risk a deadlock against concurrent grant/move operations.
    let buyback_rows = match sqlx::query_as::<_, BuybackInventoryRow>(
        "SELECT item_id, stack_size, flags AS unit_price \
         FROM sgw_inventory \
         WHERE character_id = $1 \
           AND item_id = ANY($2) \
           AND container_id = $3 \
           AND flags > 0 \
         FOR UPDATE",
    )
    .bind(player_id)
    .bind(&requested_item_ids)
    .bind(INV_BUYBACK)
    .fetch_all(&mut *tx)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                "BuybackVendorItems: buyback query failed: {e}"
            );
            return;
        }
    };

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
                    "BuybackVendorItems: balance query failed: {e}"
                );
                return;
            }
        };

    let Some(balance) = balance else {
        let _ = tx.rollback().await;
        tracing::warn!(entity_id, player_id, "BuybackVendorItems: player not found");
        return;
    };

    let rows_by_id: HashMap<i32, BuybackInventoryRow> = buyback_rows
        .into_iter()
        .map(|row| (row.item_id, row))
        .collect();

    let mut total_cash_cost = 0i32;
    for (item_id, quantity) in &items {
        let Some(row) = rows_by_id.get(item_id) else {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                item_id,
                "BuybackVendorItems: item is not in buyback"
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
                "BuybackVendorItems: requested quantity exceeds stack"
            );
            return;
        }

        let Some(line_cash_cost) = row.unit_price.checked_mul(*quantity) else {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                item_id,
                "BuybackVendorItems: line cash overflow"
            );
            return;
        };

        total_cash_cost = match total_cash_cost.checked_add(line_cash_cost) {
            Some(total) => total,
            None => {
                let _ = tx.rollback().await;
                tracing::warn!(
                    entity_id,
                    player_id,
                    "BuybackVendorItems: cash cost overflow"
                );
                return;
            }
        };
    }

    if balance < total_cash_cost {
        let _ = tx.rollback().await;
        tracing::warn!(
            entity_id,
            player_id,
            balance,
            total_cash_cost,
            "BuybackVendorItems: insufficient naquadah"
        );
        return;
    }

    let mut main_slots =
        match reserve_free_inventory_slots(&mut tx, player_id, INV_MAIN, items.len()).await {
            Ok(Some(slots)) => slots.into_iter(),
            Ok(None) => {
                let _ = tx.rollback().await;
                tracing::warn!(
                    entity_id,
                    player_id,
                    requested_items = items.len(),
                    "BuybackVendorItems: not enough main inventory slots"
                );
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    entity_id,
                    player_id,
                    "BuybackVendorItems: main slot query failed: {e}"
                );
                return;
            }
        };

    let new_cash_total = if total_cash_cost > 0 {
        match sqlx::query_scalar::<_, i32>(
            "UPDATE sgw_player SET naquadah = naquadah - $1 \
             WHERE player_id = $2 RETURNING naquadah",
        )
        .bind(total_cash_cost)
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
                    "BuybackVendorItems: player disappeared before cash update"
                );
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    entity_id,
                    player_id,
                    "BuybackVendorItems: cash update failed: {e}"
                );
                return;
            }
        }
    } else {
        balance
    };

    for (item_id, quantity) in &items {
        let Some(row) = rows_by_id.get(item_id) else {
            let _ = tx.rollback().await;
            return;
        };
        let Some(main_slot) = main_slots.next() else {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                item_id,
                "BuybackVendorItems: main slot reservation exhausted"
            );
            return;
        };

        // Flags semantics: in INV_BUYBACK, `flags` stores the per-stack buyback unit
        // price. Full-move clears it to 0 (item is back in main inventory). Partial-move
        // inserts the moved portion with flags=0 but leaves the remaining buyback row's
        // flags untouched so the remainder can still be bought back at the original price.
        let result = if *quantity >= row.stack_size {
            sqlx::query(
                "UPDATE sgw_inventory \
                 SET container_id = $1, slot_id = $2, flags = 0 \
                 WHERE character_id = $3 AND item_id = $4 AND container_id = $5",
            )
            .bind(INV_MAIN)
            .bind(main_slot)
            .bind(player_id)
            .bind(item_id)
            .bind(INV_BUYBACK)
            .execute(&mut *tx)
            .await
        } else {
            let insert_result = sqlx::query(
                "INSERT INTO sgw_inventory \
                 (character_id, type_id, stack_size, slot_id, container_id, bound, \
                  durability, charges, ammo_type, ammo_types, ammo, flags) \
                 SELECT character_id, type_id, $1, $2, $3, bound, durability, charges, \
                        ammo_type, ammo_types, ammo, 0 \
                 FROM sgw_inventory \
                 WHERE character_id = $4 AND item_id = $5 AND container_id = $6 \
                   AND stack_size >= $1",
            )
            .bind(quantity)
            .bind(main_slot)
            .bind(INV_MAIN)
            .bind(player_id)
            .bind(item_id)
            .bind(INV_BUYBACK)
            .execute(&mut *tx)
            .await;

            match insert_result {
                Ok(r) if r.rows_affected() == 1 => {
                    let upd = sqlx::query(
                        "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
                     WHERE character_id = $2 AND item_id = $3 AND container_id = $4 \
                       AND stack_size >= $1",
                    )
                    .bind(quantity)
                    .bind(player_id)
                    .bind(item_id)
                    .bind(INV_BUYBACK)
                    .execute(&mut *tx)
                    .await;

                    // If the partial-buyback decrement just reduced the row to
                    // stack_size = 0, remove it now so it doesn't show up as a
                    // free-looking ghost entry on the next vendor open. Keyed on
                    // the same (character, item, container) tuple as the UPDATE
                    // above so we never delete a row we didn't just touch.
                    if upd.is_ok() {
                        let _ = sqlx::query(
                            "DELETE FROM sgw_inventory \
                             WHERE character_id = $1 AND item_id = $2 \
                               AND container_id = $3 AND stack_size <= 0",
                        )
                        .bind(player_id)
                        .bind(item_id)
                        .bind(INV_BUYBACK)
                        .execute(&mut *tx)
                        .await;
                    }

                    upd
                }
                Ok(_) => {
                    let _ = tx.rollback().await;
                    tracing::warn!(
                        entity_id,
                        player_id,
                        item_id,
                        "BuybackVendorItems: failed to copy item into main inventory"
                    );
                    return;
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    tracing::error!(
                        entity_id,
                        player_id,
                        item_id,
                        "BuybackVendorItems: main inventory insert failed: {e}"
                    );
                    return;
                }
            }
        };

        match result {
            Ok(r) if r.rows_affected() == 1 => {}
            Ok(_) => {
                let _ = tx.rollback().await;
                tracing::warn!(
                    entity_id,
                    player_id,
                    item_id,
                    "BuybackVendorItems: no rows changed"
                );
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    entity_id,
                    player_id,
                    item_id,
                    "BuybackVendorItems: inventory update failed: {e}"
                );
                return;
            }
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(
            entity_id,
            player_id,
            "BuybackVendorItems: commit failed: {e}"
        );
        return;
    }

    if total_cash_cost > 0 {
        send_cash_changed_to_client(entity_id, new_cash_total, socket, connected, entity_to_addr)
            .await;
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
        cash_spent = total_cash_cost,
        "Vendor buyback completed"
    );
}