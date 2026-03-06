//! Inventory system for player entities.
//!
//! Each player has a set of bags (containers) with slots, and items stored
//! in those slots. This module provides the data model and wire serialization
//! for the `SGWInventoryManager` interface.
//!
//! Reference: `python/cell/Inventory.py`, `python/cell/Bag.py`, `python/cell/Item.py`

use std::collections::HashMap;

// ── Bag IDs (from python/Atrea/enums.py:660-679) ───────────────────────────

pub const INV_MAIN: i32 = 1;
pub const INV_MISSION: i32 = 2;
pub const INV_BANDOLIER: i32 = 3;
pub const INV_HEAD: i32 = 4;
pub const INV_FACE: i32 = 5;
pub const INV_NECK: i32 = 6;
pub const INV_CHEST: i32 = 7;
pub const INV_HANDS: i32 = 8;
pub const INV_WAIST: i32 = 9;
pub const INV_BACK: i32 = 10;
pub const INV_LEGS: i32 = 11;
pub const INV_FEET: i32 = 12;
pub const INV_ARTIFACT1: i32 = 13;
pub const INV_ARTIFACT2: i32 = 14;
pub const INV_CRAFTING: i32 = 15;
pub const INV_BUYBACK: i32 = 16;
pub const INV_BANK: i32 = 17;
pub const INV_AUCTION: i32 = 18;
pub const INV_TEAM_BANK: i32 = 19;
pub const INV_COMMAND_BANK: i32 = 20;

/// Default bag sizes from `python/common/Constants.py:142-160`.
pub const BAG_SIZES: &[(i32, i32)] = &[
    (INV_MAIN,       40),
    (INV_MISSION,   100),
    (INV_BANDOLIER,   4),
    (INV_HEAD,        1),
    (INV_FACE,        1),
    (INV_NECK,        1),
    (INV_CHEST,       1),
    (INV_HANDS,       1),
    (INV_WAIST,       1),
    (INV_BACK,        1),
    (INV_LEGS,        1),
    (INV_FEET,        1),
    (INV_ARTIFACT1,   1),
    (INV_ARTIFACT2,   1),
    (INV_CRAFTING,  100),
    (INV_BUYBACK,    12),
    (INV_BANK,      100),
    (INV_AUCTION,   100),
    (INV_TEAM_BANK, 100),
];

// ── Item ────────────────────────────────────────────────────────────────────

/// An inventory item matching the `InvItem` FIXED_DICT from `alias.xml`.
///
/// Wire format (per item in `onUpdateItem`):
/// `id:i32, dbid:i32, stackSize:i32, slotID:i32, containerID:i32,
///  isBound:u8, durability:i32, ammoTypes:ARRAY<i32>, curAmmoType:i32, charges:i32`
#[derive(Debug, Clone)]
pub struct InvItem {
    /// Runtime item instance ID (temporary, assigned by server).
    pub id: i32,
    /// Database item type ID (references item_types table).
    pub dbid: i32,
    /// Stack size / quantity.
    pub stack_size: i32,
    /// Slot index within the container bag.
    pub slot_id: i32,
    /// Bag ID this item is in.
    pub container_id: i32,
    /// Whether the item is bound to the player.
    pub is_bound: bool,
    /// Item durability (current).
    pub durability: i32,
    /// Available ammo types for weapons.
    pub ammo_types: Vec<i32>,
    /// Currently selected ammo type.
    pub cur_ammo_type: i32,
    /// Remaining charges (consumables, abilities).
    pub charges: i32,
}

impl InvItem {
    /// Serialize this item to the `InvItem` FIXED_DICT wire format.
    pub fn serialize(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.id.to_le_bytes());
        buf.extend_from_slice(&self.dbid.to_le_bytes());
        buf.extend_from_slice(&self.stack_size.to_le_bytes());
        buf.extend_from_slice(&self.slot_id.to_le_bytes());
        buf.extend_from_slice(&self.container_id.to_le_bytes());
        buf.push(if self.is_bound { 1 } else { 0 });
        buf.extend_from_slice(&self.durability.to_le_bytes());
        // ammoTypes: ARRAY<INT32>
        buf.extend_from_slice(&(self.ammo_types.len() as u32).to_le_bytes());
        for &ammo in &self.ammo_types {
            buf.extend_from_slice(&ammo.to_le_bytes());
        }
        buf.extend_from_slice(&self.cur_ammo_type.to_le_bytes());
        buf.extend_from_slice(&self.charges.to_le_bytes());
    }
}

// ── Bag ─────────────────────────────────────────────────────────────────────

/// A bag (container) in the inventory.
#[derive(Debug, Clone)]
pub struct Bag {
    pub bag_id: i32,
    pub slots: i32,
}

// ── Inventory ───────────────────────────────────────────────────────────────

/// Player inventory containing bags and items.
pub struct Inventory {
    /// Cash (naquadah).
    pub naquadah: i32,
    /// Bags indexed by bag ID.
    pub bags: HashMap<i32, Bag>,
    /// All items indexed by runtime item ID.
    pub items: HashMap<i32, InvItem>,
}

