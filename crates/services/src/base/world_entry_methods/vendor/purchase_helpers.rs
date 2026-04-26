use std::collections::HashMap;
use std::sync::Arc;

use sqlx::{PgPool, Postgres, Transaction};

use super::store::VendorTemplateLists;

const INV_MAIN: i32 = 1;
const VENDOR_COST_BAGS: [i32; 3] = [INV_MAIN, 2, 15];

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
struct VendorPurchaseItemRow {
    store_index: i32,
    list_item_id: i32,
    design_id: i32,
    quantity: i32,
    naquadah: i32,
}

#[derive(sqlx::FromRow)]
struct InventoryStackRow {
    item_id: i32,
    stack_size: i32,
}

#[derive(Debug, Clone)]
pub struct VendorPurchaseLine {
    pub design_id: i32,
    pub grant_quantity: i32,
    pub cash_cost: i32,
    pub item_costs: Vec<(i32, i32)>,
}

/// Load vendor template details from database.
pub async fn load_vendor_template_lists(
    pool: &Arc<PgPool>,
    vendor_template_id: i32,
    context: &str,
) -> Option<VendorTemplateLists> {
    match sqlx::query_as::<_, VendorTemplateLists>(
        "SELECT buy_item_list, sell_item_list, repair_item_list, recharge_item_list \
         FROM resources.entity_templates WHERE template_id = $1",
    )
    .bind(vendor_template_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => Some(row),
        Ok(None) => {
            tracing::warn!(vendor_template_id, "{context}: vendor template not found");
            None
        }
        Err(e) => {
            tracing::error!(
                vendor_template_id,
                "{context}: vendor template query failed: {e}"
            );
            None
        }
    }
}

/// Load purchase lines from vendor buy list with cost calculations.
pub async fn load_vendor_purchase_lines(
    pool: &Arc<PgPool>,
    vendor_template_id: i32,
    items: &[(i32, i32)],
) -> Option<Vec<VendorPurchaseLine>> {
    let template =
        load_vendor_template_lists(pool, vendor_template_id, "PurchaseVendorItems").await?;
    let Some(buy_item_list) = template.buy_item_list else {
        tracing::debug!(vendor_template_id, "PurchaseVendorItems: vendor has no buy list");
        return None;
    };

    let requested_indices: Vec<i32> = items.iter().map(|(index, _)| *index).collect();
    let rows = match sqlx::query_as::<_, VendorPurchaseItemRow>(
        "SELECT store_index, item_id AS list_item_id, design_id, quantity, naquadah \
         FROM ( \
             SELECT (ROW_NUMBER() OVER (ORDER BY item_id) - 1)::INT AS store_index, \
                    item_id, design_id, GREATEST(quantity, 1)::INT AS quantity, naquadah \
             FROM resources.item_list_items WHERE item_list_id = $1 \
         ) ordered_items WHERE store_index = ANY($2) ORDER BY store_index",
    )
    .bind(buy_item_list)
    .bind(&requested_indices)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(
                vendor_template_id,
                buy_item_list,
                "PurchaseVendorItems: buy list query failed: {e}"
            );
            return None;
        }
    };

    let list_item_ids: Vec<i32> = rows.iter().map(|row| row.list_item_id).collect();
    let price_rows = if list_item_ids.is_empty() {
        Vec::new()
    } else {
        match sqlx::query_as::<_, ItemListPriceRow>(
            "SELECT item_id, design_id, quantity FROM resources.item_list_prices WHERE item_id = ANY($1)",
        )
        .bind(&list_item_ids)
        .fetch_all(pool.as_ref())
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(
                    vendor_template_id,
                    buy_item_list,
                    "PurchaseVendorItems: item cost query failed: {e}"
                );
                return None;
            }
        }
    };

    let mut item_costs_by_list_item: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();
    for price in price_rows {
        item_costs_by_list_item
            .entry(price.item_id)
            .or_default()
            .push((price.design_id, price.quantity));
    }

    let rows_by_index: HashMap<i32, VendorPurchaseItemRow> =
        rows.into_iter().map(|row| (row.store_index, row)).collect();
    let mut lines = Vec::with_capacity(items.len());

    for (index, requested_quantity) in items {
        let Some(row) = rows_by_index.get(index) else {
            tracing::warn!(
                vendor_template_id,
                buy_item_list,
                index,
                "PurchaseVendorItems: requested index is not in vendor buy list"
            );
            return None;
        };

        let grant_quantity = match row.quantity.checked_mul(*requested_quantity) {
            Some(quantity) if quantity > 0 => quantity,
            _ => {
                tracing::warn!(vendor_template_id, index, "PurchaseVendorItems: grant quantity overflow");
                return None;
            }
        };

        let cash_cost = match row.naquadah.checked_mul(*requested_quantity) {
            Some(cost) if cost >= 0 => cost,
            _ => {
                tracing::warn!(vendor_template_id, index, "PurchaseVendorItems: cash cost overflow");
                return None;
            }
        };

        let mut item_costs = Vec::new();
        if let Some(costs) = item_costs_by_list_item.get(&row.list_item_id) {
            for (design_id, quantity) in costs {
                let Some(total_quantity) = quantity.checked_mul(*requested_quantity) else {
                    tracing::warn!(
                        vendor_template_id,
                        index,
                        design_id,
                        "PurchaseVendorItems: item cost overflow"
                    );
                    return None;
                };
                if total_quantity > 0 {
                    item_costs.push((*design_id, total_quantity));
                }
            }
        }

        lines.push(VendorPurchaseLine {
            design_id: row.design_id,
            grant_quantity,
            cash_cost,
            item_costs,
        });
    }

    Some(lines)
}

