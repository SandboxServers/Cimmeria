//! A player's ability-tree provenance, hydrated from `sgw_player` at world
//! entry through `InitPlayerState`.

use serde::{Deserialize, Serialize};

/// Trainer-purchase provenance for one character.
///
/// `trained_abilities` lists only abilities bought from a trainer (starter
/// and granted abilities are not in it), and `tree_points_spent` is the
/// archetype-wide spend that opens later tree nodes. Both default to empty
/// for NPCs and for characters created before the columns existed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeProgress {
    /// `sgw_player.trained_abilities`.
    pub trained_abilities: Vec<i32>,
    /// `sgw_player.tree_points_spent`.
    pub tree_points_spent: i32,
    /// `sgw_player.training_points`: the unspent points the purchase gate
    /// and the trainer's `trainable` byte compare against a node's cost.
    /// The base owns the debit; the cell mirrors the value it returns on
    /// `AbilityGranted` and on every level-up (`ProgressionChanged`).
    pub training_points: i32,
    /// Abilities with a non-`gm` row in `sgw_player_ability_grants`
    /// (Class Start v6, CS-01a): tutorial, racial core, signature and
    /// mission grants. The spend gate counts each one that is a node of the
    /// player's archetype tree as branch credit (OD-CS06); refunds never do.
    /// GM grants are left out on purpose: they are test scaffolding and a
    /// GM reset removes them.
    pub credited_grants: Vec<i32>,
}

/// `sgw_player_ability_grants.source_kind`: where a free ability came from.
///
/// The database `CHECK` lists the same five values; [`Self::as_str`] is the
/// column text. Every kind survives a trainer respec. Only [`Self::Gm`] is
/// removed by the GM / Debug NPC ability reset, and only it earns no branch
/// credit (lock L6, OD-CS06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityGrantKind {
    /// Taught when a tutorial introduces the system that uses it.
    Tutorial,
    /// A race's core ability (sustain, a racial resource).
    RacialCore,
    /// A class's one free signature ability.
    Signature,
    /// A mission reward.
    Mission,
    /// `.giveability`, GM grant-all or the Debug Area ability granter.
    Gm,
}

impl AbilityGrantKind {
    /// Every kind, in `CHECK` order.
    pub const ALL: [Self; 5] = [
        Self::Tutorial,
        Self::RacialCore,
        Self::Signature,
        Self::Mission,
        Self::Gm,
    ];

    /// The `source_kind` column text, also the content param and log value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tutorial => "tutorial",
            Self::RacialCore => "racial_core",
            Self::Signature => "signature",
            Self::Mission => "mission",
            Self::Gm => "gm",
        }
    }

    /// Parse the column text. `None` for anything the `CHECK` refuses.
    pub fn from_str_opt(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == value)
    }

    /// Whether a grant of this kind counts toward the trainer's spend gate
    /// and survives the GM reset: every kind but [`Self::Gm`].
    pub fn is_credited(self) -> bool {
        self != Self::Gm
    }
}

#[cfg(test)]
mod tests {
    use super::AbilityGrantKind;

    #[test]
    fn grant_kind_round_trips_its_column_text() {
        for kind in AbilityGrantKind::ALL {
            assert_eq!(AbilityGrantKind::from_str_opt(kind.as_str()), Some(kind));
        }
        assert_eq!(AbilityGrantKind::from_str_opt("trained"), None);
        assert_eq!(AbilityGrantKind::from_str_opt("GM"), None);
    }

    #[test]
    fn only_gm_grants_lack_credit() {
        let uncredited: Vec<_> = AbilityGrantKind::ALL
            .into_iter()
            .filter(|k| !k.is_credited())
            .collect();
        assert_eq!(uncredited, vec![AbilityGrantKind::Gm]);
    }
}
