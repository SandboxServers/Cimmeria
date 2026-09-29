//! The pure half of research: which items and kickers a request may use,
//! which of the item's disciplines the player can research it in, and the
//! roll at completion.

use cimmeria_cell_catalog::crafting::{CraftItemAttrs, CraftingCatalog};
use cimmeria_entity::crafting::CraftingState;

use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::item_lookup::HeldInstance;
use crate::base::crafting::rng::CraftRng;
use crate::base::crafting::transaction::ResearchedItem;

/// Expertise a successful research adds to the rolled discipline.
pub const RESEARCH_EXPERTISE_GAIN: i32 = 5;

/// Percentage points each kicker adds to the research chance.
pub const KICKER_BONUS: i32 = 5;

/// Check the researched item and its kickers.
///
/// The item must be flagged researchable. Each kicker must be flagged a
/// kicker and carry an applied science, which is neither the item's own
/// nor the science of an earlier kicker: the client's research page has one
/// kicker slot per science and refuses the item's own
/// (`ResearchPage.lua` `addKickerToWindow`), and its sender sends anyway
/// when its checks fail, so the server repeats them.
pub fn check_request(
    catalog: &CraftingCatalog,
    item: &HeldInstance,
    kickers: &[HeldInstance],
) -> Result<(), CraftReject> {
    let not_researchable = CraftReject::NotResearchable {
        item_id: item.item_id,
        type_id: item.type_id,
    };
    let attrs = catalog.item(item.type_id).ok_or(not_researchable.clone())?;
    if !attrs.flags.is_researchable() {
        return Err(not_researchable);
    }
    let mut sciences = Vec::with_capacity(kickers.len());
    for kicker in kickers {
        let (item_id, type_id) = (kicker.item_id, kicker.type_id);
        let science = catalog
            .item(type_id)
            .filter(|k| k.flags.is_kicker())
            .and_then(|k| k.applied_science_id)
            .ok_or(CraftReject::NotKicker { item_id, type_id })?;
        if attrs.applied_science_id == Some(science) {
            return Err(CraftReject::KickerSameScience {
                item_id,
                type_id,
                applied_science_id: science,
            });
        }
        if sciences.contains(&science) {
            return Err(CraftReject::KickerDuplicateScience {
                item_id,
                type_id,
                applied_science_id: science,
            });
        }
        sciences.push(science);
    }
    Ok(())
}

/// `item` with what the eligibility rule needs from its catalog row.
pub fn researched_item(item: &HeldInstance, attrs: &CraftItemAttrs) -> ResearchedItem {
    ResearchedItem {
        item_id: item.item_id,
        type_id: item.type_id,
        applied_science_id: attrs.applied_science_id,
        tech_comp: attrs.tech_comp,
        discipline_ids: attrs.discipline_ids.clone(),
    }
}

/// The disciplines of `discipline_ids` the player can research an item of
/// tech competency `tech_comp` in: known, with `0 < expertise <
/// tech_comp`, in id order. An expertise row for a discipline not in the
/// known list does not count.
pub fn eligible_disciplines(
    discipline_ids: &[i32],
    tech_comp: i32,
    state: &CraftingState,
) -> Vec<i32> {
    let mut eligible: Vec<i32> = discipline_ids
        .iter()
        .copied()
        .filter(|&d| {
            state.knows_discipline(d)
                && state
                    .get_expertise(d)
                    .is_some_and(|e| 0 < e && e < tech_comp)
        })
        .collect();
    eligible.sort_unstable();
    eligible.dedup();
    eligible
}

/// Refuse a research the player could learn nothing from: none of the
/// item's disciplines is eligible ([`eligible_disciplines`]). Checked at
/// the request, and again inside the completion transaction, so a refused
/// research never uses the item or its kickers. `Ok` carries the eligible
/// disciplines.
pub fn check_eligible(
    item: &ResearchedItem,
    state: &CraftingState,
) -> Result<Vec<i32>, CraftReject> {
    let eligible = eligible_disciplines(&item.discipline_ids, item.tech_comp, state);
    if !eligible.is_empty() {
        return Ok(eligible);
    }
    let mut item_disciplines = item.discipline_ids.clone();
    item_disciplines.sort_unstable();
    item_disciplines.dedup();
    let mut known: Vec<(i32, i32)> = state
        .discipline_ids
        .iter()
        .map(|&d| (d, state.get_expertise(d).unwrap_or(0)))
        .collect();
    known.sort_unstable();
    known.dedup();
    Err(CraftReject::NoEligibleDiscipline {
        item_id: item.item_id,
        type_id: item.type_id,
        applied_science_id: item.applied_science_id,
        tech_comp: item.tech_comp,
        item_disciplines,
        known,
    })
}

/// The outcome of one research roll.
#[derive(Debug, Clone, PartialEq)]
pub struct ResearchRoll {
    /// The item's disciplines the player knows with `0 < expertise <
    /// tech competency`, in id order.
    pub eligible: Vec<i32>,
    /// The discipline picked from `eligible`; `None` when it is empty.
    pub discipline_id: Option<i32>,
    /// `100 − expertise + 5 × kickers`, in percent.
    pub chance: Option<f64>,
    /// The roll in `[0, 100)`; a success is `roll < chance`.
    pub roll: Option<f64>,
    pub success: bool,
}

/// Roll a research of an item with `attrs` and `kickers` kickers, against
/// the player's `state`. The discipline is picked uniformly from the
/// eligible ones (one sample), then the chance is rolled (a second). With
/// no eligible discipline nothing is rolled and the research fails; the
/// request and the transaction both refuse that case first
/// ([`check_eligible`]), so a completion never applies it.
pub fn roll(
    attrs: &CraftItemAttrs,
    state: &CraftingState,
    kickers: usize,
    rng: &mut dyn CraftRng,
) -> ResearchRoll {
    let eligible = eligible_disciplines(&attrs.discipline_ids, attrs.tech_comp, state);
    if eligible.is_empty() {
        return ResearchRoll {
            eligible,
            discipline_id: None,
            chance: None,
            roll: None,
            success: false,
        };
    }
    let pick = ((rng.unit() * eligible.len() as f64) as usize).min(eligible.len() - 1);
    let discipline_id = eligible[pick];
    let expertise = state.get_expertise(discipline_id).unwrap_or(0);
    let chance = f64::from(100 - expertise) + f64::from(KICKER_BONUS) * kickers as f64;
    let roll = rng.unit() * 100.0;
    ResearchRoll {
        eligible,
        discipline_id: Some(discipline_id),
        chance: Some(chance),
        roll: Some(roll),
        success: roll < chance,
    }
}

/// The blueprints a successful research of `type_id` teaches, as
/// `(blueprint_id, discipline_id)`: every one whose product it is and whose
/// discipline the player knows, that the player does not know yet, in id
/// order. The transaction checks the discipline again under its lock.
pub fn blueprints_taught(
    catalog: &CraftingCatalog,
    state: &CraftingState,
    type_id: i32,
) -> Vec<(i32, i32)> {
    let mut found: Vec<(i32, i32)> = catalog
        .blueprints
        .values()
        .filter(|b| b.product_id == Some(type_id))
        .filter(|b| !state.blueprint_ids.contains(&b.blueprint_id))
        .filter_map(|b| {
            let d = b.discipline_id?;
            state.knows_discipline(d).then_some((b.blueprint_id, d))
        })
        .collect();
    found.sort_unstable();
    found
}
