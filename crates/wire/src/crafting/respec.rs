//! `.respeccraft` for one player (`CellToBaseMsg::RespecCraftOpen`).

/// A player's `.respeccraft`: open a crafting respec.
///
/// The client sends `respecCrafting` (100) only from the Yes button of the
/// `onCraftingRespecPrompt` (112) dialog and has no UI that opens one, so
/// the prompt starts on the server. The base answers with 112 and records a
/// pending respec; the next 100 from the same player within the window
/// carries it out. Any player may open one for themselves, so the message
/// names no caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RespecCraftOpen {
    /// The player's cell entity id.
    pub entity_id: u32,
    /// The player's `sgw_player.player_id`.
    pub player_id: i32,
}
