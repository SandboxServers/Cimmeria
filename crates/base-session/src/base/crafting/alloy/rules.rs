//! The alloy rules, decided without the database: the verb loads the
//! player's crafting state and the named item rows, and [`check_alloy`]
//! turns them into an [`AlloyPlan`] or a refusal.
//!
//! The client's own check (`AlloyPage.lua:168-199`, the native count check
//! the send path runs before every `alloying`) is the reference:
//!
//! - the current-tier item is the blueprint's one component;
//! - every elementary item is exactly one tier below that component;
//! - the elementary items' **stack quantities** are summed per quality, and
//!   exactly one quality must reach its count: Normal 10, Good 5, Great 2,
//!   Fantastic 1. None reached, or two or more reached at once, is refused.
//!   Poor has no count, so Poor items count toward nothing.
//!
//! The client only warns and sends anyway, so every rule is enforced here.

use std::collections::HashSet;

use cimmeria_cell_catalog::crafting::{CraftingCatalog, ItemQuality};
use cimmeria_entity::crafting::CraftingState;

use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::transaction::{CraftTransaction, NamedItem, CRAFTING_INPUT_BAGS};

/// The elementary count each quality needs, in the order the counts are
/// reported (Normal, Good, Great, Fantastic).
pub const ELEMENTARY_COUNTS: [(ItemQuality, i64); 4] = [
    (ItemQuality::Normal, 10),
    (ItemQuality::Good, 5),
    (ItemQuality::Great, 2),
    (ItemQuality::Fantastic, 1),
];

/// The alloy page's elementary slots (`AlloyPage.lua:467-468`). The client
/// sends at most one id per slot, so a longer list is forged.
pub const MAX_ELEMENTARY_ITEMS: usize = 10;

/// Expertise one alloy adds to the blueprint's discipline.
pub const ALLOY_EXPERTISE: i32 = 1;

/// An item instance the player owns, as the request-time read found it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldItem {
    pub item_id: i32,
    pub type_id: i32,
    pub stack_size: i32,
    pub container_id: i32,
}

/// The arguments of one `alloying` request.
#[derive(Debug, Clone, Copy)]
pub struct AlloyRequest<'a> {
    pub blueprint_id: i32,
    pub current_tier_item_id: i32,
    pub lower_tier_items: &'a [i32],
}

/// Why [`check_alloy`] did not produce a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlloyCheck {
    /// A rule refused the request; the player is told why.
    Refused(CraftReject),
    /// The catalog lacks what an alloy blueprint needs (its discipline, its
    /// one component, its product, or an item's attributes). A data fault,
    /// logged as `lookup_failed`; the player gets the "unavailable" line.
    Catalog { phase: &'static str, id: i32 },
}

impl From<CraftReject> for AlloyCheck {
    fn from(why: CraftReject) -> Self {
        AlloyCheck::Refused(why)
    }
}

/// One elementary instance the plan uses, and how much of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementaryUse {
    pub item_id: i32,
    pub type_id: i32,
    pub quality: ItemQuality,
    pub tier: i32,
    pub quantity: i32,
}

/// A validated alloy: what the completion consumes and grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlloyPlan {
    pub blueprint_id: i32,
    pub discipline_id: i32,
    pub product_id: i32,
    pub product_quantity: i32,
    /// The current-tier item the request named.
    pub current_tier_item_id: i32,
    /// The blueprint's component design and how many it takes.
    pub component_id: i32,
    pub component_quantity: i32,
    /// The quality whose count was met; `None` for a blueprint that needs
    /// no elementary components.
    pub bucket: Option<ItemQuality>,
    /// The elementary instances used, in request order. Instances of the
    /// met quality beyond its count, and instances of other qualities, are
    /// not used.
    pub elementary: Vec<ElementaryUse>,
}

impl AlloyPlan {
    /// The completion transaction. The component is consumed by design
    /// and not named: the client names one stack of it, and a queued alloy
    /// whose named component an earlier alloy used up still completes
    /// while the bags hold another. Each elementary instance is named and
    /// consumed exactly, because the count the client checked is the named
    /// stacks' quantities.
    pub fn transaction(&self) -> CraftTransaction {
        let mut named_items = Vec::with_capacity(self.elementary.len());
        let mut consume_named = Vec::with_capacity(self.elementary.len());
        for e in &self.elementary {
            named_items.push(NamedItem::new(e.item_id, e.type_id));
            consume_named.push((e.item_id, e.quantity));
        }
        CraftTransaction {
            named_items,
            consume_named,
            consume: vec![(self.component_id, self.component_quantity)],
            grant: vec![(self.product_id, self.product_quantity)],
            expertise: vec![(self.discipline_id, ALLOY_EXPERTISE)],
            ..CraftTransaction::default()
        }
    }

