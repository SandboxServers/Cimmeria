//! On-death loot generation and the kill-XP curve.
//!
//! Called from `use_ability` once a target's HEALTH drops to zero. Rolls each
//! entry in the NPC's loot table independently and, if any items drop, sets
//! the entity's interaction-flags so the client renders the loot cursor.
//!
//! Reference: `python/cell/SGWMob.py:onDead()`, `python/cell/interactions/Lootable.py`

use super::super::space_manager::SpaceManager;

/// INT_NormalLoot interaction type flag (1 << 62).
/// From `python/Atrea/enums.py: INT_NormalLoot = 4611686018427387904`.
/// This is the interaction bitflag that tells the client to show the loot cursor.
pub(crate) const INT_NORMAL_LOOT: i64 = 4611686018427387904;

/// Generate loot from the NPC's loot table and store it on the entity.
///
/// Rolls each entry independently against its probability, matching the Python
/// `Lootable.randomizeLoot()` algorithm. If any loot is generated, sets the
/// entity's interaction_type_flags to INT_NormalLoot and adds a Loot interaction.
///
/// Reference: `python/cell/SGWMob.py:onDead()`, `python/cell/interactions/Lootable.py`
pub(super) fn generate_loot_on_death(target_eid: u32, space_mgr: &mut SpaceManager) {
    // Read loot_table_id before mutable borrow
    let loot_table_id = space_mgr
        .get_entity(target_eid)
        .and_then(|e| e.loot_table_id);

    let loot_table_id = match loot_table_id {
        Some(id) => id,
        None => return, // No loot table — NPC drops nothing
    };

    // Look up loot entries (clone to avoid borrow conflict with space_mgr)
    let entries = match space_mgr.loot_tables.get(&loot_table_id) {
        Some(entries) => entries.clone(),
        None => {
            tracing::debug!(target_eid, loot_table_id, "No loot table entries found");
            return;
        }
    };

    // Roll each entry
    let target = match space_mgr.get_entity_mut(target_eid) {
        Some(e) => e,
        None => return,
    };

    for entry in &entries {
        let roll: f32 = rand::random();
        if roll <= entry.probability {
            // Guard against malformed DB rows where min > max -- the
            // subtraction would wrap as u32 and produce wildly out-of-range
            // quantities. Log and fall back to min in that case.
            let quantity = if entry.min_quantity == entry.max_quantity {
                entry.min_quantity
            } else if entry.min_quantity > entry.max_quantity {
                tracing::warn!(
                    target_eid,
                    loot_table_id,
                    design_id = ?entry.design_id,
                    min = entry.min_quantity,
                    max = entry.max_quantity,
                    "loot entry has min_quantity > max_quantity; using min as fallback"
                );
                entry.min_quantity
            } else {
                let range = (entry.max_quantity - entry.min_quantity + 1) as u32;
                entry.min_quantity + (rand::random::<u32>() % range) as i32
            };

            if quantity > 0 {
                let index = target.next_loot_index;
                target.next_loot_index += 1;

                target.loot.push(cimmeria_entity::cell_entity::LootItem {
                    design_id: entry.design_id,
                    quantity,
                    index,
                });

                let name = entry
                    .design_id
                    .map(|id| format!("item_{id}"))
                    .unwrap_or_else(|| "naquadah".to_string());
                tracing::debug!(
                    target_eid, %name, quantity, index,
                    probability = entry.probability,
                    "Loot generated"
                );
            }
        }
    }

    if !target.loot.is_empty() {
        // Set INT_NormalLoot so the client shows the loot cursor. OR-preserve to match
        // the Python reference (`setInteractionType(self.interactionType | INT_NormalLoot)`)
        // — keeps any bits a content chain set pre-death.
        target.interaction_type_flags |= INT_NORMAL_LOOT;
        target.interaction_type = Some(cimmeria_entity::cell_entity::NpcInteractionType::Loot);
        tracing::debug!(
            target_eid,
            items = target.loot.len(),
            "NPC has loot — set INT_NormalLoot interaction"
        );
    }
}

/// Calculate XP reward for killing a mob of the given level.
/// Formula: 10 * mob_level.
pub(super) fn kill_xp(mob_level: u32) -> u64 {
    10 * mob_level as u64
}