/// Consume item quantities from player inventory in a transaction.
pub async fn consume_design_quantity(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    design_id: i32,
    quantity: i32,
) -> Result<bool, sqlx::Error> {
    if quantity <= 0 {
        return Ok(true);
    }

    let stacks = sqlx::query_as::<_, InventoryStackRow>(
        "SELECT item_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 AND type_id = $2 AND container_id = ANY($3) \
         ORDER BY container_id, slot_id FOR UPDATE",
    )
    .bind(player_id)
    .bind(design_id)
    .bind(VENDOR_COST_BAGS.as_slice())
    .fetch_all(&mut **tx)
    .await?;

    let available: i32 = stacks.iter().map(|row| row.stack_size).sum();
    if available < quantity {
        return Ok(false);
    }

    let mut remaining = quantity;
    for stack in stacks {
        if remaining <= 0 {
            break;
        }

        if remaining >= stack.stack_size {
            sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1 AND item_id = $2")
                .bind(player_id)
                .bind(stack.item_id)
                .execute(&mut **tx)
                .await?;
            remaining -= stack.stack_size;
        } else {
            sqlx::query(
                "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
                 WHERE character_id = $2 AND item_id = $3",
            )
            .bind(remaining)
            .bind(player_id)
            .bind(stack.item_id)
            .execute(&mut **tx)
            .await?;
            remaining = 0;
        }
    }

    Ok(true)
}

/// Normalize and deduplicate item quantity pairs.
pub fn normalize_item_quantities(items: Vec<(i32, i32)>, allow_zero_item_id: bool) -> Vec<(i32, i32)> {
    let mut normalized: Vec<(i32, i32)> = Vec::new();
    for (item_id, quantity) in items {
        if quantity <= 0
            || (allow_zero_item_id && item_id < 0)
            || (!allow_zero_item_id && item_id <= 0)
        {
            continue;
        }

        if let Some((_, existing_quantity)) = normalized
            .iter_mut()
            .find(|(existing_id, _)| *existing_id == item_id)
        {
            *existing_quantity = existing_quantity.saturating_add(quantity);
        } else {
            normalized.push((item_id, quantity));
        }
    }
    normalized
}