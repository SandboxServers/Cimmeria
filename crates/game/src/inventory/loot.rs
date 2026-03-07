use serde::{Deserialize, Serialize};

use super::items::ItemInstance;

/// A single entry in a loot table with a drop chance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootEntry {
    pub item_template_id: i32,
    pub drop_chance: f32,
    pub min_quantity: u32,
    pub max_quantity: u32,
}

/// A loot table loaded from the database loot/loot_tables resources.
///
/// When a mob dies or a lootable container is opened, the table is rolled
/// to produce a list of item drops.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootTable {
    pub loot_table_id: i32,
    pub entries: Vec<LootEntry>,
}

impl LootTable {
    /// Create a new empty loot table.
    pub fn new(loot_table_id: i32) -> Self {
        Self {
            loot_table_id,
            entries: Vec::new(),
        }
    }

    /// Add an entry to this loot table.
    pub fn add_entry(&mut self, entry: LootEntry) {
        self.entries.push(entry);
    }

    /// Roll the loot table and return the resulting item instances.
    ///
    /// Each entry is rolled independently against its drop chance.
    pub fn roll_loot(&self) -> Vec<LootDrop> {
        let mut drops = Vec::new();

        for entry in &self.entries {
            let roll: f32 = rand::random();
            if roll < entry.drop_chance {
                let quantity = if entry.min_quantity == entry.max_quantity {
                    entry.min_quantity
                } else {
                    // Random quantity between min and max inclusive
                    let range = entry.max_quantity - entry.min_quantity + 1;
                    entry.min_quantity + (rand::random::<u32>() % range)
                };

                drops.push(LootDrop {
                    item_template_id: entry.item_template_id,
                    quantity,
                });
            }
        }

        drops
    }
}

/// A rolled loot drop: the template ID and quantity to instantiate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootDrop {
    pub item_template_id: i32,
    pub quantity: u32,
}

/// Convert a loot drop into an actual item instance.
///
/// Requires the item template data from the database to populate fields.
pub fn instantiate_loot_drop(_drop: &LootDrop, _next_instance_id: i64) -> ItemInstance {
    todo!("instantiate_loot_drop - load item template from database")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_table_drops_nothing() {
        let table = LootTable::new(1);
        let drops = table.roll_loot();
        assert!(drops.is_empty());
    }

    #[test]
    fn guaranteed_drop() {
        let mut table = LootTable::new(1);
        table.add_entry(LootEntry {
            item_template_id: 42,
            drop_chance: 1.0, // always drops
            min_quantity: 1,
            max_quantity: 1,
        });
        let drops = table.roll_loot();
        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].item_template_id, 42);
        assert_eq!(drops[0].quantity, 1);
    }

    #[test]
    fn zero_chance_never_drops() {
        let mut table = LootTable::new(1);
        table.add_entry(LootEntry {
            item_template_id: 99,
            drop_chance: 0.0,
            min_quantity: 1,
            max_quantity: 1,
        });
        // Run many times to be confident
        for _ in 0..100 {
            let drops = table.roll_loot();
            assert!(drops.is_empty());
        }
    }
}
