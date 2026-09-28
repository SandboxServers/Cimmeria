//! [`Compared`]: the values a refused rule compared, and
//! [`CraftReject::compared`], which fills them per reason.

use super::CraftReject;
use crate::base::crafting::item_use::MAX_RACIAL_PARADIGM_LEVEL;

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
    /// The blueprint a blueprint rule refused.
    pub blueprint_id: Option<i32>,
    /// An elementary component's tier, against the tier the alloy needs.
    pub tier: Option<i32>,
    pub required_tier: Option<i32>,
    /// The quantity a craft asked for.
    pub quantity: Option<i32>,
}

impl CraftReject {
    /// What the refused rule compared.
    pub fn compared(&self) -> Compared {
        match *self {
            CraftReject::NotAvailableYet { .. }
            | CraftReject::Unavailable { .. }
            | CraftReject::InductionFailed
            | CraftReject::NothingToRespec
            | CraftReject::NoPendingRespec
            | CraftReject::RespecExpired { .. } => Compared::default(),
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
            CraftReject::UnknownBlueprint { blueprint_id }
            | CraftReject::NotAlloy { blueprint_id } => Compared {
                blueprint_id: Some(blueprint_id),
                ..Compared::default()
            },
            CraftReject::DisciplineUnknown {
                blueprint_id,
                discipline_id,
            } => Compared {
                blueprint_id: Some(blueprint_id),
                discipline_id: Some(discipline_id),
                ..Compared::default()
            },
            CraftReject::WrongTier {
                item_id,
                type_id,
                tier,
                required_tier,
            } => Compared {
                item_id: Some(item_id),
                type_id: Some(type_id),
                tier: Some(tier),
                required_tier: Some(required_tier),
                ..Compared::default()
            },
            CraftReject::CountNotMet { .. } | CraftReject::MultipleBuckets { .. } => {
                Compared::default()
            }
            CraftReject::IsAlloy { blueprint_id }
            | CraftReject::NoComponentSet { blueprint_id, .. } => Compared {
                blueprint_id: Some(blueprint_id),
                ..Compared::default()
            },
            CraftReject::BadQuantity {
                blueprint_id,
                quantity,
                ..
            } => Compared {
                blueprint_id: Some(blueprint_id),
                quantity: Some(quantity),
                ..Compared::default()
            },
            CraftReject::InsufficientComponents {
                blueprint_id,
                design_id,
                needed,
                available,
            } => Compared {
                blueprint_id: Some(blueprint_id),
                design_id: Some(design_id),
                needed: Some(i64::from(needed)),
                available: Some(available),
                ..Compared::default()
            },
        }
    }
}
