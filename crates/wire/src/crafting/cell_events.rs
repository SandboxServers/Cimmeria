//! Crafting events the cell reports to the base, beside the player's own
//! requests ([`super::CraftRequest`]).
//!
//! The base owns `onUpdateCraftingOptions` (140), but only the cell knows
//! where the stations are relative to the player. So the cell reports the
//! station set when it changes ([`CraftingStations`]), and the base joins it
//! with the Field Crafting Tools in the crafting bag.

/// The nearest station entity per verb, in `CraftingOptions` section order:
/// crafting, research, reverseEngineering, alloying (`alias.xml`, the same
/// order as `cimmeria_cell_catalog::crafting::CraftType::ALL`). `None` means
/// no station for that verb within interaction range.
pub type StationSet = [Option<u32>; 4];

/// The station set around one player changed (`CellToBaseMsg::CraftingStations`).
///
/// Sent by the cell's 1 Hz station tick on a change only: the player walked
/// into or out of range, a station despawned, or the player changed world.
/// A player's first tick after its cell entity is created always reports,
/// so the base never keeps a station set from a previous world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftingStations {
    /// The player's cell entity id.
    pub entity_id: u32,
    /// `sgw_player.player_id`.
    pub player_id: i32,
    pub stations: StationSet,
}

/// `.allcraft` (D-CR17), already GM-gated by the cell's `.`-console
/// (`CellToBaseMsg::GmAllCraft`).
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
