//! The content engine's `grant_ability` action (Class Start v6, CS-01a).
//!
//! The cell sends [`ContentGrantAbilities`] for a player; the base writes
//! `sgw_player.abilities` and `sgw_player_ability_grants` in one transaction
//! and answers with [`ContentAbilitiesGranted`], which the cell mirrors: the
//! new ids into the known set, the credited ids into
//! `TreeProgress::credited_grants`, then `onKnownAbilitiesUpdate` and one
//! feedback line per newly learned ability.

use cimmeria_entity::cell_entity::AbilityGrantKind;

/// `CellToBaseMsg::ContentGrantAbilities`: grant `ability_ids` to the
/// player for free, with provenance `source_kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentGrantAbilities {
    pub entity_id: u32,
    /// The character the cell resolved; the base refuses the write when the
    /// entity's session now plays another.
    pub player_id: i32,
    /// For the telemetry (`None` when unknown).
    pub account_id: Option<u32>,
    /// The chain that fired, for the telemetry.
    pub chain_id: i64,
    /// Distinct ids, in authored order.
    pub ability_ids: Vec<i32>,
    /// Never [`AbilityGrantKind::Gm`]: the loader refuses that.
    pub source_kind: AbilityGrantKind,
    /// The mission or chain id the grant comes from.
    pub source_id: Option<i32>,
    /// `EArchetype` ordinals allowed to receive it; empty = every
    /// archetype. The base re-checks it against `sgw_player.archetype`.
    pub archetypes: Vec<i32>,
}

/// `BaseToCellMsg::ContentAbilitiesGranted`: what the base wrote for one
/// [`ContentGrantAbilities`]. Sent only after a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentAbilitiesGranted {
    pub entity_id: u32,
    /// The character the base wrote. The cell ignores the message when
    /// `entity_id` now plays another character.
    pub player_id: i32,
    pub chain_id: i64,
    pub source_kind: AbilityGrantKind,
    /// Ids the character did not know before, in request order.
    pub learned: Vec<i32>,
    /// Every requested id that now earns branch credit: a non-`gm` row and
    /// not one of the archetype's character-creation starters. The cell
    /// adds them to `TreeProgress::credited_grants`.
    pub credited: Vec<i32>,
    /// Ids the player had bought from a trainer and the grant converted
    /// (OD-CS06): the cell drops them from `trained_abilities`.
    pub converted: Vec<i32>,
    /// `training_points` and `tree_points_spent` after the commit (a
    /// conversion refunds the bought cost).
    pub training_points: i32,
    pub tree_points_spent: i32,
}
