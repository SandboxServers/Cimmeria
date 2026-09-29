//! `gmGiveExpertise` and `gmGiveAppliedSciencePoints` for one player, sent
//! in the `CellToBaseMsg::Plugin` envelope (#962 step 5; they were the
//! `GrantExpertise` and `GrantAppliedSciencePoints` variants).

/// Grant crafting expertise in one discipline and persist it
/// (`gmGiveExpertise`). One-way sink: the cell has already authorized the GM
/// and resolved `player_id`; the base loads the `CraftingState`, clamps the
/// new expertise to `[0, 100]`, adds the discipline to `discipline_ids` if
/// absent, saves, and pushes `onUpdateDiscipline` (method 136) to the
/// client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GmGrantExpertise {
    /// The target player's cell entity id.
    pub entity_id: u32,
    /// The target's `sgw_player.player_id`.
    pub player_id: i32,
    /// The target discipline (validated `> 0` cell-side).
    pub discipline_id: i32,
    /// The additive delta (validated `> 0` cell-side).
    pub amount: i32,
}

/// Grant applied-science points and persist them
/// (`gmGiveAppliedSciencePoints`). One-way sink: the base loads the
/// `CraftingState`, adds `amount` to `applied_science_points`, and saves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GmGrantAppliedSciencePoints {
    /// The target player's cell entity id.
    pub entity_id: u32,
    /// The target's `sgw_player.player_id`.
    pub player_id: i32,
    /// The points to add (validated `> 0` cell-side).
    pub amount: i32,
}
