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
    /// None of the researched item's disciplines is one the player knows
    /// with `0 < expertise < tech_comp`, so research could teach nothing.
    /// Refused at the request and again inside the completion transaction.
    NoEligibleDiscipline {
        item_id: i32,
        type_id: i32,
        applied_science_id: Option<i32>,
        tech_comp: i32,
        /// The item's disciplines, in id order.
        item_disciplines: Vec<i32>,
        /// The player's known disciplines as `(discipline_id, expertise)`,
        /// in id order; a known discipline with no expertise row is 0.
        known: Vec<(i32, i32)>,
    },
    /// The item to reverse engineer is not flagged reverse-engineerable.
    NotReverseEngineerable { item_id: i32, type_id: i32 },
    /// No blueprint with a recipe makes the item.
    NoBlueprintForItem { item_id: i32, type_id: i32 },
    /// A blueprint the catalog does not have, or the player has not
    /// learned. The client lists only known blueprints, so this is a
    /// forged or stale request.
    UnknownBlueprint { blueprint_id: i32 },
    /// The blueprint is known but its discipline is not.
    DisciplineUnknown {
        blueprint_id: i32,
        discipline_id: i32,
    },
    /// An alloy request named a blueprint that is not an alloy.
    NotAlloy { blueprint_id: i32 },
    /// An elementary component is not exactly one tier below the alloy's
    /// component.
    WrongTier {
        item_id: i32,
        type_id: i32,
        tier: i32,
        required_tier: i32,
    },
    /// No quality's elementary count was met. `counts` is the summed stack
    /// quantity per quality: Normal, Good, Great, Fantastic.
    CountNotMet { counts: [i64; 4] },
    /// More than one quality's elementary count was met at once.
    MultipleBuckets { counts: [i64; 4] },
    /// `craft` for an alloy blueprint, which only `alloying` makes.
    IsAlloy { blueprint_id: i32 },
    /// A craft quantity outside `1..=max`.
    BadQuantity {
        blueprint_id: i32,
        quantity: i32,
        max: i32,
    },
    /// The submitted components match none of the blueprint's component
    /// sets (so also every craft of a blueprint with no component set).
    NoComponentSet {
        blueprint_id: i32,
        /// The distinct designs of the submitted instances, ascending.
        type_ids: Vec<i32>,
    },
    /// At the request, the main and crafting bags hold fewer of a
    /// component than the chosen set needs for the quantity asked.
    InsufficientComponents {
        blueprint_id: i32,
        design_id: i32,
        needed: i32,
        available: i64,
    },
    /// A respec with no discipline and no expertise to clear.
    NothingToRespec,
    /// `respecCrafting` (100) with no respec open for this player: the
    /// client sends it only from the prompt's Yes, so this is a replay, a
    /// second Yes, or a forged request.
    NoPendingRespec,
    /// `respecCrafting` (100) after the respec window closed.
    RespecExpired {
        /// How long the window stays open, for the text.
        window_secs: u64,
    },
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
            CraftReject::NoEligibleDiscipline { .. } => "no_eligible_discipline",
            CraftReject::NotReverseEngineerable { .. } => "not_reverse_engineerable",
            CraftReject::NoBlueprintForItem { .. } => "no_blueprint_for_item",
            CraftReject::UnknownBlueprint { .. } => "unknown_blueprint",
            CraftReject::DisciplineUnknown { .. } => "discipline_unknown",
            CraftReject::NotAlloy { .. } => "not_alloy",
            CraftReject::WrongTier { .. } => "wrong_tier",
            CraftReject::CountNotMet { .. } => "count_not_met",
            CraftReject::MultipleBuckets { .. } => "multiple_buckets",
            CraftReject::IsAlloy { .. } => "is_alloy",
            CraftReject::BadQuantity { .. } => "bad_quantity",
            CraftReject::NoComponentSet { .. } => "no_component_set",
            CraftReject::InsufficientComponents { .. } => "insufficient_components",
            CraftReject::NothingToRespec => "nothing_to_respec",
            CraftReject::NoPendingRespec => "no_pending_respec",
            CraftReject::RespecExpired { .. } => "respec_expired",
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

    /// The summed elementary stack quantity per quality an alloy count
    /// rule compared, as the `elementary_counts` field of the `rejected`
    /// event (`normal:10,good:0,great:0,fantastic:0`); `None` for every
    /// other reason.
    pub fn elementary_counts(&self) -> Option<String> {
        match self {
            CraftReject::CountNotMet { counts } | CraftReject::MultipleBuckets { counts } => {
                Some(format!(
                    "normal:{},good:{},great:{},fantastic:{}",
                    counts[0], counts[1], counts[2], counts[3]
                ))
            }
            _ => None,
        }
    }

    /// The researched item's disciplines a no-eligible-discipline refusal
    /// compared, as the `item_disciplines` field of the `rejected` event
    /// (`21,22`); `None` for every other reason.
    pub fn item_disciplines(&self) -> Option<String> {
        match self {
            CraftReject::NoEligibleDiscipline {
                item_disciplines, ..
            } => Some(
                item_disciplines
                    .iter()
                    .map(i32::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            _ => None,
        }
    }

    /// The player's known disciplines a no-eligible-discipline refusal
    /// compared, as the `known_disciplines` field of the `rejected` event
    /// (`discipline_id:expertise,…`); `None` for every other reason.
    pub fn known_disciplines(&self) -> Option<String> {
        match self {
            CraftReject::NoEligibleDiscipline { known, .. } => Some(
                known
                    .iter()
                    .map(|(d, e)| format!("{d}:{e}"))
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            _ => None,
        }
    }

    /// The designs a craft submitted, as the `type_ids` field of the
    /// `rejected` event; `None` for every other reason.
    pub fn types_submitted(&self) -> Option<String> {
        match self {
            CraftReject::NoComponentSet { type_ids, .. } => Some(format!("{type_ids:?}")),
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
            | CraftReject::NoEligibleDiscipline { .. }
            | CraftReject::NotReverseEngineerable { .. }
            | CraftReject::NoBlueprintForItem { .. } => None,
            CraftReject::UnknownBlueprint { .. }
            | CraftReject::DisciplineUnknown { .. }
            | CraftReject::NotAlloy { .. }
            | CraftReject::WrongTier { .. }
            | CraftReject::CountNotMet { .. }
            | CraftReject::MultipleBuckets { .. } => None,
            CraftReject::IsAlloy { .. }
            | CraftReject::BadQuantity { .. }
            | CraftReject::NoComponentSet { .. }
            | CraftReject::InsufficientComponents { .. } => None,
            CraftReject::NothingToRespec
            | CraftReject::NoPendingRespec
            | CraftReject::RespecExpired { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests;
