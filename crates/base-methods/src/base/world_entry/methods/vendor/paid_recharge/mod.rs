use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::inventory::core::send_full_inventory_update;
use super::super::inventory::grant::normalize_item_ids;
use super::helpers::send_cash_changed_to_client;
use super::purchase_helpers::load_vendor_template_lists;
use super::serializers::StoreItemCostUpdate;
use super::store::send_store_update_to_client;
use super::telemetry::{VendorItem, VendorLog};
use crate::base::ConnectedClientState;

use super::VENDOR_FILTER_BAGS;

#[cfg(test)]
mod tests;

#[derive(sqlx::FromRow)]
struct StoreItemCostRow {
    cost: i32,
    item_id: i32,
}

/// Transactionally recharge items for payment and refresh inventory/cash.
#[tracing::instrument(
    name = "vendor.paid_recharge",
    level = "info",
    skip_all,
    fields(entity_id, player_id, vendor_template_id, item_count = item_ids.len()),
)]
pub async fn handle_paid_recharge_inventory_items(
    entity_id: u32,
    player_id: i32,
    item_ids: Vec<i32>,
    vendor_template_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // The vendor's entity id is not on this path (the cell forwards only the
    // template), so the rows carry `vendor_template_id` alone.
    let tel = VendorLog::new(
        "recharge",
        entity_id,
        player_id,
        None,
        Some(vendor_template_id),
        connected,
        entity_to_addr,
    );
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tel.failed("no_database", VendorItem::default(), &"no database pool");
            return;
        }
    };

    let item_ids = normalize_item_ids(item_ids);
    if item_ids.is_empty() {
        tel.refused("empty_request", VendorItem::default());
        return;
    }
    // The single item a one-item request names, for the rows below.
    let only = match item_ids.as_slice() {
        [id] => VendorItem::row(*id),
        _ => VendorItem::default(),
    };

    let Some(template) =
        load_vendor_template_lists(pool, vendor_template_id, "RechargeInventoryItems").await
    else {
        tel.failed("template_lookup_failed", only, &"no vendor template lists");
        return;
    };
    let Some(recharge_item_list) = template.recharge_item_list else {
        tel.refused("vendor_does_not_recharge", only);
        return;
    };

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tel.failed("db_error", only, &e);
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
            tel.failed("db_error", only, &e);
            return;
        }
    };

    let rows_by_id: HashMap<i32, StoreItemCostRow> =
        rows.into_iter().map(|row| (row.item_id, row)).collect();
    let mut total_cost = 0i32;
    for item_id in &item_ids {
        let Some(row) = rows_by_id.get(item_id) else {
            let _ = tx.rollback().await;
            // Not carried, a stack, already full, or not on this vendor's list.
            tel.refused("not_rechargeable", VendorItem::row(*item_id));
            return;
        };

        total_cost = match total_cost.checked_add(row.cost) {
            Some(total) => total,
            None => {
                let _ = tx.rollback().await;
                tel.refused("price_overflow", only);
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
                tel.failed("db_error", only, &e);
                return;
            }
        };

    let Some(balance) = balance else {
        let _ = tx.rollback().await;
        tel.failed("player_missing", only, &"no sgw_player row");
        return;
    };

    if balance < total_cost {
        let _ = tx.rollback().await;
        tel.refused("insufficient_cash", only.price(total_cost).cash(balance));
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
            tel.failed(
                "player_missing",
                only,
                &"player row gone before the cash update",
            );
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tel.failed("db_error", only, &e);
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
            tel.failed(
                "rows_affected_mismatch",
                only,
                &format!("expected {}, updated {}", item_ids.len(), r.rows_affected()),
            );
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tel.failed("db_error", only, &e);
            return;
        }
    }

    if let Err(e) = tx.commit().await {
        tel.failed("commit_failed", only, &e);
        return;
    }
    tel.completed(
        only.price(total_cost),
        item_ids.len(),
        Some(i64::from(balance)),
        Some(i64::from(new_cash_total)),
    );

    send_cash_changed_to_client(
        entity_id,
        new_cash_total,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
    let total_items = send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        transport,
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
    send_store_update_to_client(
        entity_id,
        &store_updates,
        transport,
        connected,
        entity_to_addr,
    )
    .await;

    let player_label = known_names::player_name(player_id);
    tracing::debug!(
        entity_id,
        entity_name = player_label,
        player_id,
        player_name = player_label,
        vendor_template_id,
        vendor_template_name = cimmeria_names::owned::template(vendor_template_id),
        item_count = item_ids.len(),
        total_cost,
        total_items,
        "Vendor recharge completed"
    );
}