impl Inventory {
    /// Create a new empty inventory with default bags.
    pub fn new(naquadah: i32) -> Self {
        let mut bags = HashMap::with_capacity(BAG_SIZES.len());
        for &(bag_id, slots) in BAG_SIZES {
            bags.insert(bag_id, Bag { bag_id, slots });
        }
        Self {
            naquadah,
            bags,
            items: HashMap::new(),
        }
    }

    /// Serialize bag info for `onBagInfo(ARRAY<BagInfo>)`.
    ///
    /// Wire: `count:u32`, per bag: `bagId:i32, numberOfSlots:i32`.
    pub fn serialize_bag_info(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 + self.bags.len() * 8);
        buf.extend_from_slice(&(self.bags.len() as u32).to_le_bytes());
        for bag in self.bags.values() {
            buf.extend_from_slice(&bag.bag_id.to_le_bytes());
            buf.extend_from_slice(&bag.slots.to_le_bytes());
        }
        buf
    }

    /// Serialize all items for `onUpdateItem(ARRAY<InvItem>)`.
    ///
    /// Wire: `count:u32`, per item: InvItem FIXED_DICT fields.
    pub fn serialize_items(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 + self.items.len() * 48);
        buf.extend_from_slice(&(self.items.len() as u32).to_le_bytes());
        for item in self.items.values() {
            item.serialize(&mut buf);
        }
        buf
    }

    /// Serialize cash for `onCashChanged(INT32)`.
    pub fn serialize_cash(&self) -> Vec<u8> {
        self.naquadah.to_le_bytes().to_vec()
    }

    /// Add an item to the inventory.
    pub fn add_item(&mut self, item: InvItem) {
        self.items.insert(item.id, item);
    }

    /// Get a bag by ID.
    pub fn get_bag(&self, bag_id: i32) -> Option<&Bag> {
        self.bags.get(&bag_id)
    }

    /// Returns the total number of items across all bags.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

impl std::fmt::Debug for Inventory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inventory")
            .field("naquadah", &self.naquadah)
            .field("bags", &self.bags.len())
            .field("items", &self.items.len())
            .finish()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_inventory_has_default_bags() {
        let inv = Inventory::new(0);
        assert_eq!(inv.bags.len(), BAG_SIZES.len());
        assert_eq!(inv.get_bag(INV_MAIN).unwrap().slots, 40);
        assert_eq!(inv.get_bag(INV_BANDOLIER).unwrap().slots, 4);
        assert_eq!(inv.get_bag(INV_HEAD).unwrap().slots, 1);
        assert_eq!(inv.get_bag(INV_CRAFTING).unwrap().slots, 100);
    }

    #[test]
    fn empty_inventory_serialize_bag_info() {
        let inv = Inventory::new(100);
        let data = inv.serialize_bag_info();
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(count as usize, BAG_SIZES.len());
        // Each bag entry is 8 bytes (bagId:i32 + slots:i32)
        assert_eq!(data.len(), 4 + (count as usize) * 8);
    }

    #[test]
    fn empty_inventory_serialize_items() {
        let inv = Inventory::new(0);
        let data = inv.serialize_items();
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(count, 0);
        assert_eq!(data.len(), 4);
    }

    #[test]
    fn serialize_cash() {
        let inv = Inventory::new(1234);
        let data = inv.serialize_cash();
        assert_eq!(i32::from_le_bytes([data[0], data[1], data[2], data[3]]), 1234);
    }

    #[test]
    fn add_and_serialize_item() {
        let mut inv = Inventory::new(0);
        inv.add_item(InvItem {
            id: 1,
            dbid: 2699,
            stack_size: 1,
            slot_id: 0,
            container_id: INV_BANDOLIER,
            is_bound: false,
            durability: 100,
            ammo_types: vec![],
            cur_ammo_type: 0,
            charges: 0,
        });
        assert_eq!(inv.item_count(), 1);

        let data = inv.serialize_items();
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(count, 1);

        // Verify first item fields
        let id = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        assert_eq!(id, 1);
        let dbid = i32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        assert_eq!(dbid, 2699);
    }

    #[test]
    fn inv_item_serialize_with_ammo() {
        let item = InvItem {
            id: 5,
            dbid: 100,
            stack_size: 1,
            slot_id: 0,
            container_id: INV_BANDOLIER,
            is_bound: true,
            durability: 50,
            ammo_types: vec![10, 20, 30],
            cur_ammo_type: 20,
            charges: 5,
        };
        let mut buf = Vec::new();
        item.serialize(&mut buf);
        // id(4) + dbid(4) + stack(4) + slot(4) + container(4) + bound(1)
        // + durability(4) + ammo_count(4) + 3*ammo(12) + curAmmo(4) + charges(4) = 49
        assert_eq!(buf.len(), 49);

        // Check is_bound = 1
        assert_eq!(buf[20], 1);
        // Check ammo count = 3
        let ammo_count = u32::from_le_bytes([buf[25], buf[26], buf[27], buf[28]]);
        assert_eq!(ammo_count, 3);
    }

    #[test]
    fn bag_ids_match_python() {
        assert_eq!(INV_MAIN, 1);
        assert_eq!(INV_BANDOLIER, 3);
        assert_eq!(INV_CRAFTING, 15);
        assert_eq!(INV_COMMAND_BANK, 20);
    }
}
