//! The pure half of reverse engineering: which items qualify, and what a
//! completion recovers.

use cimmeria_cell_catalog::crafting::{Blueprint, ComponentSet, CraftingCatalog};
use cimmeria_entity::crafting::CraftingState;

use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::item_lookup::HeldInstance;
use crate::base::crafting::rng::CraftRng;

/// The blueprints that make `type_id` and have at least one component
/// set, in id order: the ones a reverse engineering can pick from.
pub fn candidate_blueprints(catalog: &CraftingCatalog, type_id: i32) -> Vec<&Blueprint> {
    let mut found: Vec<&Blueprint> = catalog
        .blueprints
        .values()
        .filter(|b| b.product_id == Some(type_id) && !b.component_sets.is_empty())
        .collect();
    found.sort_unstable_by_key(|b| b.blueprint_id);
    found
}

/// Check the item: flagged reverse-engineerable, and made by at least one
/// blueprint with a recipe. No discipline needs to be known.
pub fn check_request(catalog: &CraftingCatalog, item: &HeldInstance) -> Result<(), CraftReject> {
    let (item_id, type_id) = (item.item_id, item.type_id);
    let engineerable = catalog
        .item(type_id)
        .is_some_and(|a| a.flags.is_reverse_engineerable());
    if !engineerable {
        return Err(CraftReject::NotReverseEngineerable { item_id, type_id });
    }
    if candidate_blueprints(catalog, type_id).is_empty() {
        return Err(CraftReject::NoBlueprintForItem { item_id, type_id });
    }
    Ok(())
}

/// One component of the picked set, and what was recovered of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentRoll {
    pub design_id: i32,
    /// The recipe's quantity.
    pub quantity: i32,
    /// The sample in `[0, 1)`.
    pub roll: f64,
    pub recovered: i32,
}

/// What one reverse engineering recovers.
#[derive(Debug, Clone, PartialEq)]
pub struct Recovery {
    pub blueprint_id: i32,
    pub component_set_id: i32,
    /// `min(1, max(expertise, 1) / tech competency)`.
    pub bias: f64,
    pub components: Vec<ComponentRoll>,
}

impl Recovery {
    /// `(design_id, quantity)` to grant, zero quantities left out.
    pub fn grants(&self) -> Vec<(i32, i32)> {
        self.components
            .iter()
            .filter(|c| c.recovered > 0)
            .map(|c| (c.design_id, c.recovered))
            .collect()
    }

    /// The `rolls` field: `design_id:roll:recovered/quantity` per component.
    pub fn rolls_field(&self) -> String {
        self.components
            .iter()
            .map(|c| {
                format!(
                    "{}:{:.4}:{}/{}",
                    c.design_id, c.roll, c.recovered, c.quantity
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// The recovery bias for `expertise` in the blueprint's discipline against
/// the product's tech competency `tech_comp`. It rises with expertise and
/// reaches 1 at the tech competency; an unknown discipline counts as
/// expertise 1, so it is never a division by zero. A product with no tech
/// competency recovers at full bias.
pub fn bias(expertise: i32, tech_comp: i32) -> f64 {
    if tech_comp <= 0 {
        return 1.0;
    }
    (f64::from(expertise.max(1)) / f64::from(tech_comp)).min(1.0)
}

/// Pick a blueprint from `candidates`, then one of its component sets,
/// each uniformly (one sample each), and roll every component of the set:
/// `floor(sample × bias × quantity)`. When every component comes to 0, the
/// component with the highest sample (the first on a tie) recovers one
/// unit, so a reverse engineering always returns something.
///
/// The expertise is the player's in the picked blueprint's discipline (0
/// when unknown) and `tech_comp` is the product's tech competency.
pub fn recover(
    candidates: &[&Blueprint],
    state: &CraftingState,
    tech_comp: i32,
    rng: &mut dyn CraftRng,
) -> Option<Recovery> {
    let blueprint = *pick(candidates, rng)?;
    let set: &ComponentSet = pick(&blueprint.component_sets, rng)?;
    let expertise = blueprint
        .discipline_id
        .and_then(|d| state.get_expertise(d))
        .unwrap_or(0);
    let bias = bias(expertise, tech_comp);
    let mut components: Vec<ComponentRoll> = set
        .components
        .iter()
        .map(|c| {
            let roll = rng.unit();
            ComponentRoll {
                design_id: c.item_id,
                quantity: c.quantity,
                roll,
                recovered: (roll * bias * f64::from(c.quantity)).floor() as i32,
            }
        })
        .collect();
    if components.iter().all(|c| c.recovered <= 0) {
        let best = components
            .iter()
            .enumerate()
            .fold(None::<(usize, f64)>, |best, (i, c)| match best {
                Some((_, r)) if r >= c.roll => best,
                _ => Some((i, c.roll)),
            });
        if let Some((i, _)) = best {
            components[i].recovered = 1;
        }
    }
    Some(Recovery {
        blueprint_id: blueprint.blueprint_id,
        component_set_id: set.set_id,
        bias,
        components,
    })
}

/// A uniform pick from `items` with one sample; `None` when empty.
fn pick<'a, T>(items: &'a [T], rng: &mut dyn CraftRng) -> Option<&'a T> {
    if items.is_empty() {
        return None;
    }
    let i = ((rng.unit() * items.len() as f64) as usize).min(items.len() - 1);
    items.get(i)
}
