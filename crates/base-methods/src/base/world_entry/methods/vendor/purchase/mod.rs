use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_cell_catalog::crafting::ItemFlags;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
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

mod placement;

#[cfg(test)]
mod concurrency_tests;
#[cfg(test)]
mod crafting_supplies_tests;
#[cfg(test)]
mod tests;

const INV_MAIN: i32 = 1;

/// Transactionally purchase vendor-list entries and refresh inventory/cash.
#[tracing::instrument(
    name = "vendor.purchase",
    level = "info",
    skip_all,
    fields(entity_id, player_id, vendor_entity_id, vendor_template_id, items_len = items.len()),
)]
pub async fn handle_purchase_vendor_items(
    entity_id: u32,
    player_id: i32,
    vendor_entity_id: i32,
    vendor_template_id: i32,
    items: Vec<(i32, i32)>,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
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

    // The player-wide inventory lock comes first, before any row: inventory
    // moves and crafting completions take it first too, and both then take
    // per-bag advisory locks before inventory rows, the reverse of the row
    // -> player -> bag order below. Holding it for the whole purchase
    // serializes the purchase with them instead of letting the two orders
    // deadlock.
    if let Err(e) = sqlx::query("SELECT pg_advisory_xact_lock($1, 0)")
        .bind(player_id)
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        tracing::error!(
            entity_id,
            player_id,
            "PurchaseVendorItems: player inventory lock failed: {e}"
        );
        return;
    }

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
                    let _ = tx.rollback().await;
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
                    let _ = tx.rollback().await;
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
                let _ = tx.rollback().await;
                tracing::error!(
                    entity_id,
                    player_id,
                    "PurchaseVendorItems: balance query failed: {e}"
                );
                return;
            }
        };

    let Some(balance) = balance else {
        let _ = tx.rollback().await;
        tracing::warn!(
            entity_id,
            player_id,
            "PurchaseVendorItems: player not found"
        );
        return;
    };

    if balance < total_cash_cost {
        let _ = tx.rollback().await;
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
                let _ = tx.rollback().await;
                tracing::warn!(entity_id, player_id, "PurchaseVendorItems: player disappeared before cash update");
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(entity_id, player_id, "PurchaseVendorItems: cash update failed: {e}");
                return;
            }
        }
    } else {
        balance
    };

    let design_ids: Vec<i32> = lines.iter().map(|line| line.design_id).collect();
    let placement = match placement::place_lines(&mut tx, player_id, &design_ids).await {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                "PurchaseVendorItems: container_sets lookup failed: {e}"
            );
            return;
        }
    };
    let line_slots =
        match placement::reserve_line_slots(&mut tx, player_id, &placement.containers).await {
            Ok(Some(slots)) => slots,
            Ok(None) => {
                let _ = tx.rollback().await;
                tracing::warn!(
                    entity_id,
                    player_id,
                    requested_items = lines.len(),
                    containers = ?placement.containers,
                    "PurchaseVendorItems: not enough free inventory slots"
                );
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    entity_id,
                    player_id,
                    "PurchaseVendorItems: slot query failed: {e}"
                );
                return;
            }
        };

    // (design_id, container, slot, quantity)
    let mut granted: Vec<(i32, i32, i32, i32)> = Vec::with_capacity(lines.len());
    for ((line, &container_id), &next_slot) in
        lines.iter().zip(&placement.containers).zip(&line_slots)
    {
        granted.push((line.design_id, container_id, next_slot, line.grant_quantity));

        // `bound` is derived from the design's own BIND_ON_ACQUIRE flag
        // (`resources.items.flags & 4`, SS-914) rather than hardcoded
        // `false`: a vendor-exclusive bind-on-acquire item must land bound
        // just as a loot or mission grant does, or it can be mailed/traded
        // straight back off the character that bought it.
        let result = sqlx::query(
            "INSERT INTO sgw_inventory \
             (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
             SELECT $1, ri.item_id, $2, $3, $4, (ri.flags & $6) <> 0, 100, ri.charges \
             FROM resources.items ri WHERE ri.item_id = $5",
        )
        .bind(player_id)
        .bind(line.grant_quantity)
        .bind(next_slot)
        .bind(container_id)
        .bind(line.design_id)
        .bind(ItemFlags::BIND_ON_ACQUIRE as i32)
        .execute(&mut *tx)
        .await;

        match result {
            Ok(r) if r.rows_affected() == 1 => {}
            Ok(_) => {
                let _ = tx.rollback().await;
                tracing::warn!(
                    entity_id,
                    player_id,
                    design_id = line.design_id,
                    "PurchaseVendorItems: item design missing"
                );
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
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
    for (design_id, container_id, slot_id, quantity) in &granted {
        let payload = CellOutboxPayload::InventoryItemGranted {
            item_id: *design_id,
            container_id: *container_id,
            slot_id: *slot_id,
            quantity: *quantity,
        };
        match outbox::enqueue_in_tx(&mut tx, entity_id, &payload).await {
            Ok(id) => outbox_pending.push((id, payload)),
            Err(e) => {
                let _ = tx.rollback().await;
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

    for (design_id, container_id, slot_id, quantity) in &granted {
        tracing::info!(
            target: "inventory",
            event = "grant_container_chosen",
            account_id = placement.account_id,
            player_id,
            entity_id,
            type_id = design_id,
            quantity,
            container_sets = %placement.container_sets_text(*design_id),
            requested_container_id = INV_MAIN,
            skipped_storage = placement.skipped_storage(*design_id, *container_id),
            container_id,
            slot_id,
            qty_before = 0,
            qty_after = quantity,
            source = "vendor_purchase",
            "grant_container_chosen"
        );
    }

    if total_cash_cost > 0 {
        send_cash_changed_to_client(
            entity_id,
            new_cash_total,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }

    let total_items = send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        transport,
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
        transport,
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
