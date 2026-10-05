//! The payload of [`crate::actions::Action::GrantAbility`] (Class Start v6,
//! CS-01a): teach the acting player abilities for free, with a recorded
//! provenance.
//!
//! Not GM-gated: tutorials, racial cores, class signatures and mission
//! rewards hand out abilities this way. The base, in one transaction under
//! the `sgw_player` row lock:
//!
//! - appends an unknown id to `sgw_player.abilities` with a provenance row;
//! - gives a known id with no row its row (a `gm` row is promoted);
//! - converts an id the player **bought** from a trainer into a grant
//!   (OD-CS06, "costs no point"): out of `trained_abilities`, its
//!   `skill_point_cost` refunded to `training_points` and taken off
//!   `tree_points_spent`, and the content row written, so a later respec
//!   keeps it;
//! - writes nothing when the player's archetype is not in `archetypes`.
//!
//! A replay changes nothing. The player gets `onKnownAbilitiesUpdate` and
//! one "You have learned ..." line per new ability.

use serde::{Deserialize, Serialize};

use crate::actions::AbilityGrantKind;

/// One `grant_ability` row, validated by the loader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityGrant {
    /// Distinct `resources.abilities` ids, in authored order. Checked at
    /// load ([`crate::loader::refuse_chains_with_unknown_abilities`]).
    pub ability_ids: Vec<i32>,
    /// Never [`AbilityGrantKind::Gm`]: the loader refuses the chain.
    pub source_kind: AbilityGrantKind,
    /// The mission or chain id the grant comes from, if any.
    pub source_id: Option<i32>,
    /// `EArchetype` ordinals allowed to receive it; empty means every
    /// archetype. Required (non-empty) for `signature` and `racial_core`.
    /// The executor and the base check it against the player's own
    /// archetype, so a chain never depends on its trigger's `archetype`
    /// condition alone.
    pub archetypes: Vec<i32>,
}

impl AbilityGrant {
    /// Whether a player of `archetype` may receive this grant.
    pub fn allows(&self, archetype: Option<i32>) -> bool {
        self.archetypes.is_empty() || archetype.is_some_and(|a| self.archetypes.contains(&a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(archetypes: Vec<i32>) -> AbilityGrant {
        AbilityGrant {
            ability_ids: vec![598],
            source_kind: AbilityGrantKind::Signature,
            source_id: None,
            archetypes,
        }
    }

    #[test]
    fn an_archetype_list_gates_and_an_empty_one_allows_all() {
        assert!(grant(vec![1]).allows(Some(1)));
        assert!(!grant(vec![1]).allows(Some(2)));
        assert!(!grant(vec![1]).allows(None), "no archetype is no match");
        assert!(grant(vec![]).allows(Some(7)));
    }
}
