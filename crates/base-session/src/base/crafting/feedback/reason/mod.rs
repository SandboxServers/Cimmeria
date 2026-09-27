//! [`CraftReject`]: why a crafting request was refused, with what the player
//! reads, the enumerated `reason`, the values the rule compared, and the
//! optional condition code.
//!
//! The enum, its `reason` label and code live here; the player's line is
//! in `text.rs`, the values a refusal compared in `compared.rs`.

use cimmeria_cell_catalog::crafting::{
    CraftType, CONDITION_FEEDBACK_NOT_ENOUGH_APPLIED_SCIENCE_POINTS,
};

use crate::cell::messages::CraftVerb;

mod compared;
mod text;

pub use compared::Compared;

/// Why a crafting request was refused. A new reason needs an arm in every
/// method below and in `text.rs` and `compared.rs`; the `reason` strings
/// are metric labels, so keep them to this fixed set.
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
mod tests;
