//! Loot table system for NPC drops.
//!
//! When an NPC dies, the loot system rolls against its loot table to determine
//! which items to drop. Loot tables are loaded from the database at startup
//! and cached for the lifetime of the CellService.
//!
//! Data flow: entity_templates.loot_table_id → loot_tables → loot entries
//! Each loot entry has: design_id (item type), probability, min/max quantity.
//!
//! Reference: `python/cell/SGWMob.py:onDeath()` → `generateLoot()`

use std::collections::HashMap;

use tokio::sync::mpsc;

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Loot data structures ────────────────────────────────────────────────────

/// A single loot entry from the database.
#[derive(Debug, Clone)]
pub struct LootEntry {
    pub design_id: i32,
    pub probability: f32,
    pub min_quantity: i32,
    pub max_quantity: i32,
}

/// A loot table containing all possible drops.
#[derive(Debug, Clone)]
pub struct LootTable {
    pub loot_table_id: i32,
    pub description: String,
    pub entries: Vec<LootEntry>,
}

/// Cache of all loot tables loaded from the database.
pub struct LootCache {
    tables: HashMap<i32, LootTable>,
}

impl LootCache {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    pub fn get(&self, loot_table_id: i32) -> Option<&LootTable> {
        self.tables.get(&loot_table_id)
    }

    pub fn table_count(&self) -> usize {
        self.tables.len()
    }
}

impl Default for LootCache {
    fn default() -> Self {
        Self::new()
    }
}

// ── DB loading ──────────────────────────────────────────────────────────────

/// Load all loot tables from the database.
pub async fn load_loot_cache(pool: &sqlx::PgPool) -> Result<LootCache, sqlx::Error> {
    use sqlx::Row;

    let mut cache = LootCache::new();

    // Load loot table metadata
    let table_rows = sqlx::query(
        "SELECT loot_table_id, description FROM resources.loot_tables ORDER BY loot_table_id",
    )
    .fetch_all(pool)
    .await?;

    for r in &table_rows {
        let id: i32 = r.get("loot_table_id");
        cache.tables.insert(
            id,
            LootTable {
                loot_table_id: id,
                description: r.get("description"),
                entries: Vec::new(),
            },
        );
    }

    // Load loot entries and attach to their tables
    let entry_rows = sqlx::query(
        "SELECT loot_table_id, design_id, probability, min_quantity, max_quantity \
         FROM resources.loot ORDER BY loot_table_id, loot_id",
    )
    .fetch_all(pool)
    .await?;

    for r in &entry_rows {
        let table_id: i32 = r.get("loot_table_id");
        if let Some(table) = cache.tables.get_mut(&table_id) {
            table.entries.push(LootEntry {
                design_id: r.get("design_id"),
                probability: r.get("probability"),
                min_quantity: r.get("min_quantity"),
                max_quantity: r.get("max_quantity"),
            });
        }
    }

    tracing::info!(
        tables = cache.tables.len(),
        entries = entry_rows.len(),
        "Loot cache loaded from database"
    );

    Ok(cache)
}

// ── Loot generation ─────────────────────────────────────────────────────────

/// A rolled loot drop result.
#[derive(Debug)]
pub struct LootDrop {
    pub item_id: i32,
    pub quantity: i32,
}

/// Roll loot for a killed NPC.
///
/// Uses a simple PRNG seeded from entity_id + a counter to generate
/// reproducible-ish rolls. Each entry in the loot table is rolled
/// independently against its probability.
pub fn roll_loot(loot_table: &LootTable, entity_id: u32, roll_seed: u32) -> Vec<LootDrop> {
    let mut drops = Vec::new();

    for (i, entry) in loot_table.entries.iter().enumerate() {
        // Generate a random value in [0.0, 1.0)
        let rand = pseudo_random(entity_id, roll_seed, i as u32);

        if rand < entry.probability as f64 {
            // Roll quantity between min and max
            let range = (entry.max_quantity - entry.min_quantity + 1).max(1);
            let qty_rand = pseudo_random(entity_id, roll_seed.wrapping_add(1000), i as u32);
            let quantity = entry.min_quantity + (qty_rand * range as f64) as i32;

            drops.push(LootDrop {
                item_id: entry.design_id,
                quantity: quantity.min(entry.max_quantity),
            });
        }
    }

    drops
}

