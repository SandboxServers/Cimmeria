//! [`CraftReject`]: why a crafting request was refused, with what the player
//! reads, the enumerated `reason`, the values the rule compared, and the
//! optional condition code.

use cimmeria_cell_catalog::crafting::CONDITION_FEEDBACK_NOT_ENOUGH_APPLIED_SCIENCE_POINTS;

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
        }
    }

    /// What the refused rule compared.
    pub fn compared(&self) -> Compared {
        match *self {
            CraftReject::NotAvailableYet { .. } | CraftReject::Unavailable { .. } => {
                Compared::default()
            }
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
            | CraftReject::PrerequisiteExpertise { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_available_text_names_the_action() {
        let text = |verb| CraftReject::not_available(&verb).text();
        assert_eq!(
            text(CraftVerb::Spend { discipline_id: 21 }),
            "Learning disciplines is not available yet."
        );
        assert_eq!(
            text(CraftVerb::Craft {
                blueprint_id: 412,
                items: vec![],
                quantity: 1
            }),
            "Crafting is not available yet."
        );
        assert_eq!(
            text(CraftVerb::Research {
                item_id: 1,
                kickers: vec![]
            }),
            "Research is not available yet."
        );
        assert_eq!(
            text(CraftVerb::ReverseEngineer { item_id: 1 }),
            "Reverse engineering is not available yet."
        );
        assert_eq!(
            text(CraftVerb::Alloy {
                blueprint_id: 42,
                current_tier_item_id: 1,
                lower_tier_items: vec![]
            }),
            "Alloying is not available yet."
        );
        assert_eq!(
            text(CraftVerb::Respec),
            "Crafting respec is not available yet."
        );
    }

    fn spend_reasons() -> Vec<CraftReject> {
        vec![
            CraftReject::Unavailable {
                action: "Learning disciplines",
            },
            CraftReject::UnknownDiscipline { discipline_id: 9 },
            CraftReject::DisciplineAlreadyKnown {
                discipline_id: 78,
                name: "X".into(),
            },
            CraftReject::NoAppliedSciencePoints { asp: 0 },
            CraftReject::ParadigmTooLow {
                discipline_id: 82,
                discipline: "X".into(),
                paradigm_id: 2,
                paradigm: "Human",
                required: 3,
                have: 1,
            },
            CraftReject::PrerequisiteMissing {
                discipline_id: 79,
                discipline: "X".into(),
                prerequisite_id: 78,
                prerequisite: "Y".into(),
            },
            CraftReject::PrerequisiteExpertise {
                discipline_id: 79,
                discipline: "X".into(),
                prerequisite_id: 78,
                prerequisite: "Y".into(),
                expertise: 49,
                required: 50,
            },
        ]
    }

    /// Only the ASP reason carries a code; every other spend reason is text
    /// only.
    #[test]
    fn only_no_asp_maps_a_condition_code() {
        for why in spend_reasons() {
            let expected = matches!(why, CraftReject::NoAppliedSciencePoints { .. }).then_some(214);
            assert_eq!(why.error_code(), expected, "{why:?}");
        }
    }

    /// The reason vocabulary is the fixed label set the metric documents.
    #[test]
    fn reasons_are_the_documented_labels() {
        let reasons: Vec<&str> = spend_reasons().iter().map(CraftReject::reason).collect();
        assert_eq!(
            reasons,
            [
                "unavailable",
                "unknown_discipline",
                "already_known",
                "no_asp",
                "paradigm_too_low",
                "prerequisite_missing",
                "prerequisite_expertise",
            ]
        );
    }

    /// Each rule refusal reports the two values it compared.
    #[test]
    fn compared_values_name_both_sides() {
        let reasons = spend_reasons();
        assert_eq!(reasons[3].compared().asp, Some(0));
        let paradigm = reasons[4].compared();
        assert_eq!(
            (
                paradigm.paradigm_id,
                paradigm.paradigm_level,
                paradigm.required_level
            ),
            (Some(2), Some(1), Some(3))
        );
        let expertise = reasons[6].compared();
        assert_eq!(
            (
                expertise.prerequisite_id,
                expertise.prerequisite_expertise,
                expertise.required_expertise
            ),
            (Some(78), Some(49), Some(50))
        );
    }
}
