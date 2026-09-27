//! The pure half of a crafting item use: what an item does
//! ([`ItemEffects`]) and whether using it now changes anything ([`decide`]).

use cimmeria_cell_catalog::crafting::racial_paradigm_name;
use cimmeria_entity::crafting::CraftingState;

use crate::base::crafting::feedback::CraftReject;

/// The highest racial paradigm level. A Guide says "to a maximum of 10"
/// (client text 28224-28234), and the level travels as an `INT8` on 138.
pub const MAX_RACIAL_PARADIGM_LEVEL: i8 = 10;

/// What using an item does, from its `resources.crafting_item_effects`
/// rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemEffects {
    /// A Blueprint item: teaches every listed blueprint (sorted, at least
    /// one).
    TeachBlueprints(Vec<i32>),
    /// A Racial Paradigm Guide: raises this paradigm by one.
    RaiseParadigm(i32),
}

impl ItemEffects {
    /// Build the effects from an item's rows, as `(blueprint_id,
    /// racial_paradigm_id)` pairs. `None` when the rows do not describe one
    /// item kind: no rows, a guide with more than one paradigm, or blueprint
    /// and paradigm rows on one item. The seed guard keeps the seed free of
    /// all three.
    pub fn from_rows(rows: &[(Option<i32>, Option<i32>)]) -> Option<Self> {
        let mut blueprints: Vec<i32> = rows.iter().filter_map(|&(b, _)| b).collect();
        let paradigms: Vec<i32> = rows.iter().filter_map(|&(_, p)| p).collect();
        match (blueprints.is_empty(), paradigms.as_slice()) {
            (false, []) => {
                blueprints.sort_unstable();
                blueprints.dedup();
                Some(ItemEffects::TeachBlueprints(blueprints))
            }
            (true, &[paradigm_id]) => Some(ItemEffects::RaiseParadigm(paradigm_id)),
            _ => None,
        }
    }
}

/// One blueprint a Blueprint item names, and whether it was known before
/// and after the use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintChange {
    pub blueprint_id: i32,
    pub known_before: bool,
    pub known_after: bool,
}

/// The change a use makes to the crafting state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// A Blueprint item taught at least one blueprint.
    Learned {
        /// Every blueprint the item names, in id order.
        blueprints: Vec<BlueprintChange>,
        /// The number of known blueprints before and after.
        known_before: usize,
        known_after: usize,
    },
    /// A Guide raised its paradigm by one.
    Raised {
        paradigm_id: i32,
        level_before: i8,
        level_after: i8,
    },
}

impl Applied {
    /// The `blueprints` field of `blueprint_learned`:
    /// `blueprint_id:known_before→known_after` per blueprint the item names.
    pub fn blueprint_field(blueprints: &[BlueprintChange]) -> String {
        blueprints
            .iter()
            .map(|b| format!("{}:{}→{}", b.blueprint_id, b.known_before, b.known_after))
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Decide a use of an item with `effects` against the locked `state`, and
/// apply it to `state` when it changes something. A Blueprint item whose
/// blueprints are all known, or a Guide whose paradigm is at
/// [`MAX_RACIAL_PARADIGM_LEVEL`], is refused and leaves `state` alone. A
/// Blueprint item that names a known and an unknown blueprint teaches the
/// unknown one.
pub fn decide(
    state: &mut CraftingState,
    effects: &ItemEffects,
    type_id: i32,
) -> Result<Applied, CraftReject> {
    match effects {
        ItemEffects::TeachBlueprints(ids) => {
            let known_before = state.blueprint_ids.len();
            let changes: Vec<BlueprintChange> = ids
                .iter()
                .map(|&blueprint_id| BlueprintChange {
                    blueprint_id,
                    known_before: state.blueprint_ids.contains(&blueprint_id),
                    known_after: true,
                })
                .collect();
            if changes.iter().all(|c| c.known_before) {
                return Err(CraftReject::BlueprintAlreadyKnown {
                    type_id,
                    blueprint_ids: ids.clone(),
                });
            }
            for change in changes.iter().filter(|c| !c.known_before) {
                state.blueprint_ids.push(change.blueprint_id);
            }
            state.blueprint_ids.sort_unstable();
            Ok(Applied::Learned {
                blueprints: changes,
                known_before,
                known_after: state.blueprint_ids.len(),
            })
        }
        &ItemEffects::RaiseParadigm(paradigm_id) => {
            // A loaded state carries every paradigm; a missing one reads as
            // its floor, 0, like the discipline gate does.
            let level_before = state
                .racial_paradigm_levels
                .get(&paradigm_id)
                .copied()
                .unwrap_or(0);
            if level_before >= MAX_RACIAL_PARADIGM_LEVEL {
                return Err(CraftReject::ParadigmAtMax {
                    type_id,
                    paradigm_id,
                    paradigm: racial_paradigm_name(paradigm_id).unwrap_or("that"),
                    level: level_before,
                });
            }
            let level_after = level_before + 1;
            state
                .racial_paradigm_levels
                .insert(paradigm_id, level_after);
            Ok(Applied::Raised {
                paradigm_id,
                level_before,
                level_after,
            })
        }
    }
}
