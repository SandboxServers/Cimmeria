use std::collections::HashSet;

use sqlx::{PgPool, Postgres, Transaction};

use super::super::super::super::resources::{bag_max_slots, bag_min_slot};

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
///
/// `min_slot` is the lowest assignable slot (see [`bag_min_slot`]) — for the
/// bandolier this is 1 because slot 0 is reserved for the empty/fist default
/// weapon (issue #119). Filtering occupants by `min_slot` matters: if a row
/// somehow lives in a sub-min slot (manual fixup, legacy data), it's
/// excluded from the occupancy set so the function can still return real
/// free slots above the threshold.
pub fn free_inventory_slots(
    min_slot: i32,
    max_slots: i32,
    occupied_slots: &[i32],
    needed: usize,
) -> Option<Vec<i32>> {
    if needed == 0 {
        return Some(Vec::new());
    }
    if max_slots <= 0 || min_slot >= max_slots {
        return None;
    }
    let min_slot = min_slot.max(0);

    let occupied: HashSet<i32> = occupied_slots
        .iter()
        .copied()
        .filter(|slot| *slot >= min_slot && *slot < max_slots)
        .collect();
    let slots: Vec<i32> = (min_slot..max_slots)
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
        bag_min_slot(container_id),
        bag_max_slots(container_id),
        &occupied_slots,
        needed,
    ))
}

#[cfg(test)]
mod serializer_tests {
    use super::*;

    #[test]
    fn store_item_cost_array_empty_emits_count_zero() {
        let mut args = Vec::new();
        serialize_store_item_cost_array(&mut args, &[]);
        assert_eq!(args, [0, 0, 0, 0], "empty cost array must still write u32(0) count");
    }

    #[test]
    fn store_item_cost_array_writes_cost_then_item_id_per_row() {
        let mut args = Vec::new();
        serialize_store_item_cost_array(&mut args, &[
            StoreItemCost { cost: 250, item_id: 1001 },
            StoreItemCost { cost: -1,  item_id: 2002 },
        ]);

        // count(u32) | (cost:i32 | item_id:i32) * 2
        assert_eq!(args.len(), 4 + 2 * 8);
        assert_eq!(u32::from_le_bytes([args[0], args[1], args[2], args[3]]), 2);
        assert_eq!(i32::from_le_bytes([args[4], args[5], args[6], args[7]]), 250);
        assert_eq!(i32::from_le_bytes([args[8], args[9], args[10], args[11]]), 1001);
        assert_eq!(i32::from_le_bytes([args[12], args[13], args[14], args[15]]), -1);
        assert_eq!(i32::from_le_bytes([args[16], args[17], args[18], args[19]]), 2002);
    }

    #[test]
    fn store_update_empty_emits_count_zero() {
        let args = serialize_store_update(&[]);
        assert_eq!(args, [0, 0, 0, 0]);
    }

    #[test]
    fn store_update_byte_exact_layout() {
        // Row layout: item_id, sell_price, repair_price, recharge_price (all i32 LE).
        // Flips a per-row sign so a regression that swaps two adjacent fields fails
        // loudly instead of silently producing the same wire bytes.
        let args = serialize_store_update(&[StoreItemCostUpdate {
            item_id: 7777,
            sell_price: 100,
            repair_price: 200,
            recharge_price: 300,
        }]);

        assert_eq!(args.len(), 4 + 16);
        assert_eq!(u32::from_le_bytes([args[0], args[1], args[2], args[3]]), 1);
        assert_eq!(i32::from_le_bytes([args[4], args[5], args[6], args[7]]), 7777);
        assert_eq!(i32::from_le_bytes([args[8], args[9], args[10], args[11]]), 100);
        assert_eq!(i32::from_le_bytes([args[12], args[13], args[14], args[15]]), 200);
        assert_eq!(i32::from_le_bytes([args[16], args[17], args[18], args[19]]), 300);
    }

    #[test]
    fn empty_store_open_writes_vendor_id_marker_and_five_empty_arrays() {
        let args = serialize_empty_store_open(424242);

        // vendor_id(i32) | marker(i32 = 1) | buy_count(u32 = 0)
        // | sell_count(u32 = 0) | buyback_count(u32 = 0)
        // | repair_count(u32 = 0) | recharge_count(u32 = 0)
        assert_eq!(args.len(), 4 + 4 + 4 + 4 * 4);
        assert_eq!(i32::from_le_bytes([args[0], args[1], args[2], args[3]]), 424242);
        // The magic "1" sandwiched between vendor_id and the buy-list count is
        // load-bearing — the client de-serializer reads it before the array
        // header. A regression that drops or moves it desyncs every later
        // field.
        assert_eq!(i32::from_le_bytes([args[4], args[5], args[6], args[7]]), 1);
        for off in (8..args.len()).step_by(4) {
            assert_eq!(
                u32::from_le_bytes([args[off], args[off + 1], args[off + 2], args[off + 3]]),
                0,
                "expected u32(0) array count at offset {off}",
            );
        }
    }

