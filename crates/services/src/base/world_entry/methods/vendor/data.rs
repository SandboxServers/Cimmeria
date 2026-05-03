use std::sync::Arc;

use sqlx::PgPool;

use super::serializers::{StoreBuyCost, StoreItem, StoreItemCost};

const COST_NAQUADAH: i32 = 1;
const COST_ITEM: i32 = 2;
const ITEM_FLAG_CAN_BE_SOLD: i32 = 1 << 10;
const INV_BUYBACK: i32 = 16;
use super::VENDOR_FILTER_BAGS;

// In INV_BUYBACK rows, the `flags` column stores the unit buyback price.
// Pending/uninitialized buyback rows leave flags negative. The buyback list
// query and clear-buyback bookkeeping use `flags > 0` as the sentinel "this
// row was sell-stamped at a real price" check — `flags = 0` rows are excluded
// to prevent free-buyback exploits where a zero-priced row would appear as a
// free item in the buyback list. If you change that convention (e.g., move
// price to a dedicated column), update both sites.

#[derive(sqlx::FromRow)]
struct ItemListItemRow {
    item_id: i32,
    design_id: i32,
    quantity: i32,
    naquadah: i32,
}

#[derive(sqlx::FromRow)]
struct ItemListPriceRow {
    item_id: i32,
    design_id: i32,
    quantity: i32,
}

#[derive(sqlx::FromRow)]
struct StoreItemCostRow {
    cost: i32,
    item_id: i32,
}

/// Load buyable items from a vendor's item list.
pub async fn load_store_buy_items(pool: &Arc<PgPool>, item_list_id: Option<i32>) -> Vec<StoreItem> {
    let Some(item_list_id) = item_list_id else {
        return Vec::new();
    };

    let rows = match sqlx::query_as::<_, ItemListItemRow>(
        "SELECT item_id, design_id, quantity, naquadah \
         FROM resources.item_list_items \
         WHERE item_list_id = $1 ORDER BY item_id",
    )
    .bind(item_list_id)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(item_list_id, "OpenVendorStore: buy list query failed: {e}");
            return Vec::new();
        }
    };

    let list_item_ids: Vec<i32> = rows.iter().map(|r| r.item_id).collect();
    let price_rows = if list_item_ids.is_empty() {
        Vec::new()
    } else {
        match sqlx::query_as::<_, ItemListPriceRow>(
            "SELECT item_id, design_id, quantity \
             FROM resources.item_list_prices \
             WHERE item_id = ANY($1) ORDER BY item_id, design_id",
        )
        .bind(&list_item_ids)
        .fetch_all(pool.as_ref())
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                // Bail rather than display items with missing prerequisite costs
                // — purchase validation rejects on prereq mismatch, so showing
                // a "free" item that the player can't actually buy is worse
                // than showing an empty store.
                tracing::error!(
                    item_list_id,
                    "OpenVendorStore: item price query failed, refusing to display partial store: {e}"
                );
                return Vec::new();
            }
        }
    };

    rows.into_iter()
        .enumerate()
        .map(|(index, row)| {
            let mut cost_list = Vec::new();
            if row.naquadah > 0 {
                cost_list.push(StoreBuyCost {
                    cost_type: COST_NAQUADAH,
                    type_id: 0,
                    quantity: row.naquadah,
                });
            }
            for price in price_rows.iter().filter(|p| p.item_id == row.item_id) {
                cost_list.push(StoreBuyCost {
                    cost_type: COST_ITEM,
                    type_id: price.design_id,
                    quantity: price.quantity,
                });
            }

            StoreItem {
                index: index as i32,
                item_id: row.design_id,
                cost_list,
                usable: 0,
                quantity: row.quantity,
            }
        })
        .collect()
}

