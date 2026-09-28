//! [`CraftReject::text`]: the line the player reads for each refusal.

use cimmeria_cell_catalog::crafting::CraftType;

use super::CraftReject;

impl CraftReject {
    /// The line the player reads.
    pub fn text(&self) -> String {
        match self {
            CraftReject::NotAvailableYet { action } => {
                format!("{action} is not available yet.")
            }
            CraftReject::Unavailable { action } => {
                format!("{action} is unavailable right now. Nothing was changed.")
            }
            CraftReject::UnknownDiscipline { discipline_id } => {
                format!("There is no discipline {discipline_id}.")
            }
            CraftReject::DisciplineAlreadyKnown { name, .. } => format!("You already know {name}."),
            CraftReject::NoAppliedSciencePoints { .. } => {
                "You have no applied science points.".to_string()
            }
            CraftReject::ParadigmTooLow {
                discipline,
                paradigm,
                required,
                have,
                ..
            } => format!(
                "{discipline} requires {paradigm} paradigm level {required}; yours is {have}."
            ),
            CraftReject::PrerequisiteMissing {
                discipline,
                prerequisite,
                ..
            } => format!("{discipline} requires {prerequisite} at expertise 50."),
            CraftReject::PrerequisiteExpertise {
                discipline,
                prerequisite,
                expertise,
                required,
                ..
            } => format!(
                "{discipline} requires {prerequisite} at expertise {required}; yours is {expertise}."
            ),
            CraftReject::NoStationOrTool { verb, .. } => {
                let verb = match verb {
                    CraftType::Craft => "crafting",
                    CraftType::Research => "research",
                    CraftType::ReverseEngineering => "reverse engineering",
                    CraftType::Alloying => "alloying",
                };
                format!("No crafting station or tool for {verb} nearby.")
            }
            CraftReject::QueueFull { limit } => {
                format!("You can have at most {limit} crafting jobs at once.")
            }
            CraftReject::ComponentMissing { .. } => {
                "A component is no longer in your inventory. Nothing was used.".to_string()
            }
            CraftReject::ComponentNotInCraftingBags { .. } => {
                "Components must be in your backpack or crafting bag. Nothing was used.".to_string()
            }
            CraftReject::ComponentMismatch { .. } => {
                "A chosen component is not the one this needs. Nothing was used.".to_string()
            }
            CraftReject::NotEnoughComponents { .. } => {
                "You do not have enough components. Nothing was used.".to_string()
            }
            CraftReject::InventoryFull { .. } => {
                "Not enough room in your bags for the result. Nothing was used.".to_string()
            }
            CraftReject::NoCarriedBagForProduct { .. } => {
                "The result cannot be placed in your bags. Nothing was used.".to_string()
            }
            CraftReject::InductionFailed => "Crafting failed. Nothing was used.".to_string(),
            CraftReject::BlueprintAlreadyKnown { blueprint_ids, .. } => {
                if blueprint_ids.len() == 1 {
                    "You already know this blueprint. The item was not used.".to_string()
                } else {
                    "You already know these blueprints. The item was not used.".to_string()
                }
            }
            CraftReject::ParadigmAtMax {
                paradigm, level, ..
            } => format!(
                "Your {paradigm} racial paradigm is already at {level}, the maximum. \
                 The guide was not used."
            ),
            CraftReject::ItemMissing { .. } => {
                "That item is no longer in your inventory.".to_string()
            }
            CraftReject::ItemNotCarried { .. } => {
                "Move that item to your crafting bag to use it.".to_string()
            }
            CraftReject::NotResearchable { .. } => {
                "That item cannot be researched. Nothing was used.".to_string()
            }
            CraftReject::NotKicker { .. } => {
                "That item is not a research kicker. Nothing was used.".to_string()
            }
            CraftReject::KickerSameScience { .. } => {
                "Kickers cannot come from the same applied science as the item being researched. \
                 Nothing was used."
                    .to_string()
            }
            CraftReject::KickerDuplicateScience { .. } => {
                "Only one kicker per applied science can be used. Nothing was used.".to_string()
            }
            CraftReject::NotReverseEngineerable { .. } => {
                "That item cannot be reverse engineered. Nothing was used.".to_string()
            }
            CraftReject::NoBlueprintForItem { .. } => {
                "No known recipe makes that item, so it cannot be reverse engineered. \
                 Nothing was used."
                    .to_string()
            }
            CraftReject::UnknownBlueprint { .. } => "You do not know that blueprint.".to_string(),
            CraftReject::DisciplineUnknown { .. } => {
                "You must learn the blueprint's discipline first.".to_string()
            }
            CraftReject::NotAlloy { .. } => "That blueprint is not an alloy.".to_string(),
            CraftReject::WrongTier { required_tier, .. } => format!(
                "Elementary components must be one tier lower than the component (tier {required_tier}). Nothing was used."
            ),
            CraftReject::CountNotMet { .. } => {
                "The quantity of elementary components per item quality was not met: 10 Normal, 5 Good, 2 Great or 1 Fantastic. Nothing was used."
                    .to_string()
            }
            CraftReject::MultipleBuckets { .. } => {
                "Multiple categories of elementary components were met; use one quality only. Nothing was used."
                    .to_string()
            }
            CraftReject::IsAlloy { .. } => {
                "That blueprint is an alloy. Use alloying to make it.".to_string()
            }
            CraftReject::BadQuantity { max, .. } => {
                format!("You can craft between 1 and {max} at a time.")
            }
            CraftReject::NoComponentSet { .. } => {
                "Those components do not match any recipe of this blueprint. Nothing was used."
                    .to_string()
            }
            CraftReject::InsufficientComponents {
                needed, available, ..
            } => format!(
                "You do not have enough components: {available} of {needed} needed. Nothing was used."
            ),
            CraftReject::NothingToRespec => {
                "You have no crafting disciplines to unlearn. Nothing was changed.".to_string()
            }
            CraftReject::NoPendingRespec => {
                "No crafting respec is waiting to be confirmed. Type .respeccraft to start one."
                    .to_string()
            }
            CraftReject::RespecExpired { window_secs } => format!(
                "The crafting respec was not confirmed within {window_secs} seconds. \
                 Type .respeccraft to start again."
            ),
        }
    }
}