    #[test]
    fn store_open_full_layout_round_trips_all_sections() {
        let buy_items = vec![StoreItem {
            index: 5,
            item_id: 9001,
            cost_list: vec![
                StoreBuyCost { cost_type: 0, type_id: 0,    quantity: 50 },
                StoreBuyCost { cost_type: 1, type_id: 1234, quantity: 1  },
            ],
            usable: 1,
            quantity: 7,
        }];
        let sell     = vec![StoreItemCost { cost: 11, item_id: 100 }];
        let buyback  = vec![StoreItemCost { cost: 22, item_id: 200 }];
        let repair   = vec![StoreItemCost { cost: 33, item_id: 300 }];
        let recharge = vec![StoreItemCost { cost: 44, item_id: 400 }];

        let args = serialize_store_open(42, &buy_items, &sell, &buyback, &repair, &recharge);

        let mut off = 0;
        let read_i32 = |args: &[u8], off: &mut usize| {
            let v = i32::from_le_bytes([args[*off], args[*off + 1], args[*off + 2], args[*off + 3]]);
            *off += 4;
            v
        };
        let read_u32 = |args: &[u8], off: &mut usize| {
            let v = u32::from_le_bytes([args[*off], args[*off + 1], args[*off + 2], args[*off + 3]]);
            *off += 4;
            v
        };

        assert_eq!(read_i32(&args, &mut off), 42, "vendor_entity_id");
        assert_eq!(read_i32(&args, &mut off), 1, "marker");

        // Buy items
        assert_eq!(read_u32(&args, &mut off), 1, "buy items count");
        assert_eq!(read_i32(&args, &mut off), 5, "item.index");
        assert_eq!(read_i32(&args, &mut off), 9001, "item.item_id");
        assert_eq!(read_u32(&args, &mut off), 2, "cost_list count");
        // Cost row 0
        assert_eq!(read_i32(&args, &mut off), 0,  "cost_list[0].cost_type");
        assert_eq!(read_i32(&args, &mut off), 0,  "cost_list[0].type_id");
        assert_eq!(read_i32(&args, &mut off), 50, "cost_list[0].quantity");
        // Cost row 1
        assert_eq!(read_i32(&args, &mut off), 1,    "cost_list[1].cost_type");
        assert_eq!(read_i32(&args, &mut off), 1234, "cost_list[1].type_id");
        assert_eq!(read_i32(&args, &mut off), 1,    "cost_list[1].quantity");
        assert_eq!(args[off], 1, "item.usable"); off += 1;
        assert_eq!(read_i32(&args, &mut off), 7, "item.quantity");

        // Sell / buyback / repair / recharge — emitted in this exact order.
        for (label, expected_cost, expected_item_id) in [
            ("sell", 11, 100),
            ("buyback", 22, 200),
            ("repair", 33, 300),
            ("recharge", 44, 400),
        ] {
            assert_eq!(read_u32(&args, &mut off), 1, "{label} count");
            assert_eq!(read_i32(&args, &mut off), expected_cost, "{label} cost");
            assert_eq!(read_i32(&args, &mut off), expected_item_id, "{label} item_id");
        }

        assert_eq!(off, args.len(), "no trailing bytes after parsed sections");
    }
}

#[cfg(test)]
mod free_inventory_slots_tests {
    use super::*;

    #[test]
    fn allocates_from_zero_for_main_bag() {
        // Main bag (container 1): min_slot = 0, slot 0 fillable.
        let slots = free_inventory_slots(0, 4, &[], 2).unwrap();
        assert_eq!(slots, vec![0, 1]);
    }

    #[test]
    fn skips_slot_0_for_bandolier() {
        // Bug #119 regression: bandolier (container 3) must start at slot 1
        // so the fist-weapon default at slot 0 isn't overwritten.
        let slots = free_inventory_slots(1, 4, &[], 3).unwrap();
        assert_eq!(slots, vec![1, 2, 3]);
    }

    #[test]
    fn bandolier_returns_none_when_only_slot_0_free() {
        // 3 weapons already occupy slots 1,2,3; slot 0 is reserved.
        // Asking for 1 more must return None, not [0].
        let slots = free_inventory_slots(1, 4, &[1, 2, 3], 1);
        assert!(slots.is_none(), "must not allocate the reserved slot 0");
    }

    #[test]
    fn ignores_sub_min_occupants_when_finding_free_slots() {
        // A stray row at slot 0 (legacy fixup) shouldn't make slot 1 look
        // occupied — the filter on `occupied_slots` is min/max bounded so
        // we still hand back the real free slot above the threshold.
        let slots = free_inventory_slots(1, 4, &[0, 2], 2).unwrap();
        assert_eq!(slots, vec![1, 3]);
    }

    #[test]
    fn returns_empty_when_zero_needed() {
        assert_eq!(free_inventory_slots(1, 4, &[], 0).unwrap(), Vec::<i32>::new());
    }

    #[test]
    fn returns_none_when_min_exceeds_max() {
        // Defensive: nonsensical min/max combo should not wrap or panic.
        assert!(free_inventory_slots(5, 4, &[], 1).is_none());
    }
}