/// Load sellable prices from player inventory based on vendor item list.
pub async fn load_vendor_sell_prices(
    pool: &Arc<PgPool>,
    player_id: i32,
    item_list_id: Option<i32>,
) -> Vec<StoreItemCost> {
    let Some(item_list_id) = item_list_id else {
        return Vec::new();
    };

    match sqlx::query_as::<_, StoreItemCostRow>(
        "SELECT ili.naquadah AS cost, inv.item_id \
         FROM resources.item_list_items ili \
         JOIN sgw_inventory inv ON inv.type_id = ili.design_id \
         JOIN resources.items ri ON ri.item_id = inv.type_id \
         WHERE ili.item_list_id = $1 AND inv.character_id = $2 \
           AND inv.container_id = ANY($3) AND inv.bound = false \
           AND (ri.flags & $4) <> 0 \
         ORDER BY inv.container_id, inv.slot_id",
    )
    .bind(item_list_id)
    .bind(player_id)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .bind(ITEM_FLAG_CAN_BE_SOLD)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|row| StoreItemCost {
                cost: row.cost,
                item_id: row.item_id,
            })
            .collect(),
        Err(e) => {
            tracing::error!(
                item_list_id,
                "OpenVendorStore: sell price query failed: {e}"
            );
            Vec::new()
        }
    }
}

/// Load buyback prices from player buyback inventory (container 16).
///
/// Filters `flags > 0` (not `>= 0`) so rows stamped at zero unit price (e.g.,
/// items sold while their per-stack value happened to round to zero) don't
/// appear as free buyback offerings.
pub async fn load_vendor_buyback_prices(pool: &Arc<PgPool>, player_id: i32) -> Vec<StoreItemCost> {
    match sqlx::query_as::<_, StoreItemCostRow>(
        "SELECT flags AS cost, item_id \
         FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2 AND flags > 0 \
         ORDER BY slot_id",
    )
    .bind(player_id)
    .bind(INV_BUYBACK)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|row| StoreItemCost {
                cost: row.cost,
                item_id: row.item_id,
            })
            .collect(),
        Err(e) => {
            tracing::error!(player_id, "OpenVendorStore: buyback query failed: {e}");
            Vec::new()
        }
    }
}

/// Load repair prices for damaged items in player inventory.
pub async fn load_vendor_repair_prices(
    pool: &Arc<PgPool>,
    player_id: i32,
    item_list_id: Option<i32>,
) -> Vec<StoreItemCost> {
    let Some(item_list_id) = item_list_id else {
        return Vec::new();
    };

    match sqlx::query_as::<_, StoreItemCostRow>(
        "SELECT GREATEST((ili.naquadah::BIGINT * (100 - inv.durability)::BIGINT) / 100, 1)::INT AS cost, inv.item_id \
         FROM resources.item_list_items ili \
         JOIN sgw_inventory inv ON inv.type_id = ili.design_id \
         WHERE ili.item_list_id = $1 AND inv.character_id = $2 \
           AND inv.container_id = ANY($3) AND inv.stack_size = 1 \
           AND inv.durability >= 0 AND inv.durability < 100 \
         ORDER BY inv.container_id, inv.slot_id",
    )
    .bind(item_list_id)
    .bind(player_id)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|row| StoreItemCost {
                cost: row.cost,
                item_id: row.item_id,
            })
            .collect(),
        Err(e) => {
            tracing::error!(item_list_id, "OpenVendorStore: repair price query failed: {e}");
            Vec::new()
        }
    }
}

/// Load recharge prices for items with charges depleted.
pub async fn load_vendor_recharge_prices(
    pool: &Arc<PgPool>,
    player_id: i32,
    item_list_id: Option<i32>,
) -> Vec<StoreItemCost> {
    let Some(item_list_id) = item_list_id else {
        return Vec::new();
    };

    match sqlx::query_as::<_, StoreItemCostRow>(
        "SELECT GREATEST((ili.naquadah::BIGINT * (ri.charges - inv.charges)::BIGINT) / NULLIF(ri.charges, 0)::BIGINT, 1)::INT AS cost, inv.item_id \
         FROM resources.item_list_items ili \
         JOIN sgw_inventory inv ON inv.type_id = ili.design_id \
         JOIN resources.items ri ON ri.item_id = inv.type_id \
         WHERE ili.item_list_id = $1 AND inv.character_id = $2 \
           AND inv.container_id = ANY($3) AND inv.stack_size = 1 \
           AND ri.charges > 0 AND inv.charges < ri.charges \
         ORDER BY inv.container_id, inv.slot_id",
    )
    .bind(item_list_id)
    .bind(player_id)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|row| StoreItemCost {
                cost: row.cost,
                item_id: row.item_id,
            })
            .collect(),
        Err(e) => {
            tracing::error!(item_list_id, "OpenVendorStore: recharge price query failed: {e}");
            Vec::new()
        }
    }
}
