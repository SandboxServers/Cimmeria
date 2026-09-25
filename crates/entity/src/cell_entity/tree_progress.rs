//! A player's ability-tree provenance, hydrated from `sgw_player` at world
//! entry through `InitPlayerState`.

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
}
