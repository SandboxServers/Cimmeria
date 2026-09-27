//! [`CraftReject`]: why a crafting request was refused, with what the player
//! reads, the enumerated `reason`, the values the rule compared, and the
//! optional condition code.

use cimmeria_cell_catalog::crafting::{
    CraftType, CONDITION_FEEDBACK_NOT_ENOUGH_APPLIED_SCIENCE_POINTS,
};

use crate::base::crafting::item_use::MAX_RACIAL_PARADIGM_LEVEL;
use crate::cell::messages::CraftVerb;

/// Why a crafting request was refused. A new reason needs an arm in every
/// method below; the `reason` strings are metric labels, so keep them to
/// this fixed set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CraftReject {
    /// The verb has no server implementation yet.
    NotAvailableYet {
        /// What the player tried, as a sentence subject ("Alloying").
        action: &'static str,
    },
    /// The server could not decide the request (no database, the catalog
    /// or the transaction failed). Nothing changed.
    Unavailable {
        /// What the player tried, as a sentence subject.
        action: &'static str,
    },
    /// A discipline the catalog does not have. The client only offers
    /// catalog disciplines, so this is a forged or stale request.
    UnknownDiscipline { discipline_id: i32 },
    /// The discipline is already known. Also what a replayed spend gets.
    DisciplineAlreadyKnown { discipline_id: i32, name: String },
    /// No unspent applied science points; `asp` is what the player has.
    NoAppliedSciencePoints { asp: i32 },
    /// The discipline's racial paradigm is below the level it needs.
    ParadigmTooLow {
        discipline_id: i32,
        discipline: String,
        paradigm_id: i32,
        paradigm: &'static str,
        required: i32,
        have: i32,
    },
    /// A required discipline is not known at all.
    PrerequisiteMissing {
        discipline_id: i32,
        discipline: String,
        prerequisite_id: i32,
        prerequisite: String,
    },
    /// A required discipline is known, but below the expertise it needs.
    PrerequisiteExpertise {
        discipline_id: i32,
        discipline: String,
        prerequisite_id: i32,
        prerequisite: String,
        expertise: i32,
        required: i32,
    },
    /// No station in reach, no covering tool in the crafting bag, and no
    /// "craft anywhere".
    NoStationOrTool {
        verb: CraftType,
        /// The `ECraftTypeFlags` mask the cell's station check granted.
        station_mask: u8,
        /// The instance ids of the tools in the crafting bag that were
        /// considered (empty when none was, e.g. for alloying).
        tools: Vec<i32>,
    },
    /// The player already has the maximum number of inductions running or
    /// queued.
    QueueFull {
        /// The queue limit, active induction included.
        limit: usize,
    },
    /// A component the request named is gone from the player's inventory
    /// (used, sold, dropped or never theirs).
    ComponentMissing { item_id: i32 },
    /// A component the request named is no longer in the main bag or the
    /// crafting bag (moved to the bank, equipped, ...).
    ComponentNotInCraftingBags { item_id: i32, container_id: i32 },
    /// A component the request named is not of the design it was named
    /// for (a forged or stale request).
    ComponentMismatch {
        item_id: i32,
        expected_design_id: i32,
        type_id: i32,
    },
    /// The main and crafting bags hold fewer of a component than needed.
    NotEnoughComponents {
        design_id: i32,
        needed: i32,
        available: i64,
    },
    /// No room for the product in the bag it goes to.
    InventoryFull { design_id: i32, container_id: i32 },
    /// The product's `container_sets` allow no carried bag.
    NoCarriedBagForProduct { design_id: i32 },
    /// An induction's completion transaction failed for a server-side
    /// reason and was rolled back. Nothing was used.
    InductionFailed,
    /// A Blueprint item whose every blueprint is already known. The item is
    /// not used.
    BlueprintAlreadyKnown {
        /// The item design (`resources.items.item_id`).
        type_id: i32,
        blueprint_ids: Vec<i32>,
    },
    /// A Racial Paradigm Guide for a paradigm already at the maximum level.
    /// The item is not used.
    ParadigmAtMax {
        type_id: i32,
        paradigm_id: i32,
        paradigm: &'static str,
        level: i8,
    },
    /// The used item instance is no longer the player's (used, moved away,
    /// traded or never theirs).
    ItemMissing { item_id: i32 },
    /// The used item is not in a carried bag (the bank, the buyback list,
    /// an equipment slot).
    ItemNotCarried {
        item_id: i32,
        type_id: i32,
        container_id: i32,
    },
    /// The item to research is not flagged researchable.
    NotResearchable { item_id: i32, type_id: i32 },
    /// A kicker is not flagged as one, or has no applied science to count
    /// against.
    NotKicker { item_id: i32, type_id: i32 },
    /// A kicker of the researched item's own applied science.
    KickerSameScience {
        item_id: i32,
        type_id: i32,
        applied_science_id: i32,
    },
    /// A second kicker of an applied science that already has one.
    KickerDuplicateScience {
        item_id: i32,
        type_id: i32,
        applied_science_id: i32,
    },
    /// The item to reverse engineer is not flagged reverse-engineerable.
    NotReverseEngineerable { item_id: i32, type_id: i32 },
    /// No blueprint with a recipe makes the item.
    NoBlueprintForItem { item_id: i32, type_id: i32 },
}

