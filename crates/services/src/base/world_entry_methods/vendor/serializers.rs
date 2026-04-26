use std::collections::HashSet;

use sqlx::{PgPool, Postgres, Transaction};

use super::super::super::resources::bag_max_slots;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreBuyCost {
    pub cost_type: i32,
    pub type_id: i32,
    pub quantity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreItem {
    pub index: i32,
    pub item_id: i32,
    pub cost_list: Vec<StoreBuyCost>,
    pub usable: u8,
    pub quantity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreItemCost {
    pub cost: i32,
    pub item_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreItemCostUpdate {
    pub item_id: i32,
    pub sell_price: i32,
    pub repair_price: i32,
    pub recharge_price: i32,
}

/// Serialize store open payload with item lists and pricing arrays.
pub fn serialize_store_open(
    vendor_entity_id: i32,
    buy_items: &[StoreItem],
    sell_prices: &[StoreItemCost],
    buyback_prices: &[StoreItemCost],
    repair_prices: &[StoreItemCost],
    recharge_prices: &[StoreItemCost],
) -> Vec<u8> {
    let mut args = Vec::with_capacity(28 + buy_items.len() * 25);
    args.extend_from_slice(&vendor_entity_id.to_le_bytes());
    args.extend_from_slice(&1i32.to_le_bytes());

    args.extend_from_slice(&u32::try_from(buy_items.len()).unwrap_or(u32::MAX).to_le_bytes());
    for item in buy_items {
        args.extend_from_slice(&item.index.to_le_bytes());
        args.extend_from_slice(&item.item_id.to_le_bytes());
        args.extend_from_slice(&u32::try_from(item.cost_list.len()).unwrap_or(u32::MAX).to_le_bytes());
        for cost in &item.cost_list {
            args.extend_from_slice(&cost.cost_type.to_le_bytes());
            args.extend_from_slice(&cost.type_id.to_le_bytes());
            args.extend_from_slice(&cost.quantity.to_le_bytes());
        }
        args.push(item.usable);
        args.extend_from_slice(&item.quantity.to_le_bytes());
    }

    serialize_store_item_cost_array(&mut args, sell_prices);
    serialize_store_item_cost_array(&mut args, buyback_prices);
    serialize_store_item_cost_array(&mut args, repair_prices);
    serialize_store_item_cost_array(&mut args, recharge_prices);
    args
}

/// Serialize an empty store (no items) with just vendor ID.
pub fn serialize_empty_store_open(vendor_entity_id: i32) -> Vec<u8> {
    serialize_store_open(vendor_entity_id, &[], &[], &[], &[], &[])
}

/// Serialize a single cost array (count + items).
pub fn serialize_store_item_cost_array(args: &mut Vec<u8>, prices: &[StoreItemCost]) {
    args.extend_from_slice(&u32::try_from(prices.len()).unwrap_or(u32::MAX).to_le_bytes());
    for price in prices {
        args.extend_from_slice(&price.cost.to_le_bytes());
        args.extend_from_slice(&price.item_id.to_le_bytes());
    }
}

/// Serialize store update with changed prices/repairs/recharges.
pub fn serialize_store_update(updates: &[StoreItemCostUpdate]) -> Vec<u8> {
    let mut args = Vec::with_capacity(4 + updates.len() * 16);
    args.extend_from_slice(&u32::try_from(updates.len()).unwrap_or(u32::MAX).to_le_bytes());
    for update in updates {
        args.extend_from_slice(&update.item_id.to_le_bytes());
        args.extend_from_slice(&update.sell_price.to_le_bytes());
        args.extend_from_slice(&update.repair_price.to_le_bytes());
        args.extend_from_slice(&update.recharge_price.to_le_bytes());
    }
    args
}

/// Find free inventory slots in a container, checking occupancy.
pub fn free_inventory_slots(max_slots: i32, occupied_slots: &[i32], needed: usize) -> Option<Vec<i32>> {
    if needed == 0 {
        return Some(Vec::new());
    }
    if max_slots <= 0 {
        return None;
    }

    let occupied: HashSet<i32> = occupied_slots
        .iter()
        .copied()
        .filter(|slot| *slot >= 0 && *slot < max_slots)
        .collect();
    let slots: Vec<i32> = (0..max_slots)
        .filter(|slot| !occupied.contains(slot))
        .take(needed)
        .collect();

    (slots.len() == needed).then_some(slots)
}

/// Atomically reserve and return free slots in a container within a transaction.
///
/// `FOR UPDATE` only locks rows that already exist; it does NOT block another
/// transaction from inserting new rows into a slot the current transaction
/// believes is free. To make slot reservation truly mutually-exclusive across
/// concurrent grants/purchases on the same (player, container), we acquire a
/// per-(player, container) advisory lock for the lifetime of this transaction
/// before reading occupancy.
pub async fn reserve_free_inventory_slots(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    container_id: i32,
    needed: usize,
) -> Result<Option<Vec<i32>>, sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(player_id)
        .bind(container_id)
        .execute(&mut **tx)
        .await?;

    #[derive(sqlx::FromRow)]
    struct InventorySlotRow {
        slot_id: i32,
    }

    let rows = sqlx::query_as::<_, InventorySlotRow>(
        "SELECT slot_id FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2 \
         FOR UPDATE",
    )
    .bind(player_id)
    .bind(container_id)
    .fetch_all(&mut **tx)
    .await?;
    let occupied_slots: Vec<i32> = rows.into_iter().map(|row| row.slot_id).collect();
    Ok(free_inventory_slots(
        bag_max_slots(container_id),
        &occupied_slots,
        needed,
    ))
}