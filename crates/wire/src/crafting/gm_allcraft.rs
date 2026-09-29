//! `.allcraft` for one player, sent in the `CellToBaseMsg::Plugin` envelope.

/// A GM's `.allcraft`, already GM-gated by the cell's `.`-console.
///
/// The base re-checks the caller's session `access_level`, then sets every
/// racial paradigm to 7, learns every discipline at expertise 100, grants
/// every blueprint, persists, pushes 136 / 138 / 139, and turns on "craft
/// anywhere" for the target's session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GmAllCraft {
    /// The target player's cell entity id.
    pub entity_id: u32,
    /// The target's `sgw_player.player_id`.
    pub player_id: i32,
    /// The GM who ran the command: the base checks this session's access
    /// level and sends it the result lines.
    pub gm_entity_id: u32,
}