/// The values a refused rule compared, logged as fields of the `rejected`
/// event. `None` fields are omitted from the event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Compared {
    pub discipline_id: Option<i32>,
    pub asp: Option<i32>,
    pub paradigm_id: Option<i32>,
    pub paradigm_level: Option<i32>,
    pub required_level: Option<i32>,
    pub prerequisite_id: Option<i32>,
    pub prerequisite_expertise: Option<i32>,
    pub required_expertise: Option<i32>,
    pub station_mask: Option<u8>,
    /// The item instance a component check refused.
    pub item_id: Option<i32>,
    /// The item design a consumption or placement refused, or the design a
    /// named instance was expected to be.
    pub design_id: Option<i32>,
    /// The design a named instance actually is.
    pub type_id: Option<i32>,
    /// The bag the refused instance sits in, or the product's bag.
    pub container_id: Option<i32>,
    /// How many of `design_id` the plan needs, against `available` in the
    /// main and crafting bags.
    pub needed: Option<i64>,
    pub available: Option<i64>,
    /// The induction limit a full queue hit.
    pub queue_limit: Option<usize>,
    /// The applied science a kicker rule compared.
    pub applied_science_id: Option<i32>,
}

impl CraftReject {
    /// The rejection for a verb whose handler has not landed.
    pub fn not_available(verb: &CraftVerb) -> Self {
        let action = match verb {
            CraftVerb::Spend { .. } => "Learning disciplines",
            CraftVerb::Craft { .. } => "Crafting",
            CraftVerb::Research { .. } => "Research",
            CraftVerb::ReverseEngineer { .. } => "Reverse engineering",
            CraftVerb::Alloy { .. } => "Alloying",
            CraftVerb::Respec => "Crafting respec",
        };
        CraftReject::NotAvailableYet { action }
    }

