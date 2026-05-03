use std::collections::HashMap;
use std::sync::Arc;

use sqlx::{PgPool, Postgres, Transaction};

use super::store::VendorTemplateLists;

const INV_MAIN: i32 = 1;
// Containers consulted when consuming item-prerequisites for a purchase:
//   INV_MAIN  = 1  — main bag
//   2          — vault/bank container (can pay from stored items)
//   15         — quick bar / hot bar (also lootable for prereqs)
// Bandolier (3) and equipment slots (4..=14) are deliberately excluded — a
// vendor purchase shouldn't strip an equipped weapon or armor piece to satisfy
// a recipe cost.
const VENDOR_COST_BAGS: [i32; 3] = [INV_MAIN, 2, 15];

// Hard cap on a single purchase line. Real vendor UIs are bounded well below
// this (typical stack sizes are <100); larger values are almost certainly a
// malformed/malicious request. Without this guard a vendor row with
// `quantity = 1` and `naquadah = 0` would let a client mint up to i32::MAX
// items at zero cost — checked_mul on `1 * MAX` doesn't overflow.
const MAX_VENDOR_PURCHASE_QUANTITY: i32 = 9_999;

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
        // Mirror the level used by the sibling "vendor template not found" arm
        // in `load_vendor_template_lists` and by the paid_repair/paid_recharge
        // missing-list branches: a vendor template that's reached via
        // PurchaseVendorItems but has no buy_item_list is either a
        // misconfiguration or an unexpected client request, neither of which
        // should be hidden behind debug-level filtering in production.
        tracing::warn!(vendor_template_id, "PurchaseVendorItems: vendor has no buy list");
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
        if *requested_quantity <= 0 || *requested_quantity > MAX_VENDOR_PURCHASE_QUANTITY {
            tracing::warn!(
                vendor_template_id, buy_item_list, index, requested_quantity,
                cap = MAX_VENDOR_PURCHASE_QUANTITY,
                "PurchaseVendorItems: requested quantity out of range"
            );
            return None;
        }

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
            Some(quantity) => {
                tracing::warn!(
                    vendor_template_id, index,
                    row_quantity = row.quantity, requested_quantity, computed = quantity,
                    "PurchaseVendorItems: rejecting non-positive grant quantity"
                );
                return None;
            }
            None => {
                tracing::warn!(
                    vendor_template_id, index,
                    row_quantity = row.quantity, requested_quantity,
                    "PurchaseVendorItems: grant quantity overflow"
                );
                return None;
            }
        };

        let cash_cost = match row.naquadah.checked_mul(*requested_quantity) {
            Some(cost) if cost >= 0 => cost,
            Some(cost) => {
                tracing::warn!(
                    vendor_template_id, index,
                    naquadah = row.naquadah, requested_quantity, computed = cost,
                    "PurchaseVendorItems: rejecting negative cash cost"
                );
                return None;
            }
            None => {
                tracing::warn!(
                    vendor_template_id, index,
                    naquadah = row.naquadah, requested_quantity,
                    "PurchaseVendorItems: cash cost overflow"
                );
                return None;
            }
        };

        let mut item_costs = Vec::new();
        if let Some(costs) = item_costs_by_list_item.get(&row.list_item_id) {
            for (design_id, quantity) in costs {
                let total_quantity = match quantity.checked_mul(*requested_quantity) {
                    Some(t) if t > 0 => t,
                    Some(t) => {
                        tracing::warn!(
                            vendor_template_id, index, design_id,
                            unit = quantity, requested_quantity, computed = t,
                            "PurchaseVendorItems: rejecting non-positive item cost"
                        );
                        return None;
                    }
                    None => {
                        tracing::warn!(
                            vendor_template_id, index, design_id,
                            unit = quantity, requested_quantity,
                            "PurchaseVendorItems: item cost overflow"
                        );
                        return None;
                    }
                };
                item_costs.push((*design_id, total_quantity));
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

    // Sum in i64 to defend against pathological inventories where summed stacks
    // exceed i32::MAX. Compare back in i64 against the (i32) requested quantity.
    let available: i64 = stacks.iter().map(|row| row.stack_size as i64).sum();
    if available < quantity as i64 {
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

#[cfg(test)]
mod normalize_item_quantities_tests {
    use super::normalize_item_quantities;

    #[test]
    fn drops_non_positive_quantities() {
        let out = normalize_item_quantities(vec![(1, 5), (2, 0), (3, -7)], false);
        assert_eq!(out, vec![(1, 5)]);
    }

    #[test]
    fn drops_non_positive_item_ids_when_zero_disallowed() {
        let out = normalize_item_quantities(vec![(0, 5), (-1, 5), (3, 5)], false);
        assert_eq!(out, vec![(3, 5)], "id 0 and -1 must be dropped when allow_zero_item_id=false");
    }

    #[test]
    fn keeps_zero_item_id_when_allowed_but_still_drops_negative() {
        // The flag allows item_id == 0 (e.g. the "any item" wildcard in some
        // recipe contexts) but a negative id is always invalid.
        let out = normalize_item_quantities(vec![(0, 5), (-1, 5), (3, 5)], true);
        assert_eq!(out, vec![(0, 5), (3, 5)]);
    }

    #[test]
    fn dedupes_by_summing_quantities() {
        let out = normalize_item_quantities(vec![(7, 1), (7, 2), (7, 4)], false);
        assert_eq!(out, vec![(7, 7)]);
    }

    #[test]
    fn preserves_first_occurrence_order() {
        // Note: function does NOT sort — dedupe-by-linear-scan keeps the order
        // of the first occurrence. Locking that in so a future "let's sort
        // these for determinism" refactor would have to be deliberate.
        let out = normalize_item_quantities(vec![(3, 1), (1, 1), (2, 1)], false);
        assert_eq!(out, vec![(3, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn dedupe_uses_saturating_add_on_overflow() {
        // saturating_add prevents a malicious client from wrapping the merged
        // quantity into a negative — without it, two i32::MAX entries would
        // sum to -2 and slip past downstream "quantity > 0" guards.
        let out = normalize_item_quantities(
            vec![(1, i32::MAX), (1, i32::MAX)], false,
        );
        assert_eq!(out, vec![(1, i32::MAX)]);
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(normalize_item_quantities(Vec::new(), false).is_empty());
    }
}