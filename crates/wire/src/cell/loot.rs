//! `onLootDisplay` (client method 114) — the loot window's contents.
//!
//! One serializer for every caller: the corpse window (`cell::interactions::
//! loot`) and the content `open_loot` action on a live container both push
//! the same bytes, so the two cannot drift.
//!
//! Wire format per `LootItemQuantity` in alias.xml:
//! `itemID:i32, quantity:i16, index:i32, typeID:i32`.
//! Outer: `entityId:i32, ARRAY<LootItemQuantity>, initial:i8`.
//!
//! `initial = 1` opens the window; `0` refreshes it after a `lootItem`. An
//! empty list closes it (`Loot.lua` hides `LootWin` when `getLootCount()`
//! is 0). Reference: `python/cell/interactions/Lootable.py:sendLootList()`.

use cimmeria_entity::cell_entity::LootItem;

/// `LOOT_Item` in the `typeID` field.
pub const LOOT_TYPE_ITEM: i32 = 1;
/// `LOOT_Cash` in the `typeID` field (naquadah, `itemID = 0`).
pub const LOOT_TYPE_CASH: i32 = 2;

/// Serialize the `onLootDisplay` arguments for `items` on `entity_id`.
pub fn serialize_on_loot_display(entity_id: i32, items: &[LootItem], initial: u8) -> Vec<u8> {
    // Per item: 4 (itemID) + 2 (quantity i16) + 4 (index) + 4 (typeID).
    let mut args = Vec::with_capacity(4 + 4 + items.len() * 14 + 1);
    args.extend_from_slice(&entity_id.to_le_bytes());
    args.extend_from_slice(&(items.len() as u32).to_le_bytes());
    for li in items {
        let (item_id, type_id) = match li.design_id {
            Some(id) => (id, LOOT_TYPE_ITEM),
            None => (0, LOOT_TYPE_CASH),
        };
        args.extend_from_slice(&item_id.to_le_bytes());
        args.extend_from_slice(&(li.quantity as i16).to_le_bytes());
        args.extend_from_slice(&li.index.to_le_bytes());
        args.extend_from_slice(&type_id.to_le_bytes());
    }
    args.push(initial);
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-exact: one item, one naquadah stack, `initial = 1`.
    #[test]
    fn serializes_items_and_cash_byte_exact() {
        let items = [
            LootItem {
                design_id: Some(3127),
                quantity: 1,
                index: 1,
            },
            LootItem {
                design_id: None,
                quantity: 40,
                index: 2,
            },
        ];
        let got = serialize_on_loot_display(0x0102_0304, &items, 1);
        let want: Vec<u8> = [
            &[0x04, 0x03, 0x02, 0x01][..], // entityId
            &[2, 0, 0, 0],                 // count
            &3127i32.to_le_bytes(),        // itemID
            &[1, 0],                       // quantity i16
            &[1, 0, 0, 0],                 // index
            &[1, 0, 0, 0],                 // LOOT_Item
            &[0, 0, 0, 0],                 // itemID 0 = naquadah
            &[40, 0],                      // quantity
            &[2, 0, 0, 0],                 // index
            &[2, 0, 0, 0],                 // LOOT_Cash
            &[1],                          // initial
        ]
        .concat();
        assert_eq!(got, want);
    }

    /// The empty list that closes the window: entity id, count 0, initial.
    #[test]
    fn an_empty_list_is_nine_bytes() {
        assert_eq!(
            serialize_on_loot_display(7, &[], 0),
            vec![7, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }
}
