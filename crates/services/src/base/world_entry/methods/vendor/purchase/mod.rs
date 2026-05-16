use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use super::super::super::super::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::helpers::send_cash_changed_to_client;
use super::purchase_helpers::{
    consume_design_quantity, load_vendor_purchase_lines, normalize_item_quantities,
};
use super::store::handle_open_vendor_store;
use crate::base::outbox::{self, CellOutboxPayload};
use crate::cell::messages::BaseToCellMsg;

#[cfg(test)]
mod tests;

const INV_MAIN: i32 = 1;

/// Transactionally purchase vendor-list entries and refresh inventory/cash.
pub async fn handle_purchase_vendor_items(
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
            tracing::debug!(entity_id, player_id, "PurchaseVendorItems: no DB pool");
            return;
        }
    };

    let items = normalize_item_quantities(items, true);
    if items.is_empty() {
        tracing::debug!(entity_id, player_id, "PurchaseVendorItems: empty item list");
        return;
    }

    let Some(lines) = load_vendor_purchase_lines(pool, vendor_template_id, &items).await else {
        return;
    };

    if lines.iter().any(|line| line.cash_cost < 0) {
        tracing::warn!(
            entity_id,
            player_id,
            vendor_template_id,
            "PurchaseVendorItems: rejecting purchase containing negative cash_cost line"
        );
        return;
    }

    let total_cash_cost = match lines
        .iter()
        .try_fold(0i32, |total, line| total.checked_add(line.cash_cost))
    {
        Some(total) if total >= 0 => total,
        Some(total) => {
            tracing::warn!(
                entity_id,
                player_id,
                vendor_template_id,
                total,
                "PurchaseVendorItems: rejecting negative aggregate cash cost"
            );
            return;
        }
        None => {
            tracing::warn!(
                entity_id,
                player_id,
                vendor_template_id,
                "PurchaseVendorItems: cash cost overflow"
            );
            return;
        }
    };

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                "PurchaseVendorItems: begin failed: {e}"
            );
            return;
        }
    };

    // Lock acquisition order: sgw_inventory rows (via consume_design_quantity's
    // FOR UPDATE prereq lookup) BEFORE sgw_player.naquadah (FOR UPDATE balance
    // read below). Matches the convention documented in paid_repair.rs and
    // shared by the rest of the vendor stack — locking the player row first
    // would deadlock against concurrent paid_repair/paid_recharge/sell/buyback
    // operations on the same player.
    for line in &lines {
        for (design_id, quantity) in &line.item_costs {
            match consume_design_quantity(&mut tx, player_id, *design_id, *quantity).await {
                Ok(true) => {}
                Ok(false) => {
                    if let Err(e) = tx.rollback().await {
                        tracing::error!("DB rollback failed: {e}");
                    }
                    tracing::warn!(
                        entity_id,
                        player_id,
                        design_id,
                        quantity,
                        "PurchaseVendorItems: missing item prerequisite"
                    );
                    return;
                }
                Err(e) => {
                    if let Err(e) = tx.rollback().await {
                        tracing::error!("DB rollback failed: {e}");
                    }
                    tracing::error!(
                        entity_id,
                        player_id,
                        design_id,
                        quantity,
                        "PurchaseVendorItems: item prerequisite consume failed: {e}"
                    );
                    return;
                }
            }
        }
    }

    let balance: Option<i32> =
        match sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1 FOR UPDATE")
            .bind(player_id)
            .fetch_optional(&mut *tx)
            .await
        {
            Ok(balance) => balance,
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::error!(
                    entity_id,
                    player_id,
                    "PurchaseVendorItems: balance query failed: {e}"
                );
                return;
            }
        };

    let Some(balance) = balance else {
        if let Err(e) = tx.rollback().await {
            tracing::error!("DB rollback failed: {e}");
        }
        tracing::warn!(
            entity_id,
            player_id,
            "PurchaseVendorItems: player not found"
        );
        return;
    };

    if balance < total_cash_cost {
        if let Err(e) = tx.rollback().await {
            tracing::error!("DB rollback failed: {e}");
        }
        tracing::warn!(
            entity_id,
            player_id,
            balance,
            total_cash_cost,
            "PurchaseVendorItems: insufficient naquadah"
        );
        return;
    }

    let new_cash_total = if total_cash_cost > 0 {
        match sqlx::query_scalar::<_, i32>(
            "UPDATE sgw_player SET naquadah = naquadah - $1 WHERE player_id = $2 RETURNING naquadah",
        )
        .bind(total_cash_cost)
        .bind(player_id)
        .fetch_optional(&mut *tx)
        .await
        {
            Ok(Some(total)) => total,
            Ok(None) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::warn!(entity_id, player_id, "PurchaseVendorItems: player disappeared before cash update");
                return;
            }
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::error!(entity_id, player_id, "PurchaseVendorItems: cash update failed: {e}");
                return;
            }
        }
    } else {
        balance
    };

    let mut grant_slots = match super::serializers::reserve_free_inventory_slots(
        &mut tx,
        player_id,
        INV_MAIN,
        lines.len(),
    )
    .await
    {
        Ok(Some(slots)) => slots.into_iter(),
        Ok(None) => {
            if let Err(e) = tx.rollback().await {
                tracing::error!("DB rollback failed: {e}");
            }
            tracing::warn!(
                entity_id,
                player_id,
                requested_items = lines.len(),
                "PurchaseVendorItems: not enough main inventory slots"
            );
            return;
        }
        Err(e) => {
            if let Err(e) = tx.rollback().await {
                tracing::error!("DB rollback failed: {e}");
            }
            tracing::error!(
                entity_id,
                player_id,
                "PurchaseVendorItems: slot query failed: {e}"
            );
            return;
        }
    };

    let mut granted: Vec<(i32, i32, i32)> = Vec::with_capacity(lines.len()); // (design_id, slot, quantity)
    for line in &lines {
        let Some(next_slot) = grant_slots.next() else {
            if let Err(e) = tx.rollback().await {
                tracing::error!("DB rollback failed: {e}");
            }
            tracing::warn!(
                entity_id,
                player_id,
                design_id = line.design_id,
                "PurchaseVendorItems: slot reservation exhausted"
            );
            return;
        };
        granted.push((line.design_id, next_slot, line.grant_quantity));

        let result = sqlx::query(
            "INSERT INTO sgw_inventory \
             (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
             SELECT $1, ri.item_id, $2, $3, $4, false, 100, ri.charges \
             FROM resources.items ri WHERE ri.item_id = $5",
        )
        .bind(player_id)
        .bind(line.grant_quantity)
        .bind(next_slot)
        .bind(INV_MAIN)
        .bind(line.design_id)
        .execute(&mut *tx)
        .await;

        match result {
            Ok(r) if r.rows_affected() == 1 => {}
            Ok(_) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::warn!(
                    entity_id,
                    player_id,
                    design_id = line.design_id,
                    "PurchaseVendorItems: item design missing"
                );
                return;
            }
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::error!(
                    entity_id,
                    player_id,
                    design_id = line.design_id,
                    "PurchaseVendorItems: inventory insert failed: {e}"
                );
                return;
            }
        }
    }

    // Enqueue one outbox row per granted item inside the same tx so the
    // entire purchase (cash debit + N inventory inserts + N cell
    // notifications) is atomic. If any outbox INSERT fails we abort the
    // whole purchase.
    let mut outbox_pending: Vec<(i64, CellOutboxPayload)> = Vec::with_capacity(granted.len());
    for (design_id, slot_id, quantity) in &granted {
        let payload = CellOutboxPayload::InventoryItemGranted {
            item_id: *design_id,
            container_id: INV_MAIN,
            slot_id: *slot_id,
            quantity: *quantity,
        };
        match outbox::enqueue_in_tx(&mut tx, entity_id, &payload).await {
            Ok(id) => outbox_pending.push((id, payload)),
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::error!(
                    entity_id,
                    player_id,
                    design_id,
                    "PurchaseVendorItems: outbox enqueue failed, aborting: {e}"
                );
                return;
            }
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(
            entity_id,
            player_id,
            "PurchaseVendorItems: commit failed: {e}"
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

    if let Some(cell_tx) = cell_tx {
        for (outbox_id, payload) in outbox_pending {
            outbox::try_dispatch_now(pool.as_ref(), cell_tx, outbox_id, entity_id, payload).await;
        }
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
        cash_spent = total_cash_cost,
        "Vendor purchase completed"
    );
}
