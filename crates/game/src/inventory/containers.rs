use serde::{Deserialize, Serialize};

use super::items::ItemInstance;

/// An inventory container with a fixed number of slots.
///
/// Corresponds to the EInventoryContainerId enum and the sgw_inventory table
/// partitioning in the database schema. Each player has several containers
/// (backpack, equipment, bank, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryContainer {
    pub container_id: i32,
    pub capacity: usize,
    slots: Vec<Option<ItemInstance>>,
}

impl InventoryContainer {
    /// Create an empty container with the given capacity.
    pub fn new(container_id: i32, capacity: usize) -> Self {
        Self {
            container_id,
            capacity,
            slots: vec![None; capacity],
        }
    }

    /// Returns the number of occupied slots.
    pub fn used_slots(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Returns the number of empty slots.
    pub fn free_slots(&self) -> usize {
        self.capacity - self.used_slots()
    }

    /// Try to add an item to the first available slot. Returns `Err` with the
    /// item back if the container is full.
    pub fn add_item(&mut self, item: ItemInstance) -> Result<usize, ItemInstance> {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(item);
                return Ok(i);
            }
        }
        Err(item)
    }

    /// Remove and return the item at the given slot index.
    pub fn remove_item(&mut self, slot_index: usize) -> Option<ItemInstance> {
        if slot_index < self.capacity {
            self.slots[slot_index].take()
        } else {
            None
        }
    }

    /// Get a reference to the item at the given slot index.
    pub fn get_item(&self, slot_index: usize) -> Option<&ItemInstance> {
        self.slots.get(slot_index).and_then(|s| s.as_ref())
    }

    /// Iterate over all occupied slots with their indices.
    pub fn items(&self) -> impl Iterator<Item = (usize, &ItemInstance)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| s.as_ref().map(|item| (i, item)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::items::{ItemQuality, ItemType};

    fn test_item(id: i64) -> ItemInstance {
        ItemInstance::new(id, 1, format!("Item_{}", id), ItemType::Misc, ItemQuality::Common, 1)
    }

    #[test]
    fn empty_container() {
        let c = InventoryContainer::new(0, 10);
        assert_eq!(c.free_slots(), 10);
        assert_eq!(c.used_slots(), 0);
    }

    #[test]
    fn add_and_retrieve_item() {
        let mut c = InventoryContainer::new(0, 5);
        let slot = c.add_item(test_item(1)).unwrap();
        assert_eq!(slot, 0);
        assert_eq!(c.used_slots(), 1);
        assert!(c.get_item(0).is_some());
    }

    #[test]
    fn full_container_rejects_item() {
        let mut c = InventoryContainer::new(0, 1);
        c.add_item(test_item(1)).unwrap();
        let result = c.add_item(test_item(2));
        assert!(result.is_err());
    }

    #[test]
    fn remove_item_frees_slot() {
        let mut c = InventoryContainer::new(0, 5);
        c.add_item(test_item(1)).unwrap();
        let removed = c.remove_item(0);
        assert!(removed.is_some());
        assert_eq!(c.free_slots(), 5);
    }
}
