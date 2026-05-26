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
#[tracing::instrument(
    name = "loot.drop",
    level = "info",
    skip_all,
    fields(target_eid, loot_table_id = tracing::field::Empty),
)]
pub(super) fn generate_loot_on_death(target_eid: u32, space_mgr: &mut SpaceManager) {
    // Read loot_table_id before mutable borrow
    let loot_table_id = space_mgr
        .get_entity(target_eid)
        .and_then(|e| e.loot_table_id);

    let loot_table_id = match loot_table_id {
        Some(id) => id,
        None => return, // No loot table — NPC drops nothing
    };
    tracing::Span::current().record("loot_table_id", loot_table_id);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use crate::cell::spawner::LootTableEntry;
    use cimmeria_entity::cell_entity::NpcInteractionType;

    /// Build a minimal `SpaceManager` with one entity at id=1 in a space
    /// large enough that AoI math doesn't intrude. The returned id is 1
    /// so callers can address the entity directly without juggling a return.
    fn make_mgr_with_entity() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        mgr
    }

    #[test]
    fn kill_xp_scales_linearly_with_mob_level() {
        assert_eq!(kill_xp(0), 0);
        assert_eq!(kill_xp(1), 10);
        assert_eq!(kill_xp(50), 500);
    }

    #[test]
    fn kill_xp_does_not_overflow_at_u32_max() {
        // 10 * u32::MAX fits in u64 — pin the widening so a careless
        // refactor to u32 multiplication wouldn't slip through.
        let xp = kill_xp(u32::MAX);
        assert_eq!(xp, 10u64 * u32::MAX as u64);
    }

    #[test]
    fn no_loot_table_id_is_a_no_op() {
        let mut mgr = make_mgr_with_entity();
        // loot_table_id defaults to None; nothing else needs to be set.
        generate_loot_on_death(1, &mut mgr);
        let e = mgr.get_entity(1).unwrap();
        assert!(e.loot.is_empty());
        assert_eq!(e.interaction_type_flags, 0);
        assert!(e.interaction_type.is_none());
    }

    #[test]
    fn missing_loot_table_entries_is_a_no_op() {
        let mut mgr = make_mgr_with_entity();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.loot_table_id = Some(999);
        }
        // mgr.loot_tables has no entry for 999 — should warn and return.
        generate_loot_on_death(1, &mut mgr);
        let e = mgr.get_entity(1).unwrap();
        assert!(e.loot.is_empty());
        assert_eq!(e.interaction_type_flags, 0);
    }

    #[test]
    fn probability_one_drops_loot_and_sets_int_normal_loot() {
        let mut mgr = make_mgr_with_entity();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.loot_table_id = Some(42);
            // Pre-existing interaction bit must survive — Python parity
            // (`SGWMob.onDead`) OR-merges INT_NormalLoot rather than overwriting.
            e.interaction_type_flags = 1 << 5;
        }
        mgr.loot_tables.insert(
            42,
            vec![LootTableEntry {
                design_id: Some(7),
                min_quantity: 3,
                max_quantity: 3,
                probability: 1.0,
            }],
        );

        generate_loot_on_death(1, &mut mgr);

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(e.loot.len(), 1, "probability=1 entry must always drop");
        assert_eq!(e.loot[0].design_id, Some(7));
        assert_eq!(e.loot[0].quantity, 3);
        // INT_NormalLoot (1<<62) is OR-merged on top of the pre-existing bit.
        assert_eq!(e.interaction_type_flags, INT_NORMAL_LOOT | (1 << 5));
        assert!(matches!(e.interaction_type, Some(NpcInteractionType::Loot)));
    }

    #[test]
    fn probability_zero_drops_no_loot_and_leaves_flags_untouched() {
        let mut mgr = make_mgr_with_entity();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.loot_table_id = Some(42);
        }
        mgr.loot_tables.insert(
            42,
            vec![LootTableEntry {
                design_id: Some(7),
                min_quantity: 1,
                max_quantity: 1,
                probability: 0.0,
            }],
        );

        generate_loot_on_death(1, &mut mgr);

        let e = mgr.get_entity(1).unwrap();
        assert!(e.loot.is_empty());
        assert_eq!(
            e.interaction_type_flags, 0,
            "no drops -> INT_NormalLoot must NOT be set"
        );
        assert!(e.interaction_type.is_none());
    }

    /// Regression guard for the malformed-row branch: when a DB row has
    /// `min_quantity > max_quantity`, the original `(max - min + 1) as u32`
    /// math wraps and produces wildly out-of-range quantities. The handler
    /// falls back to `min_quantity` and logs a warning. If a refactor ever
    /// drops the `min > max` check, this test fails because the recorded
    /// quantity diverges from `min`.
    #[test]
    fn min_greater_than_max_falls_back_to_min_quantity() {
        let mut mgr = make_mgr_with_entity();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.loot_table_id = Some(42);
        }
        mgr.loot_tables.insert(
            42,
            vec![LootTableEntry {
                design_id: Some(99),
                min_quantity: 5,
                max_quantity: 2, // intentionally inverted
                probability: 1.0,
            }],
        );

        generate_loot_on_death(1, &mut mgr);

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(e.loot.len(), 1);
        assert_eq!(
            e.loot[0].quantity, 5,
            "min > max must fall back to min, not the wrapping subtraction result"
        );
    }

    #[test]
    fn each_dropped_item_gets_a_unique_increasing_index() {
        let mut mgr = make_mgr_with_entity();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.loot_table_id = Some(42);
            e.next_loot_index = 100;
        }
        mgr.loot_tables.insert(
            42,
            vec![
                LootTableEntry {
                    design_id: Some(1),
                    min_quantity: 1,
                    max_quantity: 1,
                    probability: 1.0,
                },
                LootTableEntry {
                    design_id: Some(2),
                    min_quantity: 1,
                    max_quantity: 1,
                    probability: 1.0,
                },
                LootTableEntry {
                    design_id: Some(3),
                    min_quantity: 1,
                    max_quantity: 1,
                    probability: 1.0,
                },
            ],
        );

        generate_loot_on_death(1, &mut mgr);

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(e.loot.len(), 3);
        assert_eq!(e.loot[0].index, 100);
        assert_eq!(e.loot[1].index, 101);
        assert_eq!(e.loot[2].index, 102);
        assert_eq!(e.next_loot_index, 103);
    }
}