/// Generate loot for a dead NPC and send it to the killer.
///
/// Looks up the NPC's loot_table_id from entity properties, rolls drops,
/// and sends `onLootDisplay` + `GrantItem` messages.
pub async fn generate_and_send_loot(
    killer_entity_id: u32,
    dead_npc_id: u32,
    loot_cache: &LootCache,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    // Get the NPC's loot_table_id from properties
    let loot_table_id = match space_mgr.get_entity(dead_npc_id) {
        Some(entity) => {
            entity.properties.get("loot_table_id")
                .and_then(|v| match v {
                    cimmeria_entity::base_entity::PropertyValue::Int32(id) => Some(*id),
                    _ => None,
                })
        }
        None => None,
    };

    let loot_table_id = match loot_table_id {
        Some(id) if id > 0 => id,
        _ => return, // No loot table assigned
    };

    let table = match loot_cache.get(loot_table_id) {
        Some(t) => t,
        None => {
            tracing::debug!(dead_npc_id, loot_table_id, "No loot table found in cache");
            return;
        }
    };

    let drops = roll_loot(table, dead_npc_id, killer_entity_id);

    if drops.is_empty() {
        return;
    }

    tracing::info!(
        killer = killer_entity_id,
        npc = dead_npc_id,
        drops = drops.len(),
        "Generated loot drops"
    );

    // Build onLootDisplay args
    // Wire: entityId:i32, items:ARRAY<{itemId:i32, quantity:i16, index:i32, typeId:i32}>, initial:i8
    let mut args = Vec::with_capacity(9 + drops.len() * 14);
    args.extend_from_slice(&(dead_npc_id as i32).to_le_bytes()); // EntityID
    args.extend_from_slice(&(drops.len() as u32).to_le_bytes()); // count

    for (idx, drop) in drops.iter().enumerate() {
        args.extend_from_slice(&drop.item_id.to_le_bytes());          // itemId
        args.extend_from_slice(&(drop.quantity as i16).to_le_bytes()); // quantity
        args.extend_from_slice(&(idx as i32).to_le_bytes());           // index
        args.extend_from_slice(&drop.item_id.to_le_bytes());          // typeId (same as itemId for now)
    }

    args.push(1); // initial = 1

    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: killer_entity_id,
        method_index: 114, // onLootDisplay
        args,
    }).await;

    // Also grant items directly (auto-loot behavior)
    let player_id = space_mgr.get_entity(killer_entity_id)
        .and_then(|e| e.player_id)
        .unwrap_or(0);

    for drop in &drops {
        let _ = tx.send(CellToBaseMsg::GrantItem {
            entity_id: killer_entity_id,
            player_id,
            item_id: drop.item_id,
            container_id: 1, // general inventory
            count: drop.quantity,
        }).await;
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Simple hash-based PRNG for loot rolls. Same as abilities.rs.
fn pseudo_random(a: u32, b: u32, c: u32) -> f64 {
    let mut h = a.wrapping_mul(2654435761);
    h ^= b.wrapping_mul(2246822519);
    h ^= c.wrapping_mul(3266489917);
    h = h.wrapping_mul(668265263);
    h ^= h >> 15;
    (h as f64) / (u32::MAX as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_loot_table() -> LootTable {
        LootTable {
            loot_table_id: 1,
            description: "Test table".to_string(),
            entries: vec![
                LootEntry {
                    design_id: 100,
                    probability: 1.0, // guaranteed drop
                    min_quantity: 1,
                    max_quantity: 3,
                },
                LootEntry {
                    design_id: 200,
                    probability: 0.5, // 50% chance
                    min_quantity: 1,
                    max_quantity: 1,
                },
                LootEntry {
                    design_id: 300,
                    probability: 0.0, // never drops
                    min_quantity: 1,
                    max_quantity: 1,
                },
            ],
        }
    }

    #[test]
    fn guaranteed_drop_always_drops() {
        let table = make_loot_table();
        // Run multiple rolls — item 100 (prob=1.0) should always drop
        for seed in 0..100 {
            let drops = roll_loot(&table, 1, seed);
            assert!(
                drops.iter().any(|d| d.item_id == 100),
                "Item 100 should always drop (seed={seed})"
            );
        }
    }

    #[test]
    fn zero_probability_never_drops() {
        let table = make_loot_table();
        for seed in 0..100 {
            let drops = roll_loot(&table, 1, seed);
            assert!(
                !drops.iter().any(|d| d.item_id == 300),
                "Item 300 should never drop (seed={seed})"
            );
        }
    }

    #[test]
    fn quantity_within_range() {
        let table = make_loot_table();
        for seed in 0..100 {
            let drops = roll_loot(&table, 1, seed);
            for drop in &drops {
                if drop.item_id == 100 {
                    assert!(drop.quantity >= 1 && drop.quantity <= 3,
                        "Quantity {} out of range 1-3", drop.quantity);
                }
            }
        }
    }

    #[test]
    fn deterministic_with_same_seed() {
        let table = make_loot_table();
        let drops1 = roll_loot(&table, 42, 7);
        let drops2 = roll_loot(&table, 42, 7);
        assert_eq!(drops1.len(), drops2.len());
        for (a, b) in drops1.iter().zip(drops2.iter()) {
            assert_eq!(a.item_id, b.item_id);
            assert_eq!(a.quantity, b.quantity);
        }
    }

    #[test]
    fn different_seeds_vary() {
        let table = make_loot_table();
        // Item 200 with 50% prob should drop for some seeds and not others
        let mut dropped = 0;
        let mut not_dropped = 0;
        for seed in 0..200 {
            let drops = roll_loot(&table, 1, seed);
            if drops.iter().any(|d| d.item_id == 200) {
                dropped += 1;
            } else {
                not_dropped += 1;
            }
        }
        // With 200 trials and 50% prob, both should be > 0
        assert!(dropped > 0, "Item 200 should drop sometimes");
        assert!(not_dropped > 0, "Item 200 should NOT drop sometimes");
    }

    #[test]
    fn empty_table_gives_no_drops() {
        let table = LootTable {
            loot_table_id: 99,
            description: "Empty".to_string(),
            entries: vec![],
        };
        let drops = roll_loot(&table, 1, 0);
        assert!(drops.is_empty());
    }

    #[test]
    fn loot_cache_lookup() {
        let mut cache = LootCache::new();
        cache.tables.insert(1, make_loot_table());
        assert!(cache.get(1).is_some());
        assert!(cache.get(2).is_none());
        assert_eq!(cache.table_count(), 1);
    }
}
