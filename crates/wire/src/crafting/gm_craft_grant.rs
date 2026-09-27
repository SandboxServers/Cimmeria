//! `.craftkit` and `.learnblueprint` for one player
//! (`CellToBaseMsg::GmCraftGrant`).

/// What a GM crafting grant gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmCraftGrantKind {
    /// `.craftkit <blueprint> [count]`: the items of the blueprint's
    /// component set 1, each `count` times its quantity.
    Kit { blueprint_id: i32, count: i32 },
    /// `.learnblueprint <id>`: teach one blueprint.
    LearnBlueprint { blueprint_id: i32 },
}

/// A GM's `.craftkit` or `.learnblueprint`, already GM-gated by the cell's
/// `.`-console. The base re-checks the caller's session `access_level`,
/// validates the blueprint against the crafting catalog, applies the grant
/// in one transaction and sends the caller the result line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GmCraftGrant {
    /// The target player's cell entity id.
    pub entity_id: u32,
    /// The target's `sgw_player.player_id`.
    pub player_id: i32,
    /// The GM who ran the command: the base checks this session's access
    /// level and sends it the result line.
    pub gm_entity_id: u32,
    pub grant: GmCraftGrantKind,
}