    /// The `completed` event's `elementary` field:
    /// `item_id:type_id:quality:tier:quantity_used`, comma-separated.
    pub fn elementary_field(&self) -> String {
        self.elementary
            .iter()
            .map(|e| {
                format!(
                    "{}:{}:{}:{}:{}",
                    e.item_id,
                    e.type_id,
                    bucket_label(e.quality),
                    e.tier,
                    e.quantity
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// The `quality_bucket` label of a quality.
pub fn bucket_label(quality: ItemQuality) -> &'static str {
    match quality {
        ItemQuality::Poor => "poor",
        ItemQuality::Normal => "normal",
        ItemQuality::Good => "good",
        ItemQuality::Great => "great",
        ItemQuality::Fantastic => "fantastic",
    }
}

/// Decide an alloy request.
///
/// `held` is every named instance the player owns, wherever it sits;
/// `component_available` is how many of the blueprint's component the main
/// and crafting bags hold. Checks, in order: the blueprint exists and is an
/// alloy, the player knows it and its discipline, the current-tier item,
/// then each elementary item's ownership, bag and tier, then the counts.
pub fn check_alloy(
    state: &CraftingState,
    catalog: &CraftingCatalog,
    request: &AlloyRequest<'_>,
    held: &[HeldItem],
    component_available: i64,
) -> Result<AlloyPlan, AlloyCheck> {
    let blueprint_id = request.blueprint_id;
    let Some(blueprint) = catalog.blueprint(blueprint_id) else {
        return Err(CraftReject::UnknownBlueprint { blueprint_id }.into());
    };
    if !blueprint.is_alloy {
        return Err(CraftReject::NotAlloy { blueprint_id }.into());
    }
    if !state.blueprint_ids.contains(&blueprint_id) {
        return Err(CraftReject::UnknownBlueprint { blueprint_id }.into());
    }
    let catalog_fault = |phase| AlloyCheck::Catalog {
        phase,
        id: blueprint_id,
    };
    let discipline_id = blueprint
        .discipline_id
        .ok_or_else(|| catalog_fault("alloy_discipline"))?;
    if !state.knows_discipline(discipline_id) {
        return Err(CraftReject::DisciplineUnknown {
            blueprint_id,
            discipline_id,
        }
        .into());
    }
    // An alloy has one component set with one component (the client reads
    // the first of each, `AlloyPage.lua:255-259`).
    let component = blueprint
        .component_sets
        .first()
        .and_then(|set| set.components.first())
        .copied()
        .ok_or_else(|| catalog_fault("alloy_component"))?;
    let product_id = blueprint
        .product_id
        .ok_or_else(|| catalog_fault("alloy_product"))?;
    // The transaction skips a non-positive grant, so such a row would
    // consume the inputs for nothing.
    if blueprint.quantity <= 0 {
        return Err(catalog_fault("alloy_product_quantity"));
    }

    let current = held_in_bags(held, request.current_tier_item_id)?;
    if current.type_id != component.item_id {
        return Err(CraftReject::ComponentMismatch {
            item_id: current.item_id,
            expected_design_id: component.item_id,
            type_id: current.type_id,
        }
        .into());
    }
    if component_available < i64::from(component.quantity) {
        return Err(CraftReject::NotEnoughComponents {
            design_id: component.item_id,
            needed: component.quantity,
            available: component_available,
        }
        .into());
    }

    let mut plan = AlloyPlan {
        blueprint_id,
        discipline_id,
        product_id,
        product_quantity: blueprint.quantity,
        current_tier_item_id: current.item_id,
        component_id: component.item_id,
        component_quantity: component.quantity,
        bucket: None,
        elementary: Vec::new(),
    };
    if !blueprint.requires_elementary_components {
        return Ok(plan);
    }

    let required_tier = catalog
        .item(component.item_id)
        .map(|attrs| attrs.tier - 1)
        .ok_or(AlloyCheck::Catalog {
            phase: "alloy_item",
            id: component.item_id,
        })?;
    // The client cannot slot one instance twice (`AlloyPage.lua:193-198`);
    // a forged repeat is counted once.
    let mut seen = HashSet::with_capacity(request.lower_tier_items.len());
    let mut candidates = Vec::with_capacity(request.lower_tier_items.len());
    for &item_id in request.lower_tier_items {
        if !seen.insert(item_id) {
            continue;
        }
        let item = held_in_bags(held, item_id)?;
        let attrs = catalog.item(item.type_id).ok_or(AlloyCheck::Catalog {
            phase: "alloy_item",
            id: item.type_id,
        })?;
        if attrs.tier != required_tier {
            return Err(CraftReject::WrongTier {
                item_id,
                type_id: item.type_id,
                tier: attrs.tier,
                required_tier,
            }
            .into());
        }
        candidates.push((item, attrs.quality, attrs.tier));
    }

    let mut counts = [0i64; 4];
    for (item, quality, _) in &candidates {
        if let Some(i) = ELEMENTARY_COUNTS.iter().position(|(q, _)| q == quality) {
            counts[i] += i64::from(item.stack_size.max(0));
        }
    }
    let met: Vec<usize> = (0..ELEMENTARY_COUNTS.len())
        .filter(|&i| counts[i] >= ELEMENTARY_COUNTS[i].1)
        .collect();
    let bucket = match met.as_slice() {
        [] => return Err(CraftReject::CountNotMet { counts }.into()),
        [one] => *one,
        _ => return Err(CraftReject::MultipleBuckets { counts }.into()),
    };
    let (quality, needed) = ELEMENTARY_COUNTS[bucket];
    let mut remaining = needed;
    for (item, item_quality, tier) in candidates {
        if remaining == 0 {
            break;
        }
        if item_quality != quality || item.stack_size <= 0 {
            continue;
        }
        let take = remaining.min(i64::from(item.stack_size));
        remaining -= take;
        plan.elementary.push(ElementaryUse {
            item_id: item.item_id,
            type_id: item.type_id,
            quality,
            tier,
            // At most the bucket count (10), so the cast is exact.
            quantity: take as i32,
        });
    }
    plan.bucket = Some(quality);
    Ok(plan)
}

/// The held instance `item_id`, if the player owns it and it sits in the
/// main or crafting bag.
fn held_in_bags(held: &[HeldItem], item_id: i32) -> Result<HeldItem, CraftReject> {
    let Some(item) = held.iter().find(|h| h.item_id == item_id) else {
        return Err(CraftReject::ComponentMissing { item_id });
    };
    if !CRAFTING_INPUT_BAGS.contains(&item.container_id) {
        return Err(CraftReject::ComponentNotInCraftingBags {
            item_id,
            container_id: item.container_id,
        });
    }
    Ok(*item)
}
