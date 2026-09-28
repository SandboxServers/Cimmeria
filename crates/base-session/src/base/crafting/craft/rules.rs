//! The craft rules that need no database: the quantity bound, the
//! blueprint and discipline checks, the named-instance checks, and the
//! choice of component set.

use std::collections::HashMap;

use cimmeria_cell_catalog::crafting::{Blueprint, ComponentSet, CraftingCatalog};
use cimmeria_entity::crafting::CraftingState;

use super::{CRAFT_EXPERTISE_GAIN, MAX_CRAFT_QUANTITY};
use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::transaction::{
    CraftTransaction, RequiredKnowledge, CRAFTING_INPUT_BAGS,
};

/// One inventory instance the request named, as the database has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedInstance {
    pub item_id: i32,
    pub type_id: i32,
    pub container_id: i32,
}

/// A validated craft: what the induction will consume and grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftPlan {
    pub blueprint_id: i32,
    pub component_set_id: i32,
    /// How many times the blueprint runs, as the client asked.
    pub quantity: i32,
    pub product_id: i32,
    /// `blueprint.quantity × quantity`.
    pub product_quantity: i32,
    pub transaction: CraftTransaction,
}

/// The client's quantity slider starts at 1; 0 and negatives are a forged
/// request, and the upper bound keeps one request from queuing an
/// unbounded amount of work.
pub fn check_quantity(blueprint_id: i32, quantity: i32) -> Result<(), CraftReject> {
    if (1..=MAX_CRAFT_QUANTITY).contains(&quantity) {
        Ok(())
    } else {
        Err(CraftReject::BadQuantity {
            blueprint_id,
            quantity,
            max: MAX_CRAFT_QUANTITY,
        })
    }
}

/// The blueprint, if the player knows it, it is not an alloy, and its
/// discipline is known too.
pub fn check_blueprint<'c>(
    state: &CraftingState,
    catalog: &'c CraftingCatalog,
    blueprint_id: i32,
) -> Result<&'c Blueprint, CraftReject> {
    let blueprint = catalog
        .blueprint(blueprint_id)
        .filter(|_| state.blueprint_ids.contains(&blueprint_id))
        .ok_or(CraftReject::UnknownBlueprint { blueprint_id })?;
    if blueprint.is_alloy {
        return Err(CraftReject::IsAlloy { blueprint_id });
    }
    match blueprint.discipline_id {
        Some(id) if state.knows_discipline(id) => Ok(blueprint),
        // No seed blueprint lacks a discipline; one that did could never
        // be crafted, and 0 names no discipline.
        discipline_id => Err(CraftReject::DisciplineUnknown {
            blueprint_id,
            discipline_id: discipline_id.unwrap_or(0),
        }),
    }
}

/// The requested instances, once each and in request order, if every one
/// is the player's and sits in the main or crafting bag. `found` is what
/// the database returned for the player's own instances among `requested`.
pub fn check_named(
    requested: &[i32],
    found: &[NamedInstance],
) -> Result<Vec<NamedInstance>, CraftReject> {
    let mut named: Vec<NamedInstance> = Vec::with_capacity(requested.len());
    for &item_id in requested {
        if named.iter().any(|n| n.item_id == item_id) {
            continue;
        }
        let instance = found
            .iter()
            .find(|f| f.item_id == item_id)
            .ok_or(CraftReject::ComponentMissing { item_id })?;
        if !CRAFTING_INPUT_BAGS.contains(&instance.container_id) {
            return Err(CraftReject::ComponentNotInCraftingBags {
                item_id,
                container_id: instance.container_id,
            });
        }
        named.push(*instance);
    }
    Ok(named)
}

/// The distinct designs of `named`, ascending.
pub fn submitted_types(named: &[NamedInstance]) -> Vec<i32> {
    let mut types: Vec<i32> = named.iter().map(|n| n.type_id).collect();
    types.sort_unstable();
    types.dedup();
    types
}

/// `set`'s designs, ascending.
fn set_types(set: &ComponentSet) -> Vec<i32> {
    let mut types: Vec<i32> = set.components.iter().map(|c| c.item_id).collect();
    types.sort_unstable();
    types.dedup();
    types
}

/// Choose the component set and build the plan.
///
/// The craft page sends one instance per component of the set the player
/// picked, and the set id itself is not on the wire. So the set is the one
/// whose designs are exactly the submitted designs. "Every component
/// covered" is not enough: many seed sets are subsets of a sibling set
/// (412 set 1, 14 Steel Cores, is covered by set 2's Steel Core plus
/// Titanium Core), and the first covered set would charge the wrong
/// recipe. Where
/// two sets have the same designs (159 sets 1 and 2), the first the bags
/// can pay for wins. `available` is the total of each design in the main
/// and crafting bags.
pub fn plan_craft(
    blueprint: &Blueprint,
    product_id: i32,
    named: &[NamedInstance],
    available: &HashMap<i32, i64>,
    quantity: i32,
) -> Result<CraftPlan, CraftReject> {
    let blueprint_id = blueprint.blueprint_id;
    let bad_quantity = CraftReject::BadQuantity {
        blueprint_id,
        quantity,
        max: MAX_CRAFT_QUANTITY,
    };
    let types = submitted_types(named);
    let candidates: Vec<&ComponentSet> = blueprint
        .component_sets
        .iter()
        .filter(|set| !set.components.is_empty() && set_types(set) == types)
        .collect();
    if candidates.is_empty() {
        return Err(CraftReject::NoComponentSet {
            blueprint_id,
            type_ids: types,
        });
    }

    let mut shortfall = None;
    let mut chosen = None;
    for set in candidates {
        let mut needs = Vec::with_capacity(set.components.len());
        for c in &set.components {
            let needed = c
                .quantity
                .checked_mul(quantity)
                .ok_or(bad_quantity.clone())?;
            needs.push((c.item_id, needed));
        }
        let short = needs.iter().find_map(|&(design_id, needed)| {
            let have = available.get(&design_id).copied().unwrap_or(0);
            (have < i64::from(needed)).then_some(CraftReject::InsufficientComponents {
                blueprint_id,
                design_id,
                needed,
                available: have,
            })
        });
        match short {
            None => {
                chosen = Some((set.set_id, needs));
                break;
            }
            Some(why) => {
                shortfall.get_or_insert(why);
            }
        }
    }
    let Some((component_set_id, consume)) = chosen else {
        return Err(shortfall.unwrap_or(CraftReject::NoComponentSet {
            blueprint_id,
            type_ids: types,
        }));
    };

    let product_quantity = blueprint
        .quantity
        .checked_mul(quantity)
        .ok_or(bad_quantity)?;
    let expertise = blueprint
        .discipline_id
        .map(|d| vec![(d, CRAFT_EXPERTISE_GAIN)])
        .unwrap_or_default();
    Ok(CraftPlan {
        blueprint_id,
        component_set_id,
        quantity,
        product_id,
        product_quantity,
        // No named instances in the transaction: they only chose the set.
        // The page names the last stack it found, which an earlier queued
        // craft may drain; the completion consumes by design from the two
        // bags, so holding it to that one instance would refuse a craft the
        // player can pay for.
        transaction: CraftTransaction {
            named_items: Vec::new(),
            consume_named: Vec::new(),
            learn_blueprints: Vec::new(),
            consume,
            grant: vec![(product_id, product_quantity)],
            expertise,
            required_knowledge: blueprint
                .discipline_id
                .map(|discipline_id| RequiredKnowledge {
                    blueprint_id,
                    discipline_id,
                }),
            research: None,
        },
    })
}
