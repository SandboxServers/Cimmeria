//! The crafting stations around a player, as the cell reports them to the
//! base (in the `CellToBaseMsg::Plugin` envelope).
//!
//! The base owns `onUpdateCraftingOptions` (140), but only the cell knows
//! where the stations are relative to the player. So the cell reports the
//! station set when it changes, and the base joins it with the Field
//! Crafting Tools in the crafting bag.

/// The nearest station entity per verb, in `CraftingOptions` section order:
/// crafting, research, reverseEngineering, alloying (`alias.xml`, the same
/// order as `cimmeria_cell_catalog::crafting::CraftType::ALL`). `None` means
/// no station for that verb within interaction range.
pub type StationSet = [Option<u32>; 4];

/// Why the station set changed, for the base's `options_changed` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationChangeCause {
    /// The first report after the player's cell entity was created: a login
    /// or a world change.
    WorldChange,
    /// A station the player had in reach no longer exists.
    StationDespawned,
    /// Anything else: the player (or a station) moved.
    Moved,
}

impl StationChangeCause {
    /// The `cause` log value.
    pub fn as_str(self) -> &'static str {
        match self {
            StationChangeCause::WorldChange => "world_change",
            StationChangeCause::StationDespawned => "station_despawned",
            StationChangeCause::Moved => "moved",
        }
    }
}

/// The station set around one player changed.
///
/// Sent by the cell's 1 Hz station tick on a change only. A player's first
/// tick after its cell entity is created always reports, so the base never
/// keeps a station set from a previous world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftingStations {
    /// The player's cell entity id.
    pub entity_id: u32,
    /// `sgw_player.player_id`.
    pub player_id: i32,
    pub stations: StationSet,
    pub cause: StationChangeCause,
}
