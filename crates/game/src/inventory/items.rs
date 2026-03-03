use serde::{Deserialize, Serialize};

/// Item quality tiers matching EItemQuality from the database schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemQuality {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

/// Item type classification matching EItemType from the database schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemType {
    Weapon,
    Armor,
    Consumable,
    QuestItem,
    Misc,
}

/// A specific instance of an item in the game world or a player's inventory.
///
/// Corresponds to a row in the `sgw_inventory` table plus the item template
/// from the `items` resource table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemInstance {
    pub instance_id: i64,
    pub template_id: i32,
    pub name: String,
    pub item_type: ItemType,
    pub quality: ItemQuality,
    pub stack_count: u32,
    pub max_stack: u32,
    pub is_bound: bool,
}

impl ItemInstance {
    /// Create a new item instance from a template.
    pub fn new(instance_id: i64, template_id: i32, name: String, item_type: ItemType, quality: ItemQuality, max_stack: u32) -> Self {
        Self {
            instance_id,
            template_id,
            name,
            item_type,
            quality,
            stack_count: 1,
            max_stack,
            is_bound: false,
        }
    }

    /// Returns `true` if more units can be added to this stack.
    pub fn is_stackable(&self) -> bool {
        self.max_stack > 1 && self.stack_count < self.max_stack
    }

    /// Try to merge `count` units into this stack. Returns the number of
    /// units that did not fit.
    pub fn add_to_stack(&mut self, count: u32) -> u32 {
        let space = self.max_stack - self.stack_count;
        let added = count.min(space);
        self.stack_count += added;
        count - added
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_stack_not_stackable() {
        let item = ItemInstance::new(1, 100, "Zat'nik'tel".to_string(), ItemType::Weapon, ItemQuality::Rare, 1);
        assert!(!item.is_stackable());
    }

    #[test]
    fn stackable_item_accepts_units() {
        let mut item = ItemInstance::new(1, 200, "Tretonin".to_string(), ItemType::Consumable, ItemQuality::Common, 10);
        let overflow = item.add_to_stack(5);
        assert_eq!(overflow, 0);
        assert_eq!(item.stack_count, 6);
    }

    #[test]
    fn stack_overflow_returns_remainder() {
        let mut item = ItemInstance::new(1, 200, "Tretonin".to_string(), ItemType::Consumable, ItemQuality::Common, 5);
        let overflow = item.add_to_stack(10);
        assert_eq!(overflow, 6);
        assert_eq!(item.stack_count, 5);
    }
}