    /// The `reason` field of the `rejected` event and the `reason` label of
    /// `crafting_rejections_total`.
    pub fn reason(&self) -> &'static str {
        match self {
            CraftReject::NotAvailableYet { .. } => "not_available_yet",
            CraftReject::Unavailable { .. } => "unavailable",
            CraftReject::UnknownDiscipline { .. } => "unknown_discipline",
            CraftReject::DisciplineAlreadyKnown { .. } => "already_known",
            CraftReject::NoAppliedSciencePoints { .. } => "no_asp",
            CraftReject::ParadigmTooLow { .. } => "paradigm_too_low",
            CraftReject::PrerequisiteMissing { .. } => "prerequisite_missing",
            CraftReject::PrerequisiteExpertise { .. } => "prerequisite_expertise",
            CraftReject::NoStationOrTool { .. } => "no_station_or_tool",
            CraftReject::QueueFull { .. } => "queue_full",
            CraftReject::ComponentMissing { .. } => "component_missing",
            CraftReject::ComponentNotInCraftingBags { .. } => "component_not_in_crafting_bags",
            CraftReject::ComponentMismatch { .. } => "component_mismatch",
            CraftReject::NotEnoughComponents { .. } => "not_enough_components",
            CraftReject::InventoryFull { .. } => "inventory_full",
            CraftReject::NoCarriedBagForProduct { .. } => "no_carried_bag_for_product",
            CraftReject::InductionFailed => "induction_failed",
            CraftReject::BlueprintAlreadyKnown { .. } => "already_known",
            CraftReject::ParadigmAtMax { .. } => "paradigm_max",
            CraftReject::ItemMissing { .. } => "item_missing",
            CraftReject::ItemNotCarried { .. } => "not_carried",
            CraftReject::NotResearchable { .. } => "not_researchable",
            CraftReject::NotKicker { .. } => "not_kicker",
            CraftReject::KickerSameScience { .. } => "kicker_same_science",
            CraftReject::KickerDuplicateScience { .. } => "kicker_duplicate_science",
            CraftReject::NotReverseEngineerable { .. } => "not_reverse_engineerable",
            CraftReject::NoBlueprintForItem { .. } => "no_blueprint_for_item",
        }
    }

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
        }
    }

    /// What the refused rule compared.
    pub fn compared(&self) -> Compared {
        match *self {
            CraftReject::NotAvailableYet { .. }
            | CraftReject::Unavailable { .. }
            | CraftReject::InductionFailed => Compared::default(),
            CraftReject::QueueFull { limit } => Compared {
                queue_limit: Some(limit),
                ..Compared::default()
            },
            CraftReject::ComponentMissing { item_id } => Compared {
                item_id: Some(item_id),
                ..Compared::default()
            },
            CraftReject::ComponentNotInCraftingBags {
                item_id,
                container_id,
            } => Compared {
                item_id: Some(item_id),
                container_id: Some(container_id),
                ..Compared::default()
            },
            CraftReject::ComponentMismatch {
                item_id,
                expected_design_id,
                type_id,
            } => Compared {
                item_id: Some(item_id),
                design_id: Some(expected_design_id),
                type_id: Some(type_id),
                ..Compared::default()
            },
            CraftReject::NotEnoughComponents {
                design_id,
                needed,
                available,
            } => Compared {
                design_id: Some(design_id),
                needed: Some(i64::from(needed)),
                available: Some(available),
                ..Compared::default()
            },
            CraftReject::InventoryFull {
                design_id,
                container_id,
            } => Compared {
                design_id: Some(design_id),
                container_id: Some(container_id),
                ..Compared::default()
            },
            CraftReject::NoCarriedBagForProduct { design_id } => Compared {
                design_id: Some(design_id),
                ..Compared::default()
            },
            CraftReject::NotResearchable { item_id, type_id }
            | CraftReject::NotKicker { item_id, type_id }
            | CraftReject::NotReverseEngineerable { item_id, type_id }
            | CraftReject::NoBlueprintForItem { item_id, type_id } => Compared {
                item_id: Some(item_id),
                type_id: Some(type_id),
                ..Compared::default()
            },
            CraftReject::KickerSameScience {
                item_id,
                type_id,
                applied_science_id,
            }
            | CraftReject::KickerDuplicateScience {
                item_id,
                type_id,
                applied_science_id,
            } => Compared {
                item_id: Some(item_id),
                type_id: Some(type_id),
                applied_science_id: Some(applied_science_id),
                ..Compared::default()
            },
            CraftReject::NoStationOrTool { station_mask, .. } => Compared {
                station_mask: Some(station_mask),
                ..Compared::default()
            },
            CraftReject::UnknownDiscipline { discipline_id }
            | CraftReject::DisciplineAlreadyKnown { discipline_id, .. } => Compared {
                discipline_id: Some(discipline_id),
                ..Compared::default()
            },
            CraftReject::NoAppliedSciencePoints { asp } => Compared {
                asp: Some(asp),
                ..Compared::default()
            },
            CraftReject::ParadigmTooLow {
                discipline_id,
                paradigm_id,
                required,
                have,
                ..
            } => Compared {
                discipline_id: Some(discipline_id),
                paradigm_id: Some(paradigm_id),
                paradigm_level: Some(have),
                required_level: Some(required),
                ..Compared::default()
            },
            CraftReject::PrerequisiteMissing {
                discipline_id,
                prerequisite_id,
                ..
            } => Compared {
                discipline_id: Some(discipline_id),
                prerequisite_id: Some(prerequisite_id),
                ..Compared::default()
            },
            CraftReject::PrerequisiteExpertise {
                discipline_id,
                prerequisite_id,
                expertise,
                required,
                ..
            } => Compared {
                discipline_id: Some(discipline_id),
                prerequisite_id: Some(prerequisite_id),
                prerequisite_expertise: Some(expertise),
                required_expertise: Some(required),
                ..Compared::default()
            },
            CraftReject::BlueprintAlreadyKnown { type_id, .. } => Compared {
                design_id: Some(type_id),
                ..Compared::default()
            },
            CraftReject::ParadigmAtMax {
                type_id,
                paradigm_id,
                level,
                ..
            } => Compared {
                design_id: Some(type_id),
                paradigm_id: Some(paradigm_id),
                paradigm_level: Some(i32::from(level)),
                required_level: Some(i32::from(MAX_RACIAL_PARADIGM_LEVEL)),
                ..Compared::default()
            },
            CraftReject::ItemMissing { item_id } => Compared {
                item_id: Some(item_id),
                ..Compared::default()
            },
            CraftReject::ItemNotCarried {
                item_id,
                type_id,
                container_id,
            } => Compared {
                item_id: Some(item_id),
                design_id: Some(type_id),
                container_id: Some(container_id),
                ..Compared::default()
            },
        }
    }

    /// The blueprints an already-known refusal compared, as the
    /// `blueprint_ids` field of the `rejected` event; `None` for every other
    /// reason.
    pub fn blueprints_considered(&self) -> Option<String> {
        match self {
            CraftReject::BlueprintAlreadyKnown { blueprint_ids, .. } => {
                Some(format!("{blueprint_ids:?}"))
            }
            _ => None,
        }
    }

    /// The crafting-bag tools a station gate refusal considered, as the
    /// `tools` field of the `rejected` event; `None` for every other reason.
    pub fn tools_considered(&self) -> Option<String> {
        match self {
            CraftReject::NoStationOrTool { tools, .. } => Some(format!("{tools:?}")),
            _ => None,
        }
    }

    /// The `EConditionHandlerFeedback` value sent as a secondary
    /// `onErrorCode`. Only the not-enough-ASP code qualifies; every other
    /// reason is text only.
    pub fn error_code(&self) -> Option<u16> {
        match self {
            CraftReject::NoAppliedSciencePoints { .. } => {
                Some(CONDITION_FEEDBACK_NOT_ENOUGH_APPLIED_SCIENCE_POINTS)
            }
            CraftReject::NotAvailableYet { .. }
            | CraftReject::Unavailable { .. }
            | CraftReject::UnknownDiscipline { .. }
            | CraftReject::DisciplineAlreadyKnown { .. }
            | CraftReject::ParadigmTooLow { .. }
            | CraftReject::PrerequisiteMissing { .. }
            | CraftReject::PrerequisiteExpertise { .. }
            | CraftReject::NoStationOrTool { .. }
            | CraftReject::QueueFull { .. }
            | CraftReject::ComponentMissing { .. }
            | CraftReject::ComponentNotInCraftingBags { .. }
            | CraftReject::ComponentMismatch { .. }
            | CraftReject::NotEnoughComponents { .. }
            | CraftReject::InventoryFull { .. }
            | CraftReject::NoCarriedBagForProduct { .. }
            | CraftReject::InductionFailed => None,
            CraftReject::BlueprintAlreadyKnown { .. }
            | CraftReject::ParadigmAtMax { .. }
            | CraftReject::ItemMissing { .. }
            | CraftReject::ItemNotCarried { .. } => None,
            CraftReject::NotResearchable { .. }
            | CraftReject::NotKicker { .. }
            | CraftReject::KickerSameScience { .. }
            | CraftReject::KickerDuplicateScience { .. }
            | CraftReject::NotReverseEngineerable { .. }
            | CraftReject::NoBlueprintForItem { .. } => None,
        }
    }
}

#[cfg(test)]
#[path = "reason_tests.rs"]
mod tests;